//! Heartbeats: the list with each deadline, one heartbeat's page, and the form's writes.

use axum::{
    Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
};
use serde::{Deserialize, Serialize};

use super::{Paging, invalid, views};
use crate::{
    accounts::CurrentUser,
    app::AppState,
    checks::{
        heartbeat::{self, GRACES_S, Heartbeat, HeartbeatInput, LastPing, PERIODS_S, PingRow},
        insight::Insight,
        overview::{self, Due},
    },
    incidents::{self, SourceKind},
    platform::{
        caps, now, time,
        ui::{Body, UiError, UiResult, created, data},
    },
    projects,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/ui/projects/{slug}/heartbeats", get(list).post(create))
        .route(
            "/api/ui/projects/{slug}/heartbeats/{id}",
            get(detail).put(update).delete(delete),
        )
}

#[derive(Serialize)]
struct HeartbeatView {
    id: i64,
    name: String,
    period_s: i64,
    grace_s: i64,
    /// `1 day`, `5 minutes`.
    every: String,
    grace: String,
    severity: String,
    is_public: bool,
    alert: bool,
    auto_incident: bool,
    last_ping_at: Option<i64>,
    /// The newest ping, good or failed.
    last: Option<LastPing>,
    deadline_at: Option<i64>,
    created_at: i64,
    /// `up`, `down`, `issue` or `unknown` (waiting for the first ping).
    state: &'static str,
    /// How much of the wait before the deadline has passed.
    due: Due,
}

fn view(h: Heartbeat, last: Option<LastPing>, state: &'static str, now: i64) -> HeartbeatView {
    HeartbeatView {
        due: overview::due(&h, now),
        every: heartbeat::label(&PERIODS_S, h.period_s),
        grace: heartbeat::label(&GRACES_S, h.grace_s),
        id: h.id,
        period_s: h.period_s,
        grace_s: h.grace_s,
        severity: h.severity,
        is_public: h.is_public,
        alert: h.alert,
        auto_incident: h.auto_incident,
        last_ping_at: h.last_ping_at,
        last,
        deadline_at: h.deadline_at,
        created_at: h.created_at,
        state,
        name: h.name,
    }
}

#[derive(Serialize)]
struct ListView {
    heartbeats: Vec<HeartbeatView>,
    summary: serde_json::Value,
}

async fn list(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(slug): Path<String>,
) -> UiResult {
    let p = projects::find_for(&state.db, &user, &slug).await?;
    let all = heartbeat::list(&state.db, p.id).await?;
    let mut lasts = heartbeat::last_pings(&state.db, p.id).await?;
    let states = incidents::team_states_for(&state.db, p.id).await?;
    let state_of = |h: &Heartbeat| {
        h.state(states.get(&(SourceKind::Heartbeat, h.id)).copied())
            .0
    };
    let kinds: Vec<&str> = all.iter().map(state_of).collect();
    let (kind, text) = overview::summary(&kinds, "heartbeat");
    let t = now();
    Ok(data(ListView {
        heartbeats: all
            .into_iter()
            .map(|h| {
                let s = state_of(&h);
                let last = lasts.remove(&h.id);
                view(h, last, s, t)
            })
            .collect(),
        summary: serde_json::json!({ "kind": kind, "text": text }),
    }))
}

#[derive(Serialize)]
struct DetailView {
    heartbeat: HeartbeatView,
    /// The URL a job calls; the token in it is the heartbeat's whole secret.
    ping_url: String,
    stats: Vec<views::StatView>,
    insight: views::InsightView,
    pings: Vec<PingRow>,
    older: Option<String>,
}

async fn detail(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path((slug, id)): Path<(String, i64)>,
    Query(paging): Query<Paging>,
) -> UiResult {
    let p = projects::find_for(&state.db, &user, &slug).await?;
    let hb = heartbeat::find(&state.db, p.id, id)
        .await?
        .ok_or(UiError::NotFound)?;
    let pings = heartbeat::recent_pings(&state.db, id, paging.cursor()).await?;
    let older = (pings.len() as i64 == caps::PINGS_PAGE)
        .then(|| pings.last().map(|r| format!("{}.{}", r.received_at, r.id)))
        .flatten();
    let incident = incidents::team_states_for(&state.db, p.id)
        .await?
        .get(&(SourceKind::Heartbeat, id))
        .copied();
    let s = hb.state(incident).0;
    let off = crate::platform::settings::utc_offset(&state.db.read().await?).await?;
    let insight = Insight::heartbeat(&state.db, &hb, paging.range(), now(), off).await?;
    let every = heartbeat::label(&PERIODS_S, hb.period_s);
    let seen = hb.last_ping_at.is_some();
    let newest = heartbeat::last_pings(&state.db, p.id).await?.remove(&id);
    let failed_last = newest.as_ref().is_some_and(|l| !l.ok);
    let last = (
        "Last ping",
        newest
            .as_ref()
            .map_or_else(|| "Never".into(), |l| time::ago(l.at, insight.now)),
        if failed_last {
            format!("a fail ping, expected every {every}")
        } else {
            format!("expected every {every}")
        },
        if failed_last { "down" } else { "plain" },
    );
    let failed = insight.failed();
    let pings_stat = (
        "Pings",
        insight.series.total.to_string(),
        format!("{failed} failed, {}", insight.range_word()),
        if failed > 0 { "down" } else { "plain" },
    );
    let stats = views::stats([
        insight.streak_stat(seen),
        last,
        pings_stat,
        insight.incidents_stat(),
    ]);
    Ok(data(DetailView {
        ping_url: format!("{}/ping/{}", state.config.base_url, hb.token),
        heartbeat: view(hb, newest, s, insight.now),
        stats,
        insight: insight.into(),
        pings,
        older,
    }))
}

/// Strings, not numbers, as in the monitor form.
#[derive(Deserialize, Default)]
#[serde(default)]
struct HeartbeatForm {
    name: String,
    period_s: String,
    grace_s: String,
    severity: String,
    is_public: bool,
    /// Missing means on, so a client that never sends it keeps alerts and incidents.
    alert: Option<bool>,
    auto_incident: Option<bool>,
}

impl From<HeartbeatForm> for HeartbeatInput {
    fn from(f: HeartbeatForm) -> Self {
        HeartbeatInput {
            name: f.name.trim().to_string(),
            period_s: f.period_s.trim().parse().unwrap_or(0),
            grace_s: f.grace_s.trim().parse().unwrap_or(-1),
            severity: f.severity,
            is_public: f.is_public,
            alert: f.alert.unwrap_or(true),
            auto_incident: f.auto_incident.unwrap_or(true),
        }
    }
}

/// AC-12.01: 201 with the new heartbeat's id.
async fn create(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(slug): Path<String>,
    Body(f): Body<HeartbeatForm>,
) -> UiResult {
    let p = projects::find_for(&state.db, &user, &slug).await?;
    let input = HeartbeatInput::from(f);
    let errors = input.validate();
    if !errors.is_empty() {
        return Err(invalid(errors));
    }
    match heartbeat::create(&state.db, p.id, &input).await? {
        Some(id) => Ok(created(serde_json::json!({ "id": id }))),
        None => Err(UiError::field("name", caps::HEARTBEATS_FULL)),
    }
}

async fn update(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path((slug, id)): Path<(String, i64)>,
    Body(f): Body<HeartbeatForm>,
) -> UiResult {
    let p = projects::find_for(&state.db, &user, &slug).await?;
    let input = HeartbeatInput::from(f);
    let errors = input.validate();
    if !errors.is_empty() {
        return Err(invalid(errors));
    }
    if !heartbeat::update(&state.db, p.id, id, &input).await? {
        return Err(UiError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT.into_response())
}

async fn delete(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path((slug, id)): Path<(String, i64)>,
) -> UiResult {
    let p = projects::find_for(&state.db, &user, &slug).await?;
    heartbeat::delete(&state.db, &state.config.base_url, p.id, id).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}
