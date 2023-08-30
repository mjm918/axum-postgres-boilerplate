use std::net::AddrParseError;
use axum::{
	http::StatusCode,
	Json,
	response::{IntoResponse, Response},
};
use color_eyre::eyre::Result as EyreResult;
use derive_more::{Display, Error};
use eyre::ErrReport;
use flinch::errors::DbError;
use serde::{Deserialize, Serialize};
use serde::de::StdError;
use serde_json::json;
use sqlx::Error;
use tracing::error;

pub type AppResult<T> = EyreResult<T, AppError>;

#[derive(Deserialize, Serialize)]
pub struct AppErrorMessage {
	pub code: u16,
	pub message: String,
}

#[derive(Debug)]
pub enum AppErrorCode {
	InternalError,
	BadRequest,
	NotFound,
	UnprocessableEntity,
	Timeout,
	Unauthorized,
	TooManyRequests,
	MethodNotAllowed,
}

#[derive(Display, Debug, Error, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub enum AppError {
	#[display(fmt = "{message}")]
	FlinchError { message: String },

	#[display(fmt = "{message}")]
	LocalDbError { message: String },

	#[display(fmt = "{message}")]
	ConfigError { message: String },

	#[display(fmt = "{message}")]
	InternalError { message: String },

	#[display(fmt = "{message}")]
	BadRequest { message: String },

	#[display(fmt = "{message}")]
	NotFound { message: String },

	#[display(fmt = "{message}")]
	UnprocessableEntity { message: String },

	#[display(fmt = "Request Timeout")]
	Timeout,

	#[display(fmt = "Unauthorized")]
	Unauthorized,

	#[display(fmt = "Too Many Requests")]
	TooManyRequests,

	#[display(fmt = "Method Not Allowed")]
	MethodNotAllowed,
}

impl IntoResponse for AppError {
	fn into_response(self) -> Response {
		let status = match self {
			AppError::InternalError { .. } => StatusCode::INTERNAL_SERVER_ERROR,
			AppError::NotFound { .. } => StatusCode::NOT_FOUND,
			AppError::Unauthorized { .. } => StatusCode::UNAUTHORIZED,
			AppError::BadRequest { .. } => StatusCode::BAD_REQUEST,
			AppError::Timeout { .. } => StatusCode::REQUEST_TIMEOUT,
			AppError::TooManyRequests { .. } => StatusCode::TOO_MANY_REQUESTS,
			AppError::MethodNotAllowed { .. } => StatusCode::METHOD_NOT_ALLOWED,
			AppError::UnprocessableEntity { .. } => StatusCode::UNPROCESSABLE_ENTITY,
			_ => StatusCode::INTERNAL_SERVER_ERROR,
		};

		let body = Json(json!(AppErrorMessage {
            code: status.as_u16(),
            message: self.to_string(),
        }));

		(status, body).into_response()
	}
}

pub trait IntoInternalError {
	fn internal(message: AppError) -> AppError;
}

impl IntoInternalError for AppError {
	fn internal(message: AppError) -> AppError {
		Self::InternalError { message: message.to_string() }
	}
}

impl From<ErrReport> for AppError {
	fn from(value: ErrReport) -> Self {
		error!("[E100] {:?}",&value);
		Self::InternalError {
			message: format!("[E100]{:?}",value)
		}
	}
}

impl From<Box<dyn StdError>> for AppError {
	fn from(value: Box<dyn StdError>) -> Self {
		error!("[E100] {:?}",&value);
		Self::InternalError {
			message: format!("[E100] {:?}",value)
		}
	}
}

impl From<std::io::Error> for AppError {
	fn from(value: std::io::Error) -> Self {
		error!("[E100] {:?}",&value);
		Self::InternalError {
			message: format!("[E100] {:?}",value)
		}
	}
}

impl<T> From<std::sync::PoisonError<T>> for AppError {
	fn from(value: std::sync::PoisonError<T>) -> Self {
		error!("[E100] {:?}",&value);
		Self::InternalError {
			message: format!("[E100] {:?}",value)
		}
	}
}

impl From<pg_embed::pg_errors::PgEmbedError> for AppError {
	fn from(value: pg_embed::pg_errors::PgEmbedError) -> Self {
		error!("[E301] {:?}",&value);
		Self::LocalDbError {
			message: format!("[E301] {:?}",value)
		}
	}
}

impl From<serde_json::Error> for AppError {
	fn from(value: serde_json::Error) -> Self {
		error!("[S101] {:?}",&value);
		Self::InternalError {
			message: format!("[S101] {:?}",&value)
		}
	}
}

impl From<anyhow::Error> for AppError {
	fn from(value: anyhow::Error) -> Self {
		error!("[S109] {:?}",&value);
		Self::InternalError {
			message: format!("[S109] {:?}",&value)
		}
	}
}

impl From<DbError> for AppError {
	fn from(value: DbError) -> Self {
		error!("[M109] {:?}",&value);
		Self::InternalError {
			message: format!("[M109] {:?}",&value)
		}
	}
}

impl From<AddrParseError> for AppError {
	fn from(value: AddrParseError) -> Self {
		error!("[S109] {:?}",&value);
		Self::InternalError {
			message: format!("[S109] {:?}",&value)
		}
	}
}

impl From<hyper::Error> for AppError {
	fn from(value: hyper::Error) -> Self {
		error!("[S110] {:?}",&value);
		Self::InternalError {
			message: format!("[S110] {:?}",&value)
		}
	}
}

impl From<sqlx::Error> for AppError {
	fn from(value: Error) -> Self {
		error!("[E304] {:?}",&value);
		Self::LocalDbError {
			message: format!("[E304] {:?}",value)
		}
	}
}

impl From<flinch::headers::FlinchError> for AppError {
	fn from(value: flinch::headers::FlinchError) -> Self {
		Self::FlinchError {
			message: match value {
				flinch::headers::FlinchError::ExpressionError(err) => format!("{}",err),
				flinch::headers::FlinchError::QueryError(err) => format!("{}",err),
				flinch::headers::FlinchError::CollectionError(err) => format!("{}",err),
				flinch::headers::FlinchError::DocumentError(err) => format!("{}",err),
				flinch::headers::FlinchError::CustomError(err) => format!("{}",err),
				flinch::headers::FlinchError::IndexError(err) => format!("{}",err),
				flinch::headers::FlinchError::SchemaError(err) => format!("{}",err),
				flinch::headers::FlinchError::None => format!("OK"),
			}
		}
	}
}

#[macro_export]
macro_rules! app_error {
    ( $error:expr ) => {
        match $error {
            AppErrorCode::Timeout => AppError::Timeout,
            AppErrorCode::Unauthorized => AppError::Unauthorized,
            AppErrorCode::TooManyRequests => AppError::TooManyRequests,
            AppErrorCode::MethodNotAllowed => AppError::MethodNotAllowed,
            AppErrorCode::InternalError => AppError::InternalError {
                message: String::from("Internal Server Error"),
            },
            AppErrorCode::BadRequest => AppError::BadRequest {
                message: String::from("Bad Request"),
            },
            AppErrorCode::NotFound => AppError::NotFound {
                message: String::from("Not Found"),
            },
            AppErrorCode::UnprocessableEntity => AppError::UnprocessableEntity {
                message: String::from("Unprocessable Entity"),
            },
        }
    };

    ( $error:expr, $message:expr ) => {
        match $error {
            AppErrorCode::Timeout => AppError::Timeout,
            AppErrorCode::Unauthorized => AppError::Unauthorized,
            AppErrorCode::TooManyRequests => AppError::TooManyRequests,
            AppErrorCode::MethodNotAllowed => AppError::MethodNotAllowed,
            AppErrorCode::InternalError => {
                error!("{}", $message);
                AppError::InternalError {
                    message: $message.to_string(),
                }
            }
            AppErrorCode::BadRequest => AppError::BadRequest {
                message: $message.to_string(),
            },
            AppErrorCode::NotFound => AppError::NotFound {
                message: $message.to_string(),
            },
            AppErrorCode::UnprocessableEntity => AppError::UnprocessableEntity {
                message: $message.to_string(),
            },
        }
    };

    ( $error:expr, $message:expr, $details:expr ) => {
        match $error {
            AppErrorCode::Timeout => AppError::Timeout,
            AppErrorCode::Unauthorized => AppError::Unauthorized,
            AppErrorCode::TooManyRequests => AppError::TooManyRequests,
            AppErrorCode::MethodNotAllowed => AppError::MethodNotAllowed,
            AppErrorCode::InternalError => {
                error!("{}", $details);
                AppError::InternalError {
                    message: $message.to_string(),
                }
            }
            AppErrorCode::BadRequest => AppError::BadRequest {
                message: $message.to_string(),
            },
            AppErrorCode::NotFound => AppError::NotFound {
                message: $message.to_string(),
            },
            AppErrorCode::UnprocessableEntity => AppError::UnprocessableEntity {
                message: $message.to_string(),
            },
        }
    };
}