use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("query must not be empty")]
    EmptyQuery,

    #[error("page must be greater than zero")]
    InvalidPage,

    #[error("session is missing")]
    SessionMissing,

    #[error("captcha detected")]
    CaptchaDetected,

    #[error("request timed out")]
    Timeout,

    #[error("ChromeDriver is not reachable at {0}. Start ChromeDriver first, for example: chromedriver --port=9515")]
    DriverUnavailable(String),

    #[error("browser profile is already in use: {0}. Close Chrome windows using this profile, or set BROWSER_PROFILE_DIR to a different directory")]
    ProfileInUse(String),

    #[error("browser error: {0}")]
    Browser(String),

    #[error("internal error: {0}")]
    Internal(String),
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub detail: String,
}

impl AppError {
    pub fn status_code(&self) -> StatusCode {
        match self {
            Self::EmptyQuery | Self::InvalidPage => StatusCode::BAD_REQUEST,
            Self::SessionMissing | Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::CaptchaDetected => StatusCode::SERVICE_UNAVAILABLE,
            Self::Timeout => StatusCode::GATEWAY_TIMEOUT,
            Self::DriverUnavailable(_) => StatusCode::BAD_GATEWAY,
            Self::ProfileInUse(_) => StatusCode::CONFLICT,
            Self::Browser(_) => StatusCode::BAD_GATEWAY,
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let body = Json(ErrorResponse {
            detail: self.to_string(),
        });

        (status, body).into_response()
    }
}
