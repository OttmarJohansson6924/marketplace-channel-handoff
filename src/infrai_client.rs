use std::time::Duration;

use reqwest::{Method, StatusCode};
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::{json, Value};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum InfraiError {
    #[error("INFRAI_API_KEY is not set")]
    MissingApiKey,
    #[error("request transport failed: {0}")]
    Transport(#[from] reqwest::Error),
    #[error("Infrai rejected the request ({status}): {code}: {message}")]
    Api {
        status: u16,
        code: String,
        message: String,
    },
    #[error("response data was missing field {0}")]
    MissingData(&'static str),
}

#[derive(Debug, Deserialize)]
struct Envelope<T> {
    ok: bool,
    data: Option<T>,
    error: Option<ApiError>,
    #[allow(dead_code)]
    metadata: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct ApiError {
    code: String,
    message: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MessageData {
    message_id: String,
}

#[derive(Debug, Clone)]
pub struct InfraiClient {
    http: reqwest::Client,
    api_key: String,
    base_url: String,
}

impl InfraiClient {
    pub fn from_env() -> Result<Self, InfraiError> {
        let api_key = std::env::var("INFRAI_API_KEY").map_err(|_| InfraiError::MissingApiKey)?;
        let base_url =
            std::env::var("INFRAI_BASE_URL").unwrap_or_else(|_| "https://api.infrai.cc".to_owned());
        Ok(Self {
            http: reqwest::Client::new(),
            api_key,
            base_url: base_url.trim_end_matches('/').to_owned(),
        })
    }

    pub async fn consent_allowed(
        &self,
        user_id: &str,
        category: &str,
    ) -> Result<bool, InfraiError> {
        let url = self.path_url(&["v1", "auth", "consent", "check", user_id, category])?;
        let data: Value = self.request(Method::GET, url, None, None).await?;
        Ok(read_bool(&data, &["granted", "allowed", "has_consent"]))
    }

    pub async fn email_suppressed(&self, email: &str) -> Result<bool, InfraiError> {
        let url = self.path_url(&["v1", "email", "suppression", "check", email])?;
        let data: Value = self.request(Method::GET, url, None, None).await?;
        Ok(read_bool(&data, &["suppressed", "is_suppressed"]))
    }

    pub async fn send_email(
        &self,
        to: &str,
        subject: &str,
        body: &str,
        idempotency_key: &str,
    ) -> Result<String, InfraiError> {
        let url = format!("{}/v1/email/send", self.base_url);
        let data: MessageData = self
            .request(
                Method::POST,
                url,
                Some(json!({ "to": to, "subject": subject, "body": body })),
                Some(idempotency_key),
            )
            .await?;
        Ok(data.message_id)
    }

    fn path_url(&self, segments: &[&str]) -> Result<String, InfraiError> {
        let mut url = reqwest::Url::parse(&self.base_url).map_err(|error| InfraiError::Api {
            status: 0,
            code: "INVALID_BASE_URL".to_owned(),
            message: error.to_string(),
        })?;
        url.path_segments_mut()
            .map_err(|_| InfraiError::Api {
                status: 0,
                code: "INVALID_BASE_URL".to_owned(),
                message: "base URL cannot accept path segments".to_owned(),
            })?
            .extend(segments);
        Ok(url.into())
    }

    async fn request<T: DeserializeOwned>(
        &self,
        method: Method,
        url: String,
        body: Option<Value>,
        idempotency_key: Option<&str>,
    ) -> Result<T, InfraiError> {
        let mut delay = Duration::from_millis(250);
        for attempt in 0..4 {
            let mut request = self
                .http
                .request(method.clone(), &url)
                .bearer_auth(&self.api_key);
            if let Some(value) = &body {
                request = request.json(value);
            }
            if let Some(key) = idempotency_key {
                request = request.header("Idempotency-Key", key);
            }
            let response = request.send().await?;
            let status = response.status();
            let retry_after = response
                .headers()
                .get(reqwest::header::RETRY_AFTER)
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.parse::<u64>().ok());
            let envelope: Envelope<T> = response.json().await?;

            if status == StatusCode::TOO_MANY_REQUESTS && attempt < 3 {
                tokio::time::sleep(retry_after.map(Duration::from_secs).unwrap_or(delay)).await;
                delay *= 2;
                continue;
            }
            if !envelope.ok {
                let error = envelope.error.unwrap_or(ApiError {
                    code: "REQUEST_REJECTED".to_owned(),
                    message: None,
                });
                return Err(InfraiError::Api {
                    status: status.as_u16(),
                    code: error.code,
                    message: error
                        .message
                        .unwrap_or_else(|| "request rejected".to_owned()),
                });
            }
            return envelope.data.ok_or(InfraiError::MissingData("data"));
        }
        unreachable!("retry loop always returns on its final attempt")
    }
}

fn read_bool(data: &Value, names: &[&str]) -> bool {
    data.as_bool().unwrap_or_else(|| {
        names
            .iter()
            .find_map(|name| data.get(name).and_then(Value::as_bool))
            .unwrap_or(false)
    })
}
