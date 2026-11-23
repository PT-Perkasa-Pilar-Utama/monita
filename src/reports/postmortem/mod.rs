//! Post-mortems (v1.2 Q63): the tech lead's account of a resolved incident, in Google's sections.
//! The state machine is in docs/decisions/2026-11-23-v1.2-scope.md, "Post-mortem states".
//! Publishing emails the incident PDF, post-mortem included, to the project's post-mortem list.

pub mod api;
mod store;

use serde::{Deserialize, Serialize};

use crate::{
    notify::{self, AlertError, Pdf},
    platform::db::Db,
    projects::Project,
};

/// Characters in one section: 8 sections at 6 bytes a character stay under the body cap.
pub const SECTION_MAX: usize = 2_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum State {
    None,
    Draft,
    Published,
}

impl State {
    pub fn as_str(self) -> &'static str {
        match self {
            State::None => "none",
            State::Draft => "draft",
            State::Published => "published",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Save,
    Publish,
    /// Published back to draft, to change it.
    Edit,
    Discard,
}

/// The sections, as the form and the PDF name them.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Text {
    pub summary: String,
    pub impact: String,
    pub root_cause: String,
    pub trigger: String,
    pub resolution: String,
    pub detection: String,
    pub action_items: String,
    pub lessons: String,
}

impl Text {
    /// Each section's form field, heading and text, in reading order.
    pub fn sections(&self) -> [(&'static str, &'static str, &str); 8] {
        [
            ("summary", "Summary", &self.summary),
            ("impact", "Impact", &self.impact),
            ("root_cause", "Root cause", &self.root_cause),
            ("trigger", "Trigger", &self.trigger),
            ("resolution", "Resolution", &self.resolution),
            ("detection", "Detection", &self.detection),
            ("action_items", "Action items", &self.action_items),
            ("lessons", "Lessons learned", &self.lessons),
        ]
    }

    fn trimmed(&self) -> Text {
        let t = |s: &str| s.trim().to_string();
        Text {
            summary: t(&self.summary),
            impact: t(&self.impact),
            root_cause: t(&self.root_cause),
            trigger: t(&self.trigger),
            resolution: t(&self.resolution),
            detection: t(&self.detection),
            action_items: t(&self.action_items),
            lessons: t(&self.lessons),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PostmortemError {
    /// Fields and what is wrong with each.
    #[error("Check the marked sections.")]
    Invalid(Vec<(&'static str, String)>),
    #[error("Resolve the incident before you write its post-mortem.")]
    IncidentOpen,
    #[error("{0}")]
    NotAllowed(&'static str),
    #[error("No such incident.")]
    NotFound,
    #[error(transparent)]
    Emails(#[from] AlertError),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

/// The state after `action` from `from`, or why the move is refused. Pure: the table's rows.
/// `was_published`: it went out once, so even as a draft again it cannot be deleted.
pub fn next(
    from: State,
    action: Action,
    resolved: bool,
    was_published: bool,
) -> Result<State, PostmortemError> {
    if !resolved {
        return Err(PostmortemError::IncidentOpen);
    }
    match (from, action) {
        (State::Draft, Action::Discard) if was_published => Err(PostmortemError::NotAllowed(
            "This post-mortem was published, so it cannot be deleted.",
        )),
        (State::None | State::Draft, Action::Save) => Ok(State::Draft),
        (State::None | State::Draft, Action::Publish) => Ok(State::Published),
        (State::Draft, Action::Discard) => Ok(State::None),
        (State::Published, Action::Edit) => Ok(State::Draft),
        (State::Published, Action::Save | Action::Publish) => Err(PostmortemError::NotAllowed(
            "This post-mortem is published, so select Edit before you change it.",
        )),
        (State::Published, Action::Discard) => Err(PostmortemError::NotAllowed(
            "A published post-mortem cannot be deleted.",
        )),
        (State::None, Action::Edit | Action::Discard) => Err(PostmortemError::NotAllowed(
            "This incident has no post-mortem yet.",
        )),
        (State::Draft, Action::Edit) => Err(PostmortemError::NotAllowed(
            "This post-mortem is already a draft.",
        )),
    }
}

/// Every section within `SECTION_MAX`; to publish, summary, root cause and resolution filled.
pub fn check(t: &Text, action: Action) -> Result<(), PostmortemError> {
    let mut bad = Vec::new();
    for (field, label, text) in t.sections() {
        if text.chars().count() > SECTION_MAX {
            bad.push((field, format!("{label} needs at most 2 000 characters.")));
        }
    }
    if action == Action::Publish {
        for (field, label, text) in &t.sections()[..] {
            let needed = matches!(*field, "summary" | "root_cause" | "resolution");
            if needed && text.trim().is_empty() && !bad.iter().any(|(f, _)| f == field) {
                bad.push((
                    field,
                    format!("Write the {} before you publish.", label.to_lowercase()),
                ));
            }
        }
    }
    if bad.is_empty() {
        Ok(())
    } else {
        Err(PostmortemError::Invalid(bad))
    }
}

/// A post-mortem as the incident page shows it; state `None` with empty text when there is none.
#[derive(Debug, Serialize)]
pub struct View {
    pub state: State,
    #[serde(flatten)]
    pub text: Text,
    pub updated_by: Option<String>,
    pub updated_at: Option<i64>,
    pub published_at: Option<i64>,
    /// How many addresses a publish emails now: 0 when the list is off.
    pub recipients: usize,
}

pub async fn view(db: &Db, project_id: i64, incident_id: i64) -> anyhow::Result<View> {
    let c = db.read().await?;
    let row = store::one(&c, project_id, incident_id).await?;
    let (on, emails) = store::recipients(&c, project_id).await?;
    Ok(View {
        state: row.as_ref().map_or(State::None, |r| r.state),
        text: row.as_ref().map(|r| r.text.clone()).unwrap_or_default(),
        updated_by: row.as_ref().map(|r| r.updated_by.clone()),
        updated_at: row.as_ref().map(|r| r.updated_at),
        published_at: row.as_ref().and_then(|r| r.published_at),
        recipients: if on {
            notify::split_emails(&emails).len()
        } else {
            0
        },
    })
}

/// The text the incident PDF includes, and whether it is still a draft.
pub async fn for_report(
    db: &Db,
    project_id: i64,
    incident_id: i64,
) -> anyhow::Result<Option<(Text, bool)>> {
    Ok(store::one(&db.read().await?, project_id, incident_id)
        .await?
        .map(|r| (r.text, r.state == State::Draft)))
}

/// What a move did: the new state and how many emails it queued.
#[derive(Debug, Serialize)]
pub struct Outcome {
    pub state: State,
    pub emailed: usize,
}

/// One move asked for: the action, the form's text (ignored by Edit and Discard), and who asks,
/// as the name the post-mortem shows.
pub struct Move<'a> {
    pub action: Action,
    pub text: &'a Text,
    pub by: &'a str,
}

/// Applies one move of the state machine. The form is checked on a read connection first; a
/// publish draws its PDF before the write, then the write reads the state and the list again,
/// stores the post-mortem and queues the emails together.
pub async fn apply(
    db: &Db,
    project: &Project,
    incident_id: i64,
    mv: Move<'_>,
    now: i64,
    off: i64,
) -> Result<Outcome, PostmortemError> {
    let Move { action, text, by } = mv;
    let text = text.trimmed();
    if matches!(action, Action::Save | Action::Publish) {
        check(&text, action)?;
    }
    let Some((incident, _, _)) = crate::incidents::find(db, project.id, incident_id).await? else {
        return Err(PostmortemError::NotFound);
    };
    let resolved = incident.resolved_at.is_some();
    let before = view(db, project.id, incident_id).await?;
    next(
        before.state,
        action,
        resolved,
        before.published_at.is_some(),
    )?;
    let pdf = if action == Action::Publish {
        let bytes =
            super::incident_pdf_with(db, project, incident_id, Some((&text, false)), now, off)
                .await?
                .ok_or(PostmortemError::NotFound)?;
        Some(Pdf {
            name: format!("{}-inc-{incident_id}-postmortem.pdf", project.slug),
            bytes,
        })
    } else {
        None
    };
    let subject = format!(
        "Post-mortem{}: INC-{incident_id} {}",
        if before.published_at.is_some() {
            " updated"
        } else {
            ""
        },
        incident.title
    );
    db.write(async |c| {
        let row = store::one(c, project.id, incident_id).await?;
        let from = row.as_ref().map_or(State::None, |r| r.state);
        let was = row.as_ref().is_some_and(|r| r.published_at.is_some());
        let to = match next(from, action, resolved, was) {
            Ok(to) => to,
            Err(e) => return Ok(Err(e)),
        };
        match action {
            Action::Save | Action::Publish => {
                store::save(c, project.id, incident_id, to, &text, by, now).await?;
            }
            Action::Edit => store::set_state(c, incident_id, to, by, now).await?,
            Action::Discard => store::delete(c, incident_id).await?,
        }
        let mut emailed = 0;
        if let Some(pdf) = &pdf {
            let (on, emails) = store::recipients(c, project.id).await?;
            if on {
                let body = format!(
                    "The post-mortem of INC-{incident_id}, {}, in {}, is attached as a PDF.\n\n\
                         You get this email because your address is on the project's post-mortem \
                         list, under Settings, Reports. Ask the project's tech lead to remove it.",
                    incident.title, project.name
                );
                for to in notify::split_emails(&emails) {
                    notify::queue_email_with_pdf(c, &to, &subject, &body, pdf, now).await?;
                    emailed += 1;
                }
            }
        }
        Ok(Ok(Outcome { state: to, emailed }))
    })
    .await?
}

/// The post-mortem list as its settings section shows it.
#[derive(Debug, Serialize)]
pub struct Recipients {
    pub enabled: bool,
    pub emails: Vec<String>,
}

pub async fn recipients(db: &Db, project_id: i64) -> anyhow::Result<Recipients> {
    let (enabled, emails) = store::recipients(&db.read().await?, project_id).await?;
    Ok(Recipients {
        enabled,
        emails: notify::split_emails(&emails),
    })
}

/// The list takes the alert cards' address rule; an empty list is fine while it is off.
pub async fn save_recipients(
    db: &Db,
    project_id: i64,
    enabled: bool,
    emails: &str,
) -> Result<(), PostmortemError> {
    let list = match notify::email_list(emails) {
        Ok(list) => list,
        Err(AlertError::NoEmails) if !enabled => Vec::new(),
        Err(e) => return Err(e.into()),
    };
    let joined = list.join("\n");
    db.write(async |c| store::save_recipients(c, project_id, enabled, &joined).await)
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_row_of_the_state_table_and_every_refusal() {
        use Action::*;
        use State::*;
        assert_eq!(next(None, Save, true, false).unwrap(), Draft);
        assert_eq!(next(None, Publish, true, false).unwrap(), Published);
        assert_eq!(next(Draft, Save, true, false).unwrap(), Draft);
        assert_eq!(next(Draft, Publish, true, true).unwrap(), Published);
        assert_eq!(next(Draft, Discard, true, false).unwrap(), None);
        assert!(
            matches!(
                next(Draft, Discard, true, true),
                Err(PostmortemError::NotAllowed(_))
            ),
            "a draft that was published stays"
        );
        assert_eq!(next(Published, Edit, true, true).unwrap(), Draft);
        for (from, action) in [
            (Published, Save),
            (Published, Publish),
            (Published, Discard),
            (None, Edit),
            (None, Discard),
            (Draft, Edit),
        ] {
            assert!(
                matches!(
                    next(from, action, true, false),
                    Err(PostmortemError::NotAllowed(_))
                ),
                "{from:?} {action:?}"
            );
        }
        for from in [None, Draft, Published] {
            assert!(matches!(
                next(from, Save, false, false),
                Err(PostmortemError::IncidentOpen)
            ));
        }
    }

    #[test]
    fn a_publish_needs_summary_root_cause_and_resolution_and_every_section_has_a_cap() {
        let mut t = Text::default();
        assert!(check(&t, Action::Save).is_ok(), "a draft may be empty");
        let Err(PostmortemError::Invalid(bad)) = check(&t, Action::Publish) else {
            panic!()
        };
        let fields: Vec<_> = bad.iter().map(|(f, _)| *f).collect();
        assert_eq!(fields, ["summary", "root_cause", "resolution"]);
        t.summary = "s".into();
        t.root_cause = "r".into();
        t.resolution = "x".repeat(SECTION_MAX);
        assert!(check(&t, Action::Publish).is_ok());
        t.lessons = "é".repeat(SECTION_MAX + 1);
        let Err(PostmortemError::Invalid(bad)) = check(&t, Action::Save) else {
            panic!()
        };
        assert_eq!(bad[0].0, "lessons");
    }
}
