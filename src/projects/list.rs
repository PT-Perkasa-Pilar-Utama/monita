//! The project list's page: the search, the owner and state filters, the sort and the page.

use super::{Project, Sort, store};
use crate::{accounts::User, platform::db::Db};

/// One page of the projects a user may open, and how many match in all.
pub struct ProjectPage {
    pub projects: Vec<Project>,
    pub total: i64,
    pub page: i64,
}

/// The project list's state filter (v1.1 Q43), as the cards' badges read: down, an issue, or up
/// (neither: up, in maintenance, waiting or paused).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StateFilter {
    All,
    Down,
    Issue,
    Up,
}

impl StateFilter {
    /// `?state=`; anything else is all.
    pub fn parse(raw: &str) -> Self {
        match raw {
            "down" => Self::Down,
            "issue" => Self::Issue,
            "up" => Self::Up,
            _ => Self::All,
        }
    }
}

/// Which page of the list a viewer asks for.
pub struct ListAt<'a> {
    pub q: &'a str,
    pub owner: &'a str,
    pub sort: Sort,
    pub state: StateFilter,
    pub page: i64,
    pub page_size: i64,
}

/// A tech lead pages through their own projects; a super admin through every project.
/// `page` counts from 1; a lower value reads page 1. `q` matches name, slug, owner email or
/// description, ignoring ASCII case. `owner` is an owner's email, or empty for any. `page_size`
/// is a cap from `platform::caps`: the list's, or the compact view's.
pub async fn page_for(db: &Db, user: &User, at: &ListAt<'_>) -> anyhow::Result<ProjectPage> {
    let q: String = at.q.trim().to_lowercase().chars().take(100).collect();
    let ids = match at.state {
        StateFilter::All => None,
        _ => {
            let (down, issue) = crate::incidents::troubled_projects(db).await?;
            Some(match at.state {
                StateFilter::Down => store::Ids::Only(down),
                StateFilter::Issue => store::Ids::Only(issue),
                _ => store::Ids::Not([down, issue].concat()),
            })
        }
    };
    let f = store::Filter {
        owner_id: (!user.sees_all()).then_some(user.id),
        q: &q,
        owner_email: at.owner.trim(),
        sort: at.sort,
        ids,
        viewer: user.pin_owner(),
    };
    let page = at.page.max(1);
    let offset = (page - 1).saturating_mul(at.page_size);
    let conn = db.read().await?;
    Ok(ProjectPage {
        total: store::count_matching(&conn, &f).await?,
        projects: store::page_matching(&conn, &f, at.page_size, offset).await?,
        page,
    })
}

/// Emails that own a project, for a super admin's owner filter.
pub async fn owner_emails(db: &Db) -> anyhow::Result<Vec<String>> {
    store::owner_emails(&db.read().await?).await
}
