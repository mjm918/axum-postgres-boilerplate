//! HTTP request validation module

use serde_json::json;
use validator::Validate;
use tracing::error;
use utility::app_error;
use utility::errors::{AppResult,AppErrorCode, AppError};

/// Validate the HTTP request parameters
pub fn validate_request_data<T: Validate>(data: &T) -> AppResult<()> {
	match data.validate() {
		Ok(_) => Ok(()),
		Err(errors) => Err(app_error!(AppErrorCode::BadRequest, json!(errors).to_string())),
	}
}