//! Shared pieces of HTML pages: the app shell data and the page error type.

use askama::Template;
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

use super::render::render;

/// What `templates/layout/app.html` needs: who is signed in and where they are.
pub struct Shell {
    pub email: String,
    pub is_admin: bool,
    /// Nav key of the current page: `monitors`, `heartbeats`, `settings`, `users`, `limits`, `projects`.
    pub active: &'static str,
    pub project: Option<ProjectNav>,
}

impl Shell {
    /// A page outside any project, such as Users or Limits.
    pub fn plain(email: &str, is_admin: bool, active: &'static str) -> Self {
        Shell {
            email: email.to_string(),
            is_admin,
            active,
            project: None,
        }
    }

    /// The first letter of the email, for the avatar in the user menu.
    pub fn initial(&self) -> String {
        self.email
            .chars()
            .next()
            .map(|c| c.to_uppercase().to_string())
            .unwrap_or_default()
    }

    /// The email split before its `@`, so a narrow sidebar wraps it there and not mid-word.
    pub fn email_parts(&self) -> (&str, &str) {
        self.email
            .split_at(self.email.find('@').unwrap_or(self.email.len()))
    }

    /// The pages that share a left sidebar: a project's settings, or the admin pages.
    pub fn area(&self) -> &'static str {
        match self.active {
            "settings" | "alerts" => "settings",
            "users" | "limits" => "admin",
            _ => "",
        }
    }

    pub fn role(&self) -> &'static str {
        if self.is_admin {
            "Super admin"
        } else {
            "Tech lead"
        }
    }

    pub fn in_project(mut self, name: &str, slug: &str) -> Self {
        self.project = Some(ProjectNav {
            name: name.to_string(),
            slug: slug.to_string(),
        });
        self
    }
}

pub struct ProjectNav {
    pub name: String,
    pub slug: String,
}

impl ProjectNav {
    pub fn initial(&self) -> char {
        self.name.chars().next().unwrap_or('?').to_ascii_uppercase()
    }
}

/// Error for HTML handlers: 404 hides what exists, everything else is logged as a 500.
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

#[derive(Template)]
#[template(path = "error.html")]
struct ErrorPage<'a> {
    title: &'a str,
    text: &'a str,
}

impl IntoResponse for PageError {
    fn into_response(self) -> Response {
        let (status, title, text) = match self {
            PageError::NotFound => (
                StatusCode::NOT_FOUND,
                "Not found",
                "This page does not exist, or you cannot open it.",
            ),
            PageError::Forbidden => (
                StatusCode::FORBIDDEN,
                "Not allowed",
                "Your account cannot open this page.",
            ),
            PageError::Internal(e) => {
                tracing::error!(error = %e, "page failed");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Something broke",
                    "The error is in the server log. Try again in a moment.",
                )
            }
        };
        (status, render(&ErrorPage { title, text })).into_response()
    }
}
