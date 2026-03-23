use anyhow::{Context, Result};
use reqwest::blocking::{Client, Response};

use crate::auth;
use crate::config;

pub struct ApiClient {
    client: Client,
    base_url: String,
    token: String,
}

impl ApiClient {
    pub fn new() -> Result<Self> {
        let mut cfg = config::load()?;
        let token = auth::get_token(&mut cfg)?;
        Ok(Self {
            client: Client::new(),
            base_url: cfg.api_url.trim_end_matches('/').to_string(),
            token,
        })
    }

    pub fn get(&self, path: &str) -> Result<serde_json::Value> {
        let url = format!("{}{}", self.base_url, path);
        let resp = self
            .client
            .get(&url)
            .bearer_auth(&self.token)
            .send()
            .with_context(|| format!("Request failed: GET {}", path))?;
        self.handle_response(resp)
    }

    pub fn get_with_query(&self, path: &str, query: &[(&str, &str)]) -> Result<serde_json::Value> {
        let url = format!("{}{}", self.base_url, path);
        let resp = self
            .client
            .get(&url)
            .bearer_auth(&self.token)
            .query(query)
            .send()
            .with_context(|| format!("Request failed: GET {}", path))?;
        self.handle_response(resp)
    }

    pub fn post(&self, path: &str, body: &serde_json::Value) -> Result<serde_json::Value> {
        let url = format!("{}{}", self.base_url, path);
        let resp = self
            .client
            .post(&url)
            .bearer_auth(&self.token)
            .json(body)
            .send()
            .with_context(|| format!("Request failed: POST {}", path))?;
        self.handle_response(resp)
    }

    pub fn put(&self, path: &str, body: &serde_json::Value) -> Result<serde_json::Value> {
        let url = format!("{}{}", self.base_url, path);
        let resp = self
            .client
            .put(&url)
            .bearer_auth(&self.token)
            .json(body)
            .send()
            .with_context(|| format!("Request failed: PUT {}", path))?;
        self.handle_response(resp)
    }

    pub fn delete(&self, path: &str) -> Result<()> {
        let url = format!("{}{}", self.base_url, path);
        let resp = self
            .client
            .delete(&url)
            .bearer_auth(&self.token)
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
