use axum::{Json};
use axum::extract::State;
use axum::response::Html;
use serde_json::Value;
use tera::Context;
use tracing::instrument;
use utility::errors::{AppError, AppResult, IntoInternalError};
use crate::extractor::ExtractRequestId;
use crate::state::SharedState;
use crate::TEMPLATES;

#[instrument(skip(_state), level = "trace")]
pub async fn say_ok(
	State(_state): State<SharedState>,
	ExtractRequestId(_request_id): ExtractRequestId,
) -> Json<Value> {
	Json(Value::Array(vec![Value::String(format!("OK"))]))
}

pub async fn health_check() -> AppResult<Html<String>> {
	Ok(Html(
		TEMPLATES
			.as_ref()
			.map_err(|err| AppError::internal(AppError::InternalError { message: err.to_string() }))?
			.render("html/index.html", &Context::new())
			.map_err(|err| AppError::internal(AppError::InternalError { message: err.to_string() }))?,
	))
}