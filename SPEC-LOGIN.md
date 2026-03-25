# Spec: OAuth2 Login Flow for Authalla CLI

## Overview

Replace the current manual M2M client credentials setup (`authalla config set --client-id --client-secret`) with an `authalla login` command that authenticates the user via OAuth2 Authorization Code + PKCE in the browser — the same standard used by `gh`, `vercel`, `aws`, etc.

The admin users already authenticate via OAuth2 against a dedicated Authalla tenant. The CLI is simply another OAuth2 client on that same tenant.

## Current State

- CLI requires `authalla config set --api-url --client-id --client-secret`
- User must first create an Authalla account, then create an M2M client manually
- Auth uses `client_credentials` grant → token is tenant-scoped
- Config stored at `~/.config/authalla/config.json`

## Target State

- User runs `authalla login` → browser opens → user authenticates via standard OAuth2 → CLI receives tokens
- CLI can list the user's accounts/tenants and switch between them
- M2M credentials flow remains supported as a fallback (for CI/CD, scripts, etc.)

## Prerequisites

A **public native OAuth2 client** must be created on the admin tenant (the same tenant used for admin UI login). This client should have:
- **Type**: Public (no client secret)
- **Application type**: Native
- **Allowed redirect URIs**: `http://localhost` (with any port)
- **Grant types**: `authorization_code`, `refresh_token`
- **Scopes**: `openid`, `profile`, `email`

The client ID is hardcoded in the CLI or discoverable via the admin tenant's OIDC configuration.

## Login Flow

### 1. `authalla login`

Standard OAuth2 Authorization Code flow with PKCE:

```
1. CLI generates a random code_verifier (43-128 chars, RFC 7636)
2. CLI computes code_challenge = BASE64URL(SHA256(code_verifier))
3. CLI starts a local HTTP server on a random available port (e.g. 19836)
4. CLI discovers endpoints via OIDC:
   GET https://{admin-tenant}.authalla.com/.well-known/openid-configuration

5. CLI opens browser to the authorization endpoint:
   https://{admin-tenant}.authalla.com/oauth2/authorize?
     response_type=code&
     client_id={cli_client_id}&
     redirect_uri=http://localhost:{port}/callback&
     scope=openid+profile+email&
     code_challenge={code_challenge}&
     code_challenge_method=S256&
     state={random_state}

6. User authenticates in the browser (passkey, magic link, etc.)
7. Browser redirects to http://localhost:{port}/callback?code={code}&state={state}
8. CLI verifies state matches
9. CLI exchanges the code for tokens:
   POST https://{admin-tenant}.authalla.com/oauth2/token
   Content-Type: application/x-www-form-urlencoded

   grant_type=authorization_code&
   code={code}&
   redirect_uri=http://localhost:{port}/callback&
   client_id={cli_client_id}&
   code_verifier={code_verifier}

10. CLI receives: { access_token, refresh_token, id_token, expires_in, token_type }
11. CLI stores tokens in config
12. CLI calls GET /api/v1/me to get accounts/tenants
13. CLI prompts for account/tenant selection if multiple
14. CLI prints "Logged in as {email}. Active tenant: {tenant_name}"
```

### 2. Token Refresh

When the access token expires:
```
POST https://{admin-tenant}.authalla.com/oauth2/token
Content-Type: application/x-www-form-urlencoded

grant_type=refresh_token&
refresh_token={refresh_token}&
client_id={cli_client_id}
```

Standard OAuth2 refresh — no custom endpoints needed.

### 3. Account & Tenant Selection

After login, the CLI needs to know which account/tenant to operate on:

1. CLI calls `GET /api/v1/me` with `Authorization: Bearer {access_token}` (no X-Tenant-ID needed)
2. Response includes the user's accounts and tenants
3. If single account + single tenant → auto-select
4. If multiple → prompt the user to pick or accept `--account` / `--tenant` flags
5. Store the selected account ID and tenant ID in config

Switch later with:
- `authalla accounts list` — show available accounts
- `authalla accounts select` — switch active account
- `authalla tenant list` — show tenants in the active account
- `authalla tenant select {id}` — switch active tenant

### 4. API Requests with User Tokens

After login, the CLI sends requests to the public API:
- `Authorization: Bearer {access_token}` — standard OAuth2 bearer token
- `X-Tenant-ID: {tenant_public_id}` — required for all endpoints except `GET /api/v1/me`

The server detects that this is an admin user token (not an M2M client token) because:
- The token's `tid` claim matches the admin issuer tenant
- The token's `sub` claim differs from `client_id` (it's a user, not a machine)

Admin users get full access to all scopes for tenants they own/manage.

## Config Changes

### New config format (after `authalla login`)

```json
{
  "auth_method": "login",
  "issuer_url": "https://{admin-tenant}.authalla.com",
  "client_id": "{cli_client_id}",
  "access_token": "eyJ...",
  "refresh_token": "...",
  "id_token": "eyJ...",
  "expires_at": 1711296000,
  "user": {
    "email": "user@example.com",
    "name": "Jane Doe"
  },
  "account_id": "abc123",
  "tenant_id": "xyz789"
}
```

### Legacy M2M config (still supported)

```json
{
  "auth_method": "client_credentials",
  "api_url": "https://tenant-id.authalla.com",
  "client_id": "client_abc",
  "client_secret": "secret_xyz",
  "token": {
    "access_token": "eyJ...",
    "expires_at": 1711296000
  }
}
```

### Auth method detection

- If `auth_method` is `"login"` → use OAuth2 user token flow (refresh via standard `/oauth2/token`)
- If `auth_method` is `"client_credentials"` or missing → use M2M flow as today
- `authalla login` sets `auth_method` to `"login"`
- `authalla config set --client-id --client-secret` sets `auth_method` to `"client_credentials"`

## New Commands

### `authalla login`

```
$ authalla login
Opening browser to authenticate...
Waiting for callback on http://localhost:19836/callback

✓ Logged in as jane@example.com

You have access to 2 accounts:
  1. Acme Corp (acme-corp)
  2. Personal (personal-123)

Select account [1]: 1

Active account: Acme Corp
Active tenant: production (lnviblxvycjbe3ffq1ssg)
```

### `authalla logout`

Clears stored tokens from config.

### `authalla accounts list`

```
$ authalla accounts list
  NAME        ID              ROLE
* Acme Corp   acme-corp       owner
  Personal    personal-123    owner
```

### `authalla accounts select`

```
$ authalla accounts select personal-123
Active account: Personal
Active tenant: default (abc123def)
```

## API Endpoints

All endpoints are standard OAuth2/OIDC — no custom auth endpoints needed.

### OIDC Discovery (existing)
```
GET https://{admin-tenant}.authalla.com/.well-known/openid-configuration
```

### Authorization (existing)
```
GET https://{admin-tenant}.authalla.com/oauth2/authorize
```

### Token (existing)
```
POST https://{admin-tenant}.authalla.com/oauth2/token
```

### User Info (new, server-side)
```
GET https://{admin-tenant}.authalla.com/api/v1/me
Authorization: Bearer {access_token}
```

Response:
```json
{
  "user": {
    "email": "jane@example.com",
    "name": "Jane Doe"
  },
  "accounts": [
    {
      "id": "acme-corp",
      "name": "Acme Corp",
      "role": "owner",
      "tenants": [
        { "id": "lnviblxvycjbe3ffq1ssg", "name": "production" },
        { "id": "abc123def", "name": "staging" }
      ]
    }
  ]
}
```

## Dependencies

New Rust crates needed:
- `tiny_http` or `std::net::TcpListener` — for the local callback server
- `sha2` — for PKCE S256 code challenge
- `open` — to open the browser

## Implementation Notes

- OIDC discovery (`/.well-known/openid-configuration`) gives you the `authorization_endpoint` and `token_endpoint` — use those instead of hardcoding paths
- The local callback server should listen on `127.0.0.1` (not `0.0.0.0`)
- After receiving the callback, show a simple HTML page saying "You can close this tab"
- Poll interval is not needed — the local HTTP server receives the callback directly
- `X-Tenant-ID` header must be sent with all API requests except `GET /api/v1/me`

## Migration

- Existing configs without `auth_method` field default to `"client_credentials"` (backwards compatible)
- `authalla config set` continues to work for M2M setup
- `authalla login` is the new recommended onboarding path
