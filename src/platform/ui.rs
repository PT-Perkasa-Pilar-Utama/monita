//! The JSON surface the React frontend calls, under `/api/ui/`. Errors use the api-specs 01
//! envelope, plus `fields` (form field name to message) on a 422.

use std::collections::BTreeMap;

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;

use super::page::PageError;

#[derive(Debug)]
pub enum UiError {
    Unauthorized,
    Forbidden,
    NotFound,
    /// A link or token that worked once and no longer does.
    Gone(String),
    /// 422: a message for the form, and the fields it concerns.
    Invalid {
        message: String,
        fields: BTreeMap<String, String>,
    },
    /// 413: a body over the 128 KiB limit (SEC-19).
    TooLarge,
    /// 429: a daily cap is reached (SEC-13).
    DailyCap(String),
    Internal(anyhow::Error),
}

impl UiError {
    /// A form error with no particular field.
    pub fn invalid(message: impl Into<String>) -> Self {
        UiError::Invalid {
            message: message.into(),
            fields: BTreeMap::new(),
        }
    }

    /// A form error on one field.
    pub fn field(field: &str, message: impl Into<String>) -> Self {
        let message = message.into();
        UiError::Invalid {
            fields: BTreeMap::from([(field.to_string(), message.clone())]),
            message,
        }
    }

    /// A form error on several fields; the first one is the message.
    pub fn fields(fields: BTreeMap<String, String>) -> Self {
        let message = fields
            .values()
            .next()
            .cloned()
            .unwrap_or_else(|| "Fix the fields marked below.".into());
        UiError::Invalid { message, fields }
    }
}

impl From<anyhow::Error> for UiError {
    fn from(e: anyhow::Error) -> Self {
        UiError::Internal(e)
    }
}

impl From<PageError> for UiError {
    fn from(e: PageError) -> Self {
        match e {
            PageError::NotFound => UiError::NotFound,
            PageError::Forbidden => UiError::Forbidden,
            PageError::Internal(e) => UiError::Internal(e),
        }
    }
}

#[derive(Serialize)]
struct ErrorBody<'a> {
    code: &'a str,
    message: String,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    fields: BTreeMap<String, String>,
}

impl IntoResponse for UiError {
    fn into_response(self) -> Response {
        let (status, code, message, fields) = match self {
            UiError::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                "unauthorized",
                "Sign in to continue.".to_string(),
                BTreeMap::new(),
            ),
            UiError::Forbidden => (
                StatusCode::FORBIDDEN,
                "forbidden",
                "Your account cannot do this.".to_string(),
                BTreeMap::new(),
            ),
            UiError::NotFound => (
                StatusCode::NOT_FOUND,
                "not_found",
                "This does not exist, or you cannot open it.".to_string(),
                BTreeMap::new(),
            ),
            UiError::Gone(m) => (StatusCode::GONE, "gone", m, BTreeMap::new()),
            UiError::DailyCap(m) => (
                StatusCode::TOO_MANY_REQUESTS,
                "daily_cap_reached",
                m,
                BTreeMap::new(),
            ),
            UiError::TooLarge => (
                StatusCode::PAYLOAD_TOO_LARGE,
                "too_large",
                "The request body is over 128 KiB.".to_string(),
                BTreeMap::new(),
            ),
            UiError::Invalid { message, fields } => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "validation_failed",
                message,
                fields,
            ),
            UiError::Internal(e) => {
                tracing::error!(error = %e, "ui request failed");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal",
                    "The error is in the server log. Try again in a moment.".to_string(),
                    BTreeMap::new(),
                )
            }
        };
        let body = ErrorBody {
            code,
            message,
            fields,
        };
        (status, Json(serde_json::json!({ "error": body }))).into_response()
    }
}

/// `{"data": …}`, the api-specs 01 success envelope.
pub fn data<T: Serialize>(value: T) -> Response {
    Json(serde_json::json!({ "data": value })).into_response()
}

/// `201` with `{"data": …}`.
pub fn created<T: Serialize>(value: T) -> Response {
    (
        StatusCode::CREATED,
        Json(serde_json::json!({ "data": value })),
    )
        .into_response()
}

pub type UiResult = Result<Response, UiError>;

/// Public reads may be cached for 30 s by browsers and proxies. Only a success is: a cached 500
/// would outlive its cause.
pub fn public_cache(mut res: Response) -> Response {
    if res.status().is_success() {
        res.headers_mut().insert(
            axum::http::header::CACHE_CONTROL,
            axum::http::HeaderValue::from_static("public, max-age=30"),
        );
    }
    res
}

/// A JSON body. A body that is not JSON, or not the expected shape, is a 422 in the envelope
/// rather than axum's plain-text rejection.
pub struct Body<T>(pub T);

impl<S, T> axum::extract::FromRequest<S> for Body<T>
where
    T: serde::de::DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = UiError;

    async fn from_request(req: axum::extract::Request, state: &S) -> Result<Self, Self::Rejection> {
        match Json::<T>::from_request(req, state).await {
            Ok(Json(v)) => Ok(Body(v)),
            Err(e) if e.status() == StatusCode::PAYLOAD_TOO_LARGE => Err(UiError::TooLarge),
            Err(e) => Err(UiError::invalid(format!(
                "The request body is not what this form sends: {}",
                e.body_text()
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_success_is_publicly_cached() {
        let ok = public_cache(StatusCode::OK.into_response());
        assert_eq!(ok.headers()["cache-control"], "public, max-age=30");
        let failed = public_cache(StatusCode::INTERNAL_SERVER_ERROR.into_response());
        assert!(failed.headers().get("cache-control").is_none());
    }
}
