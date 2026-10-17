//! The error type of domain functions that a page or route turns into a status.

use axum::response::{IntoResponse, Response};

/// 404 hides what exists, 403 refuses, everything else is logged as a 500.
#[derive(Debug)]
pub enum PageError {
    NotFound,
    Forbidden,
    Internal(anyhow::Error),
}

impl<E: Into<anyhow::Error>> From<E> for PageError {
    fn from(e: E) -> Self {
        PageError::Internal(e.into())
    }
}

/// The api-specs 01 envelope, as `UiError` writes it.
impl IntoResponse for PageError {
    fn into_response(self) -> Response {
        super::ui::UiError::from(self).into_response()
    }
}
