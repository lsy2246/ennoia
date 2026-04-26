//! Shared API contract primitives for backend responses.

pub mod behavior;
pub mod memory;

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use ennoia_error_utils::normalize_error_message;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    BadRequest,
    Unauthorized,
    Forbidden,
    NotFound,
    Conflict,
    RateLimited,
    Timeout,
    PayloadTooLarge,
    Internal,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ApiErrorBody {
    pub code: ErrorCode,
    pub message: String,
    pub request_id: Option<String>,
    pub trace_id: Option<String>,
    #[serde(default)]
    pub details: serde_json::Value,
    pub retryable: bool,
}

#[derive(Debug, Clone)]
pub struct ApiError {
    status: StatusCode,
    body: ApiErrorBody,
}

impl ApiError {
    pub fn new(status: StatusCode, code: ErrorCode, message: impl Into<String>) -> Self {
        let message = normalize_error_message(message.into());
        Self {
            status,
            body: ApiErrorBody {
                code,
                message,
                request_id: None,
                trace_id: None,
                details: serde_json::Value::Object(Default::default()),
                retryable: false,
            },
        }
    }

    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, ErrorCode::BadRequest, message)
    }

    pub fn unauthorized(message: impl Into<String>) -> Self {
        Self::new(StatusCode::UNAUTHORIZED, ErrorCode::Unauthorized, message)
    }

    pub fn forbidden(message: impl Into<String>) -> Self {
        Self::new(StatusCode::FORBIDDEN, ErrorCode::Forbidden, message)
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(StatusCode::NOT_FOUND, ErrorCode::NotFound, message)
    }

    pub fn conflict(message: impl Into<String>) -> Self {
        Self::new(StatusCode::CONFLICT, ErrorCode::Conflict, message)
    }

    pub fn rate_limited(message: impl Into<String>) -> Self {
        Self::new(
            StatusCode::TOO_MANY_REQUESTS,
            ErrorCode::RateLimited,
            message,
        )
    }

    pub fn timeout(message: impl Into<String>) -> Self {
        let mut error = Self::new(StatusCode::REQUEST_TIMEOUT, ErrorCode::Timeout, message);
        error.body.retryable = true;
        error
    }

    pub fn payload_too_large(message: impl Into<String>) -> Self {
        Self::new(
            StatusCode::PAYLOAD_TOO_LARGE,
            ErrorCode::PayloadTooLarge,
            message,
        )
    }

    pub fn internal(message: impl Into<String>) -> Self {
        let mut error = Self::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            ErrorCode::Internal,
            message,
        );
        error.body.retryable = true;
        error
    }

    pub fn with_request_id(mut self, request_id: impl Into<String>) -> Self {
        self.body.request_id = Some(request_id.into());
        self
    }

    pub fn with_trace_id(mut self, trace_id: impl Into<String>) -> Self {
        self.body.trace_id = Some(trace_id.into());
        self
    }

    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.body.details = details;
        self
    }

    pub fn message(&self) -> &str {
        &self.body.message
    }
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.body.message)
    }
}

impl std::error::Error for ApiError {}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status, Json(self.body)).into_response()
    }
}
