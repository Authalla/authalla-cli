use anyhow::{Context, Result};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use chrono::Utc;
use rand::Rng;
use sha2::{Digest, Sha256};
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};

use crate::config::{self, Config, UserInfo};

const DEFAULT_ISSUER_URL: &str = "https://login.authalla.com";
const DEFAULT_CLIENT_ID: &str = "mefyzxhuy1qpltczeglpa";

/// Discover OIDC endpoints from the issuer's well-known configuration.
fn discover_oidc(issuer_url: &str) -> Result<(String, String)> {
    let url = format!(
        "{}/.well-known/openid-configuration",
        issuer_url.trim_end_matches('/')
    );

    let client = reqwest::blocking::Client::new();
    let resp = client
        .get(&url)
        .send()
        .with_context(|| format!("Failed to fetch OIDC configuration from {}", url))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().unwrap_or_default();
        anyhow::bail!("OIDC discovery failed ({}): {}", status, body);
    }

    let config: serde_json::Value = resp.json().context("Invalid OIDC configuration response")?;

    let authorization_endpoint = config["authorization_endpoint"]
        .as_str()
        .context("Missing authorization_endpoint in OIDC configuration")?
        .to_string();

    let token_endpoint = config["token_endpoint"]
        .as_str()
        .context("Missing token_endpoint in OIDC configuration")?
        .to_string();

    Ok((authorization_endpoint, token_endpoint))
}

/// Parsed callback parameters from the OAuth2 redirect.
struct CallbackParams {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
    error_description: Option<String>,
}

/// Send an HTML response on a TCP stream and close it.
fn send_html_response(stream: &mut TcpStream, body: &str) {
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body,
    );
    let _ = stream.write_all(response.as_bytes());
    let _ = stream.flush();
}

pub fn run(issuer_url: Option<String>, client_id: Option<String>) -> Result<()> {
    let issuer_url = issuer_url
        .unwrap_or_else(|| DEFAULT_ISSUER_URL.to_string())
        .trim_end_matches('/')
        .to_string();

    // Discover OIDC endpoints
    eprintln!("Discovering OIDC endpoints...");
    let (authorization_endpoint, token_endpoint) = discover_oidc(&issuer_url)?;

    let client_id = client_id.unwrap_or_else(|| DEFAULT_CLIENT_ID.to_string());

    // Generate PKCE code verifier (43-128 random URL-safe chars)
    let code_verifier = generate_code_verifier();
    let code_challenge = generate_code_challenge(&code_verifier);

    // Generate random state for CSRF protection
    let state = generate_random_string(32);

    // Start local callback server on a random port
    let listener = TcpListener::bind("127.0.0.1:0").context("Failed to bind local server")?;
    let port = listener.local_addr()?.port();
    let redirect_uri = format!("http://localhost:{}/callback", port);

    // Build authorization URL
    let auth_url = format!(
        "{}?response_type=code&client_id={}&redirect_uri={}&scope={}&code_challenge={}&code_challenge_method=S256&state={}",
        authorization_endpoint,
        urlencoding(&client_id),
        urlencoding(&redirect_uri),
        urlencoding("openid profile email offline_access"),
        code_challenge,
        state,
    );

    eprintln!("Opening browser to authenticate...");
    eprintln!(
        "Waiting for callback on http://localhost:{}/callback",
        port
    );

    // Open the browser
    if open::that(&auth_url).is_err() {
        eprintln!("\nCould not open browser automatically. Please open this URL:");
        eprintln!("{}", auth_url);
    }

    // Wait for the callback (captures params, keeps stream open for response)
    let (params, mut stream) = wait_for_callback(&listener)?;

    // Handle OAuth2 error response
    if let Some(err) = params.error {
        let desc = params.error_description.unwrap_or_default();
        send_html_response(&mut stream, &error_page(&err, &desc));
        anyhow::bail!("Authorization failed: {} {}", err, desc);
    }

    let code = params.code.context("Missing 'code' parameter in callback")?;
    let returned_state = params
        .state
        .context("Missing 'state' parameter in callback")?;

    // Verify state
    if returned_state != state {
        send_html_response(&mut stream, &error_page("state_mismatch", "The state parameter did not match. This may indicate a CSRF attack."));
        anyhow::bail!("State mismatch — possible CSRF attack. Login aborted.");
    }

    // Exchange code for tokens
    let http = reqwest::blocking::Client::new();

    let resp = http
        .post(&token_endpoint)
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", code.as_str()),
            ("redirect_uri", redirect_uri.as_str()),
            ("client_id", client_id.as_str()),
            ("code_verifier", code_verifier.as_str()),
        ])
        .send()
        .context("Failed to exchange authorization code")?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().unwrap_or_default();
        send_html_response(
            &mut stream,
            &error_page("token_exchange_failed", &format!("Server returned {}", status)),
        );
        anyhow::bail!("Token exchange failed ({}): {}", status, body);
    }

    let token_body: serde_json::Value = resp.json().context("Invalid token response")?;

    let access_token = token_body["access_token"]
        .as_str()
        .context("Missing access_token")?
        .to_string();
    let refresh_token = token_body["refresh_token"]
        .as_str()
        .context("Missing refresh_token")?
        .to_string();
    let id_token = token_body["id_token"].as_str().map(|s| s.to_string());
    let expires_in = token_body["expires_in"].as_i64().unwrap_or(900);
    let expires_at = Utc::now().timestamp() + expires_in;

    // Fetch user info and accounts via /api/v1/me
    let me_resp = http
        .get(&format!("{}/api/v1/me", issuer_url))
        .bearer_auth(&access_token)
        .send()
        .context("Failed to fetch user info")?;

    if !me_resp.status().is_success() {
        let status = me_resp.status();
        let body = me_resp.text().unwrap_or_default();
        send_html_response(
            &mut stream,
            &error_page("user_info_failed", &format!("Failed to fetch user info ({})", status)),
        );
        anyhow::bail!("Failed to fetch user info ({}): {}", status, body);
    }

    let me: serde_json::Value = me_resp.json().context("Invalid /me response")?;

    // User info is nested under "user" key
    let email = me["user"]["email"]
        .as_str()
        .unwrap_or("unknown")
        .to_string();
    let name = me["user"]["name"].as_str().unwrap_or("").to_string();

    let accounts = me["accounts"]
        .as_array()
        .context("Expected accounts array in /me response")?;

    if accounts.is_empty() {
        send_html_response(
            &mut stream,
            &error_page("no_accounts", "No accounts found for this user."),
        );
        anyhow::bail!("No accounts found for this user.");
    }

    // Now send the success page with user details
    send_html_response(&mut stream, &success_page(&email, &name));

    // Build config
    let mut cfg = Config::new_login(
        issuer_url,
        client_id,
        access_token,
        refresh_token,
        id_token,
        expires_at,
        UserInfo {
            email: email.clone(),
            name,
        },
    );

    eprintln!("\n\u{2713} Logged in as {}", email);

    // Account selection
    if accounts.len() == 1 {
        let account = &accounts[0];
        let account_id = account["id"].as_str().unwrap_or_default().to_string();
        let account_name = account["name"].as_str().unwrap_or_default();

        cfg.account_id = Some(account_id);

        if let Some(tenants) = account["tenants"].as_array() {
            if let Some(tenant) = tenants.first() {
                let tenant_id = tenant["id"].as_str().unwrap_or_default().to_string();
                let tenant_name = tenant["name"].as_str().unwrap_or("default");
                cfg.tenant_id = Some(tenant_id.clone());
                eprintln!("Active account: {}", account_name);
                eprintln!("Active tenant: {} ({})", tenant_name, tenant_id);
            }
        }
    } else {
        eprintln!("\nYou have access to {} accounts:", accounts.len());
        for (i, account) in accounts.iter().enumerate() {
            let name = account["name"].as_str().unwrap_or("unnamed");
            let id = account["id"].as_str().unwrap_or("");
            eprintln!("  {}. {} ({})", i + 1, name, id);
        }

        let selection = prompt_selection(accounts.len())?;
        let account = &accounts[selection];
        let account_id = account["id"].as_str().unwrap_or_default().to_string();
        let account_name = account["name"].as_str().unwrap_or_default();

        cfg.account_id = Some(account_id);

        if let Some(tenants) = account["tenants"].as_array() {
            if let Some(tenant) = tenants.first() {
                let tenant_id = tenant["id"].as_str().unwrap_or_default().to_string();
                let tenant_name = tenant["name"].as_str().unwrap_or("default");
                cfg.tenant_id = Some(tenant_id.clone());
                eprintln!("\nActive account: {}", account_name);
                eprintln!("Active tenant: {} ({})", tenant_name, tenant_id);
            }
        }
    }

    config::save(&cfg)?;
    Ok(())
}

fn generate_code_verifier() -> String {
    generate_random_string(64)
}

fn generate_code_challenge(verifier: &str) -> String {
    let hash = Sha256::digest(verifier.as_bytes());
    URL_SAFE_NO_PAD.encode(hash)
}

fn generate_random_string(len: usize) -> String {
    const CHARSET: &[u8] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-._~";
    let mut rng = rand::thread_rng();
    (0..len)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

fn urldecoding(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.bytes();
    while let Some(b) = chars.next() {
        if b == b'%' {
            let hi = chars.next().unwrap_or(0);
            let lo = chars.next().unwrap_or(0);
            let hex = [hi, lo];
            if let Ok(s) = std::str::from_utf8(&hex) {
                if let Ok(val) = u8::from_str_radix(s, 16) {
                    result.push(val as char);
                    continue;
                }
            }
            result.push('%');
            result.push(hi as char);
            result.push(lo as char);
        } else if b == b'+' {
            result.push(' ');
        } else {
            result.push(b as char);
        }
    }
    result
}

fn urlencoding(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                String::from(b as char)
            }
            _ => format!("%{:02X}", b),
        })
        .collect()
}

/// Wait for the OAuth2 callback. Returns parsed params and the open TCP stream
/// so we can send the response later (after token exchange + /me call).
fn wait_for_callback(listener: &TcpListener) -> Result<(CallbackParams, TcpStream)> {
    let (stream, _) = listener.accept().context("Failed to accept connection")?;
    let mut reader = BufReader::new(&stream);

    let mut request_line = String::new();
    reader
        .read_line(&mut request_line)
        .context("Failed to read request")?;

    // Consume remaining headers (required before we can write the response)
    let mut header = String::new();
    loop {
        header.clear();
        reader.read_line(&mut header).context("Failed to read headers")?;
        if header.trim().is_empty() {
            break;
        }
    }

    // Parse "GET /callback?code=xxx&state=yyy HTTP/1.1"
    let path = request_line
        .split_whitespace()
        .nth(1)
        .context("Invalid HTTP request")?
        .to_string();

    let query = path.split('?').nth(1).unwrap_or("");

    let mut code = None;
    let mut state = None;
    let mut error = None;
    let mut error_description = None;

    for param in query.split('&') {
        let mut parts = param.splitn(2, '=');
        let key = parts.next().unwrap_or("");
        let value = parts.next().unwrap_or("");
        match key {
            "code" => code = Some(value.to_string()),
            "state" => state = Some(value.to_string()),
            "error" => error = Some(urldecoding(value)),
            "error_description" => error_description = Some(urldecoding(value)),
            _ => {}
        }
    }

    // Get the underlying TcpStream back from the BufReader
    let stream = reader.into_inner().try_clone().context("Failed to clone stream")?;

    Ok((
        CallbackParams {
            code,
            state,
            error,
            error_description,
        },
        stream,
    ))
}

fn prompt_selection(max: usize) -> Result<usize> {
    use std::io::{self, Write};

    loop {
        eprint!("\nSelect account [1]: ");
        io::stderr().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();

        if input.is_empty() {
            return Ok(0);
        }

        if let Ok(n) = input.parse::<usize>() {
            if n >= 1 && n <= max {
                return Ok(n - 1);
            }
        }

        eprintln!("Please enter a number between 1 and {}", max);
    }
}

// ---------------------------------------------------------------------------
// HTML templates styled to match the Authalla login screen
// ---------------------------------------------------------------------------

const PAGE_STYLE: &str = r#"
    <style>
      @import url('https://fonts.googleapis.com/css2?family=DM+Sans:wght@400;500;600;700&display=swap');

      * { margin: 0; padding: 0; box-sizing: border-box; }

      body {
        font-family: 'DM Sans', ui-sans-serif, system-ui, sans-serif;
        -webkit-font-smoothing: antialiased;
        -moz-osx-font-smoothing: grayscale;
        min-height: 100vh;
        display: flex;
        flex-direction: column;
        justify-content: center;
        align-items: center;
        padding: 1rem;
        background-color: #f3f4f6;
      }

      @media (prefers-color-scheme: dark) {
        body { background-color: #0a0a0b; }
        .card { background-color: #18181b; }
        .title { color: #fafafa; }
        .subtitle { color: #a1a1aa; }
        .detail-label { color: #a1a1aa; }
        .detail-value { color: #fafafa; }
        .divider { border-color: rgba(255,255,255,0.1); }
        .footer-text { color: #71717a; }
      }

      .card {
        width: 100%;
        max-width: 28rem;
        background-color: #ffffff;
        border-radius: 0.5rem;
        padding: 3rem 1.5rem;
        box-shadow: 0 10px 15px -3px rgba(0,0,0,0.1), 0 4px 6px -4px rgba(0,0,0,0.1);
        text-align: center;
      }

      @media (min-width: 640px) {
        .card { padding: 3rem; }
      }

      .icon-circle {
        width: 3.5rem;
        height: 3.5rem;
        border-radius: 50%;
        display: flex;
        align-items: center;
        justify-content: center;
        margin: 0 auto 1.5rem;
      }

      .icon-circle.success {
        background-color: rgba(147, 51, 234, 0.1);
      }

      .icon-circle.error {
        background-color: rgba(239, 68, 68, 0.1);
      }

      .icon-circle svg {
        width: 1.5rem;
        height: 1.5rem;
      }

      .title {
        font-size: 1.5rem;
        font-weight: 700;
        letter-spacing: -0.025em;
        color: #111827;
        margin-bottom: 0.5rem;
      }

      .subtitle {
        font-size: 0.875rem;
        color: #4b5563;
        margin-bottom: 2rem;
      }

      .details {
        text-align: left;
        margin-bottom: 1.5rem;
      }

      .detail-row {
        display: flex;
        justify-content: space-between;
        align-items: center;
        padding: 0.75rem 0;
      }

      .detail-row + .detail-row {
        border-top: 1px solid rgba(0,0,0,0.06);
      }

      @media (prefers-color-scheme: dark) {
        .detail-row + .detail-row {
          border-color: rgba(255,255,255,0.06);
        }
      }

      .detail-label {
        font-size: 0.875rem;
        font-weight: 500;
        color: #6b7280;
      }

      .detail-value {
        font-size: 0.875rem;
        font-weight: 600;
        color: #111827;
      }

      .divider {
        border: none;
        border-top: 1px solid rgba(0,0,0,0.06);
        margin: 1.5rem 0;
      }

      .footer-text {
        font-size: 0.75rem;
        color: #4b5563;
      }

      .error-code {
        display: inline-block;
        font-size: 0.75rem;
        font-weight: 600;
        color: #dc2626;
        background-color: #fef2f2;
        border: 1px solid #fecaca;
        padding: 0.25rem 0.75rem;
        border-radius: 9999px;
        margin-bottom: 1rem;
      }

      @media (prefers-color-scheme: dark) {
        .error-code {
          color: #fca5a5;
          background-color: rgba(239, 68, 68, 0.2);
          border-color: rgba(239, 68, 68, 0.3);
        }
      }

      .error-message {
        font-size: 0.875rem;
        color: #111827;
        line-height: 1.5;
        margin-bottom: 1.5rem;
      }

      @media (prefers-color-scheme: dark) {
        .error-message { color: #e4e4e7; }
      }
    </style>
"#;

fn success_page(email: &str, name: &str) -> String {
    let display_name = if name.is_empty() {
        email.to_string()
    } else {
        name.to_string()
    };

    // Get initial for avatar
    let initial = display_name
        .chars()
        .next()
        .unwrap_or('?')
        .to_uppercase()
        .to_string();

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>Logged in - Authalla CLI</title>
  {style}
  <style>
    .avatar {{
      width: 3.5rem;
      height: 3.5rem;
      border-radius: 50%;
      background-color: #9333ea;
      color: #ffffff;
      display: flex;
      align-items: center;
      justify-content: center;
      margin: 0 auto 1.5rem;
      font-size: 1.25rem;
      font-weight: 700;
    }}
  </style>
</head>
<body>
  <div class="card">
    <div class="avatar">{initial}</div>
    <h1 class="title">You're logged in</h1>
    <p class="subtitle">You can close this tab and return to the terminal.</p>
    <div class="details">
      <div class="detail-row">
        <span class="detail-label">Name</span>
        <span class="detail-value">{name}</span>
      </div>
      <div class="detail-row">
        <span class="detail-label">Email</span>
        <span class="detail-value">{email}</span>
      </div>
    </div>
    <hr class="divider">
    <p class="footer-text">Authenticated via Authalla CLI</p>
  </div>
</body>
</html>"#,
        style = PAGE_STYLE,
        initial = html_escape(&initial),
        name = html_escape(&display_name),
        email = html_escape(email),
    )
}

fn error_page(error: &str, description: &str) -> String {
    format!(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>Login failed - Authalla CLI</title>
  {style}
</head>
<body>
  <div class="card">
    <div class="icon-circle error">
      <svg fill="none" viewBox="0 0 24 24" stroke-width="2" stroke="#ef4444">
        <path stroke-linecap="round" stroke-linejoin="round" d="M6 18L18 6M6 6l12 12" />
      </svg>
    </div>
    <h1 class="title">Login failed</h1>
    <p class="subtitle">Something went wrong during authentication.</p>
    <span class="error-code">{error}</span>
    <p class="error-message">{description}</p>
    <hr class="divider">
    <p class="footer-text">Check the terminal for details and try again.</p>
  </div>
</body>
</html>"##,
        style = PAGE_STYLE,
        error = html_escape(error),
        description = html_escape(description),
    )
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
