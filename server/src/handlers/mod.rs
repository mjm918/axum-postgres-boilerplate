use std::io;

use axum::BoxError;
use tower::timeout::error::Elapsed;
use tracing::error;
use utility::app_error;
use utility::errors::{AppResult,AppErrorCode,AppError};

pub async fn timeout_error(err: BoxError) -> AppResult<()> {
	if err.is::<Elapsed>() {
		Err(app_error!(AppErrorCode::Timeout))
	} else {
		Err(app_error!(AppErrorCode::InternalError, err.to_string()))
	}
}

/// Static file error
pub async fn static_file_error(err: io::Error) -> AppResult<()> {
	Err(app_error!(
        AppErrorCode::InternalError,
        "error when serving static file",
        format!("Unhandled internal error: {err}")
    ))
}