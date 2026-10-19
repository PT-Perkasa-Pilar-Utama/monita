//! JSON for project backends and browsers: ref IDs, ticket submit, the reporter's list.

use super::*;

fn cors(mut res: Response) -> Response {
    let h = res.headers_mut();
    h.insert(
        header::ACCESS_CONTROL_ALLOW_ORIGIN,
        HeaderValue::from_static("*"),
    );
    h.insert(
        header::ACCESS_CONTROL_ALLOW_METHODS,
        HeaderValue::from_static("GET, POST, OPTIONS"),
    );
    h.insert(
        header::ACCESS_CONTROL_ALLOW_HEADERS,
        HeaderValue::from_static("content-type, x-ref-id"),
    );
    h.insert(
        header::ACCESS_CONTROL_MAX_AGE,
        HeaderValue::from_static("86400"),
    );
    res
}

pub(super) async fn preflight() -> Response {
    cors(StatusCode::NO_CONTENT.into_response())
}

fn ticket_error(e: TicketError) -> Response {
    match e {
        TicketError::Invalid(_) => {
            api_error(StatusCode::BAD_REQUEST, "invalid_request", &e.to_string())
        }
        TicketError::RefIdInvalid => api_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "ref_id_invalid",
            &e.to_string(),
        ),
        TicketError::DailyCap => api_error(
            StatusCode::TOO_MANY_REQUESTS,
            "daily_cap_reached",
            &e.to_string(),
        ),
        TicketError::NeedReason => {
            api_error(StatusCode::BAD_REQUEST, "invalid_request", &e.to_string())
        }
        TicketError::Other(e) => {
            tracing::error!(error = %e, "ticket api failed");
            api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal",
                "Unexpected failure.",
            )
        }
    }
}

#[derive(Deserialize, Default)]
pub(super) struct RefIdRequest {
    #[serde(default)]
    single_use: bool,
}

pub(super) async fn issue_ref_id(
    State(state): State<AppState>,
    ApiProject(project): ApiProject,
    body: Bytes,
) -> Response {
    // The body is optional: no body, `null` and `{}` all mean the defaults.
    let req = if body.iter().all(u8::is_ascii_whitespace) {
        RefIdRequest::default()
    } else {
        match serde_json::from_slice::<Option<RefIdRequest>>(&body) {
            Ok(r) => r.unwrap_or_default(),
            Err(_) => {
                return api_error(
                    StatusCode::BAD_REQUEST,
                    "invalid_request",
                    "Body must be JSON like {\"single_use\": true}.",
                );
            }
        }
    };
    match crate::tickets::issue_ref_id(&state.db, project.id, req.single_use).await {
        Ok((ref_id, created_at)) => (
            StatusCode::CREATED,
            Json(serde_json::json!({ "data": { "ref_id": ref_id, "single_use": req.single_use, "created_at": created_at } })),
        )
            .into_response(),
        Err(e) => ticket_error(TicketError::Other(e)),
    }
}

#[derive(Deserialize)]
pub(super) struct TicketBody {
    ref_id: String,
    title: String,
    description: String,
    page_url: Option<String>,
}

impl TicketBody {
    fn ticket(&self) -> NewTicket {
        NewTicket {
            title: self.title.clone(),
            description: self.description.clone(),
            page_url: self
                .page_url
                .clone()
                .map(|u| u.trim().to_string())
                .filter(|u| !u.is_empty()),
        }
    }
}

pub(super) async fn api_submit(State(state): State<AppState>, body: Bytes) -> Response {
    let Ok(req) = serde_json::from_slice::<TicketBody>(&body) else {
        return cors(api_error(
            StatusCode::BAD_REQUEST,
            "invalid_request",
            "Body must be JSON with ref_id, title and description.",
        ));
    };
    cors(match crate::tickets::submit(&state.db, &state.config.base_url, None, &req.ref_id, &req.ticket()).await {
        Ok(c) => (StatusCode::CREATED, Json(serde_json::json!({ "data": { "number": c.number, "stage": "triage", "created_at": c.created_at } }))).into_response(),
        Err(e) => ticket_error(e),
    })
}

pub(super) async fn api_mine(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let Some(ref_id) = headers
        .get("x-ref-id")
        .and_then(|v| v.to_str().ok())
        .filter(|v| !v.is_empty())
    else {
        return cors(api_error(
            StatusCode::UNAUTHORIZED,
            "unauthorized",
            "Send the ref ID in the X-Ref-Id header.",
        ));
    };
    cors(match crate::tickets::mine(&state.db, ref_id, None).await {
        Ok(list) => Json(serde_json::json!({ "data": list })).into_response(),
        Err(e) => ticket_error(e),
    })
}
