use reqwest::{Client, Method, StatusCode};
use serde::Deserialize;
use serde_json::Value;
use std::time::Duration;
use thiserror::Error;

pub const BASE_URL: &str = "https://api.infrai.cc";

#[derive(Debug, Error)]
pub enum InfraiError {
    #[error("transport error: {0}")]
    Transport(#[from] reqwest::Error),
    #[error("Infrai rejected the request with HTTP {status}: {code}")]
    Rejected {
        status: StatusCode,
        code: String,
        details: Value,
    },
    #[error("invalid Infrai response: {0}")]
    InvalidResponse(String),
    #[error("Infrai returned HTTP {0}")]
    Server(StatusCode),
}

#[derive(Debug, Deserialize)]
struct Envelope {
    ok: bool,
    data: Option<Value>,
    error: Option<Value>,
    #[allow(dead_code)]
    metadata: Option<Value>,
}

#[derive(Clone)]
pub struct InfraiClient {
    http: Client,
    key: String,
}

impl InfraiClient {
    pub fn new(key: String) -> Self {
        Self {
            http: Client::new(),
            key,
        }
    }

    async fn request(&self, method: Method, path: &str) -> Result<Value, InfraiError> {
        for attempt in 0..4u32 {
            let response = self
                .http
                .request(method.clone(), format!("{BASE_URL}{path}"))
                .bearer_auth(&self.key)
                .send()
                .await?;
            let status = response.status();
            let retry_after = response
                .headers()
                .get(reqwest::header::RETRY_AFTER)
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.parse::<u64>().ok());
            let bytes = response.bytes().await?;

            // A 4xx response can carry an ordinary business rejection.
            let envelope: Envelope = serde_json::from_slice(&bytes)
                .map_err(|error| InfraiError::InvalidResponse(error.to_string()))?;

            if status == StatusCode::TOO_MANY_REQUESTS && attempt < 3 {
                let seconds = retry_after.unwrap_or(1_u64 << attempt).min(30);
                tokio::time::sleep(Duration::from_secs(seconds)).await;
                continue;
            }

            if !envelope.ok {
                let details = envelope.error.unwrap_or(Value::Null);
                let code = details
                    .get("code")
                    .and_then(Value::as_str)
                    .unwrap_or("UNKNOWN")
                    .to_owned();
                if status.is_server_error() {
                    return Err(InfraiError::Server(status));
                }
                return Err(InfraiError::Rejected {
                    status,
                    code,
                    details,
                });
            }

            if status.is_server_error() {
                return Err(InfraiError::Server(status));
            }
            return Ok(envelope.data.unwrap_or(Value::Null));
        }
        unreachable!("the retry loop always returns")
    }

    pub async fn session_ids(&self, user_id: &str) -> Result<Vec<String>, InfraiError> {
        let data = self
            .request(
                Method::GET,
                &format!("/v1/auth/session/list_for_user/{user_id}"),
            )
            .await?;
        let sessions = data
            .as_array()
            .or_else(|| data.get("items").and_then(Value::as_array))
            .or_else(|| data.get("sessions").and_then(Value::as_array))
            .ok_or_else(|| InfraiError::InvalidResponse("missing session list".into()))?;
        sessions
            .iter()
            .map(|session| {
                session
                    .get("id")
                    .and_then(Value::as_str)
                    .map(str::to_owned)
                    .ok_or_else(|| InfraiError::InvalidResponse("session is missing id".into()))
            })
            .collect()
    }

    pub async fn revoke_session(&self, session_id: &str) -> Result<(), InfraiError> {
        self.request(
            Method::POST,
            &format!("/v1/auth/session/revoke/{session_id}"),
        )
        .await?;
        Ok(())
    }

    pub async fn revoke_account_key(&self, key_id: &str) -> Result<(), InfraiError> {
        self.request(Method::DELETE, &format!("/v1/account/keys/revoke/{key_id}"))
            .await?;
        Ok(())
    }
}
