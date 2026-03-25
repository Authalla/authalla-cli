use anyhow::{Context, Result};
use reqwest::blocking::{Client, RequestBuilder, Response};

use crate::auth;
use crate::config::{self, AuthMethod};

pub struct ApiClient {
    client: Client,
    base_url: String,
    token: String,
    /// Tenant ID header, set for user token auth (not M2M).
    tenant_id: Option<String>,
}

impl ApiClient {
    pub fn new() -> Result<Self> {
        let mut cfg = config::load()?;
        let token = auth::get_token(&mut cfg)?;
        let base_url = cfg.base_url()?;

        let tenant_id = if cfg.auth_method == AuthMethod::Login {
            Some(
                cfg.tenant_id
                    .clone()
                    .context("No tenant selected. Run `authalla accounts select` first.")?,
            )
        } else {
            None
        };

        Ok(Self {
            client: Client::new(),
            base_url,
            token,
            tenant_id,
        })
    }

    /// Create an ApiClient for requests that don't require a tenant (e.g. /api/v1/me).
    pub fn new_without_tenant() -> Result<Self> {
        let mut cfg = config::load()?;
        let token = auth::get_token(&mut cfg)?;
        let base_url = cfg.base_url()?;

        Ok(Self {
            client: Client::new(),
            base_url,
            token,
            tenant_id: None,
        })
    }

    fn apply_headers(&self, req: RequestBuilder) -> RequestBuilder {
        let req = req.bearer_auth(&self.token);
        if let Some(ref tenant_id) = self.tenant_id {
            req.header("X-Tenant-ID", tenant_id)
        } else {
            req
        }
    }

    pub fn get(&self, path: &str) -> Result<serde_json::Value> {
        let url = format!("{}{}", self.base_url, path);
        let resp = self
            .apply_headers(self.client.get(&url))
            .send()
            .with_context(|| format!("Request failed: GET {}", path))?;
        self.handle_response(resp)
    }

    pub fn get_with_query(&self, path: &str, query: &[(&str, &str)]) -> Result<serde_json::Value> {
        let url = format!("{}{}", self.base_url, path);
        let resp = self
            .apply_headers(self.client.get(&url))
            .query(query)
            .send()
            .with_context(|| format!("Request failed: GET {}", path))?;
        self.handle_response(resp)
    }

    pub fn post(&self, path: &str, body: &serde_json::Value) -> Result<serde_json::Value> {
        let url = format!("{}{}", self.base_url, path);
        let resp = self
            .apply_headers(self.client.post(&url))
            .json(body)
            .send()
            .with_context(|| format!("Request failed: POST {}", path))?;
        self.handle_response(resp)
    }

    pub fn put(&self, path: &str, body: &serde_json::Value) -> Result<serde_json::Value> {
        let url = format!("{}{}", self.base_url, path);
        let resp = self
            .apply_headers(self.client.put(&url))
            .json(body)
            .send()
            .with_context(|| format!("Request failed: PUT {}", path))?;
        self.handle_response(resp)
    }

    pub fn delete(&self, path: &str) -> Result<()> {
        let url = format!("{}{}", self.base_url, path);
        let resp = self
            .apply_headers(self.client.delete(&url))
            .send()
            .with_context(|| format!("Request failed: DELETE {}", path))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().unwrap_or_default();
            anyhow::bail!("API error ({}): {}", status, body);
        }
        Ok(())
    }

    fn handle_response(&self, resp: Response) -> Result<serde_json::Value> {
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().unwrap_or_default();
            anyhow::bail!("API error ({}): {}", status, body);
        }
        let body: serde_json::Value = resp.json().context("Invalid JSON response")?;
        Ok(body)
    }
}
