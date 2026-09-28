//! One error type for every handler, rendered as the wire format the client
//! expects: `{"error": {"code", "message", "details?"}}` (see `ErrorCode` in
//! `web/src/api/types.ts`).
//!
//! Internal failures are logged with their full cause chain and answered with a
//! generic message — a database error string is a gift to an attacker and
//! useless to a user.

use axum::http::{header, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::{json, Value};
use std::collections::BTreeMap;

pub type ApiResult<T> = Result<T, ApiError>;

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("{0}")]
    BadRequest(String),
    /// Form errors keyed by field, and/or a list of messages for inputs that are
    /// validated as a set (trace inputs).
    #[error("{message}")]
    Validation {
        message: String,
        fields: BTreeMap<String, String>,
        errors: Vec<String>,
    },
    #[error("{0}")]
    Unauthorized(String),
    #[error("{0}")]
    Forbidden(String),
    #[error("missing or invalid CSRF token")]
    Csrf,
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    Conflict(String),
    #[error("{0}")]
    PaymentRequired(String),
    #[error("too many requests")]
    RateLimited { retry_after_secs: u64 },
    #[error("verify your email address first")]
    EmailUnverified,
    #[error("account temporarily locked")]
    AccountLocked { retry_after_secs: u64 },
    #[error("{0}")]
    Unavailable(String),
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

impl ApiError {
    pub fn not_found(what: &str) -> Self {
        Self::NotFound(format!("{what} not found"))
    }

    pub fn field(field: &str, message: impl Into<String>) -> Self {
        let message = message.into();
        Self::Validation {
            message: message.clone(),
            fields: BTreeMap::from([(field.to_string(), message)]),
            errors: vec![],
        }
    }

    pub fn fields(fields: BTreeMap<String, String>) -> Self {
        let message = fields
            .values()
            .next()
            .cloned()
            .unwrap_or_else(|| "invalid input".into());
        Self::Validation {
            message,
            fields,
            errors: vec![],
        }
    }

    pub fn errors(errors: Vec<String>) -> Self {
        Self::Validation {
            message: errors
                .first()
                .cloned()
                .unwrap_or_else(|| "invalid input".into()),
            fields: BTreeMap::new(),
            errors,
        }
    }

    pub fn internal(msg: impl std::fmt::Display) -> Self {
        Self::Internal(anyhow::anyhow!("{msg}"))
    }

    fn parts(&self) -> (StatusCode, &'static str, Option<Value>) {
        use ApiError::*;
        match self {
            BadRequest(_) => (StatusCode::BAD_REQUEST, "bad_request", None),
            Validation { fields, errors, .. } => {
                let mut d = serde_json::Map::new();
                if !fields.is_empty() {
                    d.insert("fields".into(), json!(fields));
                }
                if !errors.is_empty() {
                    d.insert("errors".into(), json!(errors));
                }
                (
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "validation",
                    Some(Value::Object(d)),
                )
            }
            Unauthorized(_) => (StatusCode::UNAUTHORIZED, "unauthorized", None),
            Forbidden(_) => (StatusCode::FORBIDDEN, "forbidden", None),
            Csrf => (StatusCode::FORBIDDEN, "csrf", None),
            NotFound(_) => (StatusCode::NOT_FOUND, "not_found", None),
            Conflict(_) => (StatusCode::CONFLICT, "conflict", None),
            PaymentRequired(_) => (StatusCode::PAYMENT_REQUIRED, "payment_required", None),
            RateLimited { retry_after_secs } => (
                StatusCode::TOO_MANY_REQUESTS,
                "rate_limited",
                Some(json!({ "retry_after_secs": retry_after_secs })),
            ),
            EmailUnverified => (StatusCode::FORBIDDEN, "email_unverified", None),
            AccountLocked { retry_after_secs } => (
                StatusCode::LOCKED,
                "account_locked",
                Some(json!({ "retry_after_secs": retry_after_secs })),
            ),
            Unavailable(_) => (StatusCode::SERVICE_UNAVAILABLE, "unavailable", None),
            Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, "internal", None),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, code, details) = self.parts();
        let message = match &self {
            ApiError::Internal(e) => {
                tracing::error!(error = ?e, "internal error");
                "Something went wrong on our side. Please try again.".to_string()
            }
            other => other.to_string(),
        };
        let mut body = json!({ "error": { "code": code, "message": message } });
        if let Some(d) = details {
            body["error"]["details"] = d;
        }
        let mut res = (status, Json(body)).into_response();
        if let ApiError::RateLimited { retry_after_secs }
        | ApiError::AccountLocked { retry_after_secs } = self
        {
            if let Ok(v) = HeaderValue::from_str(&retry_after_secs.to_string()) {
                res.headers_mut().insert(header::RETRY_AFTER, v);
            }
        }
        res
    }
}

impl From<sqlx::Error> for ApiError {
    fn from(e: sqlx::Error) -> Self {
        ApiError::Internal(e.into())
    }
}

impl From<redis::RedisError> for ApiError {
    fn from(e: redis::RedisError) -> Self {
        ApiError::Internal(e.into())
    }
}

/// True when `e` is a Postgres unique-constraint violation — the one database
/// error a handler turns into a user-facing answer (409) rather than a 500.
pub fn is_unique_violation(e: &sqlx::Error) -> bool {
    matches!(e, sqlx::Error::Database(db) if db.code().as_deref() == Some("23505"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use http_body_util::BodyExt;

    async fn body(res: Response) -> Value {
        let bytes = res.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&bytes).unwrap()
    }

    #[tokio::test]
    async fn validation_errors_carry_fields_and_lists() {
        let res = ApiError::field("email", "that is not an email address").into_response();
        assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
        let b = body(res).await;
        assert_eq!(b["error"]["code"], "validation");
        assert_eq!(
            b["error"]["details"]["fields"]["email"],
            "that is not an email address"
        );

        let b = body(ApiError::errors(vec!["a".into(), "b".into()]).into_response()).await;
        assert_eq!(b["error"]["details"]["errors"], json!(["a", "b"]));
    }

    #[tokio::test]
    async fn internal_errors_do_not_leak_their_cause() {
        let res = ApiError::internal("password authentication failed for user dsa").into_response();
        assert_eq!(res.status(), StatusCode::INTERNAL_SERVER_ERROR);
        let b = body(res).await;
        assert!(!b.to_string().contains("password authentication"));
    }

    #[tokio::test]
    async fn rate_limits_set_retry_after() {
        let res = ApiError::RateLimited {
            retry_after_secs: 17,
        }
        .into_response();
        assert_eq!(res.headers()[header::RETRY_AFTER], "17");
        let b = body(res).await;
        assert_eq!(b["error"]["details"]["retry_after_secs"], 17);
    }
}
