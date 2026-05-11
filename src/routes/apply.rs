use axum::{extract::State, http::StatusCode, Json};
use serde_json::Value;
use tracing::{info, warn};

use crate::models::{ApplyRequest, ApplyResponse};
use crate::AppState;

pub async fn apply_to_vacancy(
    State(state): State<AppState>,
    Json(payload): Json<ApplyRequest>,
) -> (StatusCode, Json<Value>) {
    let vacancy_url = payload.vacancy_url().unwrap_or_default().trim().to_owned();
    let cover_letter = payload.cover_letter.trim().to_owned();
    let resume_id = payload
        .resume_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned);

    info!(%vacancy_url, has_resume_id = resume_id.is_some(), "received apply request");

    if vacancy_url.is_empty() {
        return failed(StatusCode::BAD_REQUEST, "vacancy_url must not be empty");
    }

    if !vacancy_url.starts_with("https://hh.ru/vacancy/")
        && !vacancy_url.starts_with("http://hh.ru/vacancy/")
    {
        return failed(
            StatusCode::BAD_REQUEST,
            "vacancy_url must be an HH.ru vacancy URL",
        );
    }

    if cover_letter.is_empty() {
        return failed(StatusCode::BAD_REQUEST, "cover_letter must not be empty");
    }

    match state
        .browser
        .apply_to_vacancy(&vacancy_url, &cover_letter, resume_id.as_deref())
        .await
    {
        Ok(()) => {
            info!(%vacancy_url, "manual review completed and success returned");
            (StatusCode::OK, Json(payload.into_applied_response()))
        }
        Err(err) => {
            let status = err.status_code();
            warn!(%vacancy_url, error = %err, status = %status, "apply request failed");
            failed(status, err.to_string())
        }
    }
}

fn failed(status: StatusCode, detail: impl Into<String>) -> (StatusCode, Json<Value>) {
    let body = serde_json::to_value(ApplyResponse::failed(detail))
        .unwrap_or_else(|_| Value::String("failed".to_owned()));
    (status, Json(body))
}
