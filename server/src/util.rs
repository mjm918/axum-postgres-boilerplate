use std::str::from_utf8;

use axum::body::Full;
use axum::headers::HeaderName;
use axum::http::{
	header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE, ORIGIN},
	HeaderValue,
	Method, Request, response::Parts,
};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use bytes::Bytes;
use hyper::body::to_bytes;
use hyper::StatusCode;
use jsonwebtoken::{DecodingKey, EncodingKey};
use tower_http::cors::{AllowOrigin, Any, CorsLayer};
use tower_http::request_id::{MakeRequestId, RequestId};
use tracing::error;
use uuid::Uuid;
use utility::app_error;
use utility::errors::{AppErrorMessage,AppErrorCode,AppError};
use utility::env::Variables;

/// Construct response body from `Parts`, status code, message and headers
pub fn body_from_parts(
	parts: &mut Parts,
	status_code: StatusCode,
	message: &str,
	headers: Option<Vec<(HeaderName, HeaderValue)>>,
) -> Bytes {
	// Status
	parts.status = status_code;

	// Headers
	parts
		.headers
		.insert(CONTENT_TYPE, HeaderValue::from_static(mime::APPLICATION_JSON.as_ref()));
	if let Some(headers) = headers {
		for header in headers {
			parts.headers.insert(header.0, header.1);
		}
	}

	// Body
	let msg = serde_json::json!(AppErrorMessage {
        code: status_code.as_u16(),
        message: String::from(message),
    });

	Bytes::from(msg.to_string())
}

/// Request ID middleware
#[derive(Clone, Copy)]
pub struct MakeRequestUuid;

impl MakeRequestId for MakeRequestUuid {
	fn make_request_id<B>(&mut self, _request: &Request<B>) -> Option<RequestId> {
		let id = Uuid::new_v4().to_string().parse();
		match id {
			Ok(id) => Some(RequestId::new(id)),
			_ => None,
		}
	}
}

// =============== Utils ================

/// Convert `HeaderValue` to `&str`
pub fn header_value_to_str(value: Option<&HeaderValue>) -> &str {
	match value {
		Some(value) => from_utf8(value.as_bytes()).unwrap_or_default(),
		None => "",
	}
}

// ================ CORS ================

/// CORS layer
pub fn cors(config: &Variables) -> CorsLayer {
	let allow_origin = config.cors_allow_origin.clone();

	let layer = CorsLayer::new()
		.allow_methods([Method::GET, Method::POST, Method::PUT, Method::PATCH, Method::DELETE])
		.allow_headers([AUTHORIZATION, ACCEPT, ORIGIN, CONTENT_TYPE]);

	if allow_origin == "*" {
		layer.allow_origin(Any)
	} else {
		let origins = allow_origin
			.split(',')
			.filter(|url| *url != "*" && !url.is_empty())
			.filter_map(|url| url.parse().ok())
			.collect::<Vec<HeaderValue>>();

		if origins.is_empty() {
			layer.allow_origin(Any)
		} else {
			layer
				.allow_origin(AllowOrigin::predicate(move |origin: &HeaderValue, _| {
					origins.contains(origin)
				}))
				.allow_credentials(true)
		}
	}
}

// =============== Override some HTTP errors ================

/// Layer which override some HTTP errors by using `AppError`
pub async fn override_http_errors<B>(req: Request<B>, next: Next<B>) -> impl IntoResponse {
	let response = next.run(req).await;

	// If it is an image, audio or video, we return response
	let headers = response.headers();
	if let Some(content_type) = headers.get("content-type") {
		let content_type = content_type.to_str().unwrap_or_default();
		if content_type.starts_with("image/")
			|| content_type.starts_with("audio/")
			|| content_type.starts_with("video/")
		{
			return response;
		}
	}

	let (parts, body) = response.into_parts();
	match to_bytes(body).await {
		Ok(body_bytes) => match String::from_utf8(body_bytes.to_vec()) {
			Ok(body) => match parts.status {
				StatusCode::METHOD_NOT_ALLOWED => app_error!(AppErrorCode::MethodNotAllowed).into_response(),
				StatusCode::UNPROCESSABLE_ENTITY => app_error!(AppErrorCode::UnprocessableEntity, body).into_response(),
				_ => Response::from_parts(parts, axum::body::boxed(Full::from(body))),
			},
			Err(err) => app_error!(AppErrorCode::InternalError, err.to_string()).into_response(),
		},
		Err(err) => app_error!(AppErrorCode::InternalError, err.to_string()).into_response(),
	}
}

pub struct ConfigState {
	pub jwt_encoding_key: EncodingKey,
	pub jwt_decoding_key: DecodingKey,
	pub jwt_lifetime: i64,
}

impl From<Variables> for ConfigState {
	fn from(config: Variables) -> Self {
		Self {
			jwt_encoding_key: EncodingKey::from_secret(config.jwt_secret_key.clone().as_bytes()),
			jwt_decoding_key: DecodingKey::from_secret(config.jwt_secret_key.as_bytes()),
			jwt_lifetime: config.jwt_lifetime,
		}
	}
}