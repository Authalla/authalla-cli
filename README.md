# Authalla CLI

A command-line interface for the [Authalla](https://authalla.com) OAuth2 & OpenID Connect platform. Manage tenants, users, OAuth2 clients, branding, custom domains, and more — all from your terminal.

This CLI is designed to be **agent-optimized** — every command outputs structured JSON with built-in `schema` subcommands that describe expected inputs. While it works great on its own, the recommended way to use it is through the [Authalla agent skill](https://github.com/Authalla/agent-skills), which lets AI coding agents like Claude Code manage your Authalla resources conversationally.

## Installation

### Homebrew (macOS & Linux)

```sh
brew install authalla/tap/authalla
```

### From source

Requires [Rust](https://rustup.rs/) 1.70+.

```sh
cargo install --git https://github.com/authalla/authalla-cli
```

### Prebuilt binaries

Download the latest release for your platform from [GitHub Releases](https://github.com/authalla/authalla-cli/releases).

## Getting started

### 1. Configure credentials

Before using the CLI, configure it with your Authalla API credentials:

```sh
authalla config set \
  --api-url https://your-tenant.authalla.com \
  --client-id your_client_id \
  --client-secret your_client_secret
```

Credentials are stored in `~/.config/authalla/config.json` with `0600` file permissions.

### 2. Verify the configuration

```sh
authalla config show
```

### 3. Start managing resources

```sh
authalla tenant list
authalla user list
authalla client list
```

## Commands

| Command | Description |
|---------|-------------|
| `config` | Configure API credentials |
| `tenant` | Manage tenants |
| `user` | Manage users |
| `client` | Manage OAuth2 clients |
| `theme` | Manage login page branding |
| `custom-domain` | Manage custom domains |
| `custom-email` | Manage custom email sender domains |
| `social-login` | Manage social login providers |
| `well-known` | Fetch OpenID Connect discovery and JWKS endpoints |

### Common operations

Most resource commands follow a consistent pattern:

```sh
authalla <resource> list [--limit N] [--offset N]
authalla <resource> get --id <id>
authalla <resource> create --json '<json>'
authalla <resource> update --id <id> --json '<json>'
authalla <resource> delete --id <id>
authalla <resource> schema <create|update>
```

The `schema` subcommand prints the JSON schema for create or update payloads, so you can see exactly which fields are required and what values are accepted.

### Tenants

```sh
# List all tenants
authalla tenant list

# Create a tenant
authalla tenant create --json '{"name": "Production", "allow_registration": true}'

# Update a tenant with specific auth methods
authalla tenant update --id tenant_abc123 \
  --json '{"name": "Production", "allow_registration": true, "auth_methods": ["magic_link", "passkeys", "social_logins"]}'

# Delete a tenant
authalla tenant delete --id tenant_abc123
```

### Users

```sh
# List users with search
authalla user list --search "jane@example.com"

# Create a user
authalla user create --json '{"email": "jane@example.com", "name": "Jane Doe"}'

# Suspend a user
authalla user update --id user_abc123 --json '{"status": "suspended"}'
```

### OAuth2 Clients

```sh
# Create a web application client
authalla client create --json '{
  "name": "My Web App",
  "tenant_id": "tenant_abc123",
  "application_type": "web",
  "redirect_uris": ["https://app.example.com/callback"]
}'

# Create a machine-to-machine backend client
authalla client create --json '{
  "name": "Backend Service",
  "tenant_id": "tenant_abc123",
  "application_type": "backend"
}'

# View the create schema for full field reference
authalla client schema create
```

Application types:
- **`spa`** — Public client for single-page applications
- **`native`** — Public client for mobile/desktop apps
- **`web`** — Confidential client for server-rendered web apps
- **`backend`** — Confidential client for machine-to-machine (client credentials)

> **Note:** The client secret is only returned once on creation for confidential clients (`web` and `backend`).

### Theme

Customize the look of your login pages:

```sh
# View current theme
authalla theme get

# Update brand colors (hex format)
authalla theme update --json '{
  "primary_color": "#9333ea",
  "background_color": "#ffffff",
  "dark": {
    "primary_color": "#a855f7",
    "background_color": "#0f172a"
  }
}'
```

### Custom Domains

Serve your login pages from your own domain:

```sh
# Add a custom domain
authalla custom-domain create --json '{
  "tenant_id": "tenant_abc123",
  "domain": "auth.example.com"
}'

# After configuring the DNS records returned by create, verify them
authalla custom-domain verify --id domain_abc123
```

### Custom Email Domains

Send authentication emails from your own domain:

```sh
# Add a custom email domain
authalla custom-email create --json '{
  "tenant_id": "tenant_abc123",
  "email_domain": "mail.example.com"
}'

# After configuring DNS records, verify them
authalla custom-email verify --id email_abc123
```

### Social Login Providers

```sh
# List configured providers
authalla social-login list

# Add Google as a social login provider
authalla social-login create --json '{
  "name": "Google Login",
  "provider_type": "google",
  "client_id": "xxx.apps.googleusercontent.com",
  "client_secret": "GOCSPX-xxx",
  "tenant_ids": ["tenant_abc123"]
}'
```

Supported providers: `google`, `github`, `apple`, `microsoft`

### Well-Known Endpoints

Fetch public OpenID Connect discovery metadata:

```sh
# Fetch the OpenID Connect discovery document
authalla well-known openid-configuration

# Fetch the JSON Web Key Set (JWKS)
authalla well-known jwks
```

These commands do not require authentication — they only need the `api_url` from your configuration.

## Output

All commands output JSON to stdout, making it easy to pipe into tools like [`jq`](https://jqlang.github.io/jq/):

```sh
# Get the issuer from the OpenID configuration
authalla well-known openid-configuration | jq '.issuer'

# List all user emails
authalla user list | jq '.users[].email'
```

## License

[MIT](LICENSE)
