//! Shared pieces of HTML pages: the app shell data and the page error type.

use askama::Template;
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

use super::render::render;

/// The filter bar above a list: a search text and two selects, read from the query string.
/// An empty field matches every row. A value no row has matches none.
#[derive(Debug, Default, serde::Deserialize)]
pub struct ListFilter {
    #[serde(default)]
    pub q: String,
    #[serde(default)]
    pub state: String,
    #[serde(default)]
    pub severity: String,
}

impl ListFilter {
    /// Longest search text kept; the rest is cut.
    const Q_MAX: usize = 100;

    pub fn is_set(&self) -> bool {
        !self.needle().is_empty() || !self.state.is_empty() || !self.severity.is_empty()
    }

    fn needle(&self) -> String {
        self.q
            .trim()
            .chars()
            .take(Self::Q_MAX)
            .collect::<String>()
            .to_lowercase()
    }

    /// True when any of `text` contains the search, ignoring case, and the selects match.
    pub fn keeps(&self, text: &[&str], state: &str, severity: &str) -> bool {
        let needle = self.needle();
        (needle.is_empty() || text.iter().any(|t| t.to_lowercase().contains(&needle)))
            && (self.state.is_empty() || self.state == state)
            && (self.severity.is_empty() || self.severity == severity)
    }

    /// "4 monitors", or "2 of 4 monitors" when the filter hides some.
    pub fn count(&self, shown: usize, total: usize, noun: &str) -> String {
        let plural = if total == 1 { "" } else { "s" };
        if shown == total {
            format!("{total} {noun}{plural}")
        } else {
            format!("{shown} of {total} {noun}{plural}")
        }
    }
}

/// What `templates/layout/app.html` needs: who is signed in and where they are.
pub struct Shell {
    pub email: String,
    pub is_admin: bool,
    /// Nav key of the current page: `monitors`, `heartbeats`, `settings`, `users`, `limits`, `projects`.
    pub active: &'static str,
    pub project: Option<ProjectNav>,
    pub counts: TabCounts,
}

/// The numbers beside the project tabs: monitors, heartbeats, open incidents, open tickets.
#[derive(Default)]
pub struct TabCounts {
    pub monitors: i64,
    pub heartbeats: i64,
    pub incidents: i64,
    pub tickets: i64,
}

impl Shell {
    /// A page outside any project, such as Users or Limits.
    pub fn plain(email: &str, is_admin: bool, active: &'static str) -> Self {
        Shell {
            email: email.to_string(),
            is_admin,
            active,
            project: None,
            counts: TabCounts::default(),
        }
    }

    pub fn with_counts(mut self, counts: TabCounts) -> Self {
        self.counts = counts;
        self
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
            "settings" | "keys" | "allowlist" | "alerts" => "settings",
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

    pub fn in_project(mut self, name: &str, slug: &str, icon_at: Option<i64>) -> Self {
        self.project = Some(ProjectNav {
            name: name.to_string(),
            slug: slug.to_string(),
            icon_at,
        });
        self
    }
}

pub struct ProjectNav {
    pub name: String,
    pub slug: String,
    /// Versions the icon's URL; `None` shows the first letter instead.
    pub icon_at: Option<i64>,
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

#[cfg(test)]
mod tests {
    use super::ListFilter;

    fn filter(q: &str, state: &str, severity: &str) -> ListFilter {
        ListFilter {
            q: q.into(),
            state: state.into(),
            severity: severity.into(),
        }
    }

    #[test]
    fn an_empty_filter_keeps_every_row() {
        let f = filter("  ", "", "");
        assert!(!f.is_set());
        assert!(f.keeps(&["API"], "down", "critical"));
    }

    #[test]
    fn the_search_matches_any_text_ignoring_case_and_the_selects_match_exactly() {
        let f = filter(" PORTAL ", "down", "critical");
        assert!(f.keeps(&["API", "https://portal.example.com"], "down", "critical"));
        assert!(!f.keeps(&["API", "https://api.example.com"], "down", "critical"));
        assert!(!f.keeps(&["Portal"], "up", "critical"));
        assert!(!f.keeps(&["Portal"], "down", "warning"));
    }

    #[test]
    fn the_count_names_the_total_and_what_the_filter_hides() {
        let f = ListFilter::default();
        assert_eq!(f.count(1, 1, "monitor"), "1 monitor");
        assert_eq!(f.count(2, 4, "monitor"), "2 of 4 monitors");
    }
}
