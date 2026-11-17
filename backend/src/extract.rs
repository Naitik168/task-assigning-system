//! Custom extractors that turn framework rejections into our JSON error format.

use axum::{
    extract::{rejection::JsonRejection, FromRequest, Request},
    Json,
};
use serde::de::DeserializeOwned;
use validator::{Validate, ValidationErrors};

use crate::error::AppError;

/// Like `axum::Json`, but also runs `validator` rules and returns
/// `AppError` (JSON body) instead of axum's plain-text rejections.
pub struct ValidatedJson<T>(pub T);

impl<S, T> FromRequest<S> for ValidatedJson<T>
where
    T: DeserializeOwned + Validate,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let Json(value) = Json::<T>::from_request(req, state)
            .await
            .map_err(|rejection| match rejection {
                JsonRejection::JsonDataError(e) => AppError::Validation(e.body_text()),
                other => AppError::BadRequest(other.body_text()),
            })?;
        value
            .validate()
            .map_err(|e| AppError::Validation(format_validation_errors(&e)))?;
        Ok(Self(value))
    }
}

pub fn format_validation_errors(errors: &ValidationErrors) -> String {
    let mut messages: Vec<String> = errors
        .field_errors()
        .iter()
        .flat_map(|(field, errs)| {
            errs.iter().map(move |e| {
                e.message
                    .as_ref()
                    .map(|m| m.to_string())
                    .unwrap_or_else(|| format!("{field} is invalid"))
            })
        })
        .collect();
    messages.sort();
    messages.join("; ")
}
