use std::net::SocketAddr;
use std::task::{Context, Poll};

use axum::body::{Body, Full};
use axum::extract::ConnectInfo;
use axum::http::{HeaderValue, Request, StatusCode};
use axum::http::response::Parts;
use axum::response::Response;
use chrono::Utc;
use derive_more::{Display, Error};
use flinch::database::Database;
use flinch::doc::QueryBased;
use flinch::doc_trait::Document;
use flinch::extension::FuncResultExtractor;
use futures::future::BoxFuture;
use tower::{Layer, Service};
use utility::errors::AppResult;
use crate::layers::jwt::claims::Claims;
use crate::RATE_LIMITER_BUCKET;
use crate::state::SharedState;
use crate::util::body_from_parts;

const RATE_LIMITER_PREFIX: &str = "rl_";
const LIMIT_HEADER: &str = "x-ratelimit-limit";
const REMAINING_HEADER: &str = "x-ratelimit-remaining";
const RESET_HEADER: &str = "x-ratelimit-reset";
const RETRY_AFTER_HEADER: &str = "retry-after";

#[derive(Clone)]
pub struct RateLimiterLayer {
	pub state: SharedState,
	pub requests_by_second: i32,
	pub expire_in_seconds: i64,
	pub white_list: String,
}

impl RateLimiterLayer {
	pub fn new(
		state: SharedState,
		requests_by_second: i32,
		expire_in_seconds: i64,
		white_list: String,
	) -> Self {
		Self {
			state,
			requests_by_second,
			expire_in_seconds,
			white_list,
		}
	}
}

impl<S> Layer<S> for RateLimiterLayer {
	type Service = RateLimiterMiddleware<S>;

	fn layer(&self, inner: S) -> Self::Service {
		let white_list = self.white_list.split(',').map(|s| s.to_string()).collect();

		RateLimiterMiddleware {
			inner,
			state: self.state.clone(),
			requests_by_second: self.requests_by_second,
			expire_in_seconds: self.expire_in_seconds,
			white_list,
		}
	}
}

#[derive(Clone)]
pub struct RateLimiterMiddleware<S> {
	inner: S,
	state: SharedState,
	requests_by_second: i32,
	expire_in_seconds: i64,
	white_list: Vec<String>,
}

impl<S> Service<Request<Body>> for RateLimiterMiddleware<S>
	where
		S: Service<Request<Body>, Response=Response> + Send + 'static,
		S::Future: Send + 'static,
{
	type Response = S::Response;
	type Error = S::Error;
	// `BoxFuture` is a type alias for `Pin<Box<dyn Future + Send + 'a>>`
	type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

	fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
		self.inner.poll_ready(cx)
	}

	fn call(&mut self, request: Request<Body>) -> Self::Future {
		// Check JWT claims
		let claims = Claims::extract_from_request(request.headers(), &self.state.config.jwt_decoding_key.clone());

		// Get socket address
		let addr = request.extensions().get::<ConnectInfo<SocketAddr>>();

		// Initialize RateLimiterCheck
		let check = RateLimiterCheck::init(
			claims,
			addr,
			&self.white_list,
			self.requests_by_second,
		);
		let check_result = check.process(&self.state.flinch, self.expire_in_seconds);

		let future = self.inner.call(request);
		Box::pin(async move {
			let mut response = Response::default();

			response = match check_result {
				Ok((limit, _remaining, _reset)) if limit <= -1 => future.await?,
				Ok((limit, remaining, reset)) if remaining >= 0 => {
					// Limit OK
					// --------
					let (mut parts, body) = future.await?.into_parts();

					set_headers(&mut parts, limit, remaining, reset);

					Response::from_parts(parts, body)
				}
				Ok((limit, remaining, reset)) => {
					// Limit KO
					// --------
					let (mut parts, _body) = response.into_parts();

					// Headers
					set_headers(&mut parts, limit, remaining, reset);

					let msg = body_from_parts(&mut parts, StatusCode::TOO_MANY_REQUESTS, "Too Many Requests", None);
					Response::from_parts(parts, axum::body::boxed(Full::from(msg)))
				}
				Err(err) => match err {
					RateLimiterError::JwtDecoding => {
						let (mut parts, _body) = response.into_parts();
						let msg = body_from_parts(&mut parts, StatusCode::UNAUTHORIZED, "Unauthorized", None);
						Response::from_parts(parts, axum::body::boxed(Full::from(msg)))
					}
					_ => {
						let (mut parts, _body) = response.into_parts();
						let msg =
							body_from_parts(&mut parts, StatusCode::INTERNAL_SERVER_ERROR, &err.to_string(), None);
						Response::from_parts(parts, axum::body::boxed(Full::from(msg)))
					}
				},
			};

			Ok(response)
		})
	}
}

/// Set middleware specific headers
fn set_headers(parts: &mut Parts, limit: i32, remaining: i64, reset: i64) {
	if remaining >= 0 {
		// Limit OK
		if let Ok(limit) = HeaderValue::from_str(limit.to_string().as_str()) {
			parts.headers.insert(LIMIT_HEADER, limit);
		}

		if let Ok(remaining) = HeaderValue::from_str(remaining.to_string().as_str()) {
			parts.headers.insert(REMAINING_HEADER, remaining);
		}

		if let Ok(reset) = HeaderValue::from_str(reset.to_string().as_str()) {
			parts.headers.insert(RESET_HEADER, reset);
		}
	} else {
		// Limit reached
		if let Ok(reset) = HeaderValue::from_str(reset.to_string().as_str()) {
			parts.headers.insert(RETRY_AFTER_HEADER, reset);
		}
	}
}

#[derive(Display, Debug, Error, Clone, PartialEq)]
enum RateLimiterError {
	Ip,
	JwtDecoding,
	FlinchError { message: String },
}

#[derive(Debug, PartialEq)]
struct RateLimiterCheck {
	/// Potential error during the check
	error: Option<RateLimiterError>,

	/// Key used by the uniqueness of consumer
	key: Option<String>,

	/// Request limit (-1: unlimited, 0: when error, >=1: request limit)
	limit: i32,
}

impl Default for RateLimiterCheck {
	fn default() -> Self {
		Self {
			error: None,
			key: None,
			limit: -1,
		}
	}
}

impl RateLimiterCheck {
	/// Create a new instance of `RateLimiterCheck`
	fn new(error: Option<RateLimiterError>, key: Option<String>, limit: i32) -> Self {
		Self { error, key, limit }
	}

	fn init(
		claims: Option<AppResult<(Claims, String)>>,
		addr: Option<&ConnectInfo<SocketAddr>>,
		white_list: &[String],
		requests_by_second: i32,
	) -> Self {
		match claims {
			None => {
				let default_limit = requests_by_second;
				if default_limit == -1 {
					// No limit
					Self::default()
				} else {
					// Client Remote IP address
					match addr {
						None => Self::new(Some(RateLimiterError::Ip), None, 0),
						Some(remote_address) => {
							let mut key = remote_address.0.ip().to_string();
							// Check if IP address is in white list
							if white_list.contains(&key) {
								// No limit
								Self::default()
							} else {
								key.insert_str(0, RATE_LIMITER_PREFIX);

								Self::new(None, Some(key), default_limit)
							}
						}
					}
				}
			}
			Some(claims) => match claims {
				Ok((claims, _)) => {
					if claims.rate_limit == -1 {
						// No limit
						Self::default()
					} else {
						let mut key = claims.id;
						key.insert_str(0, RATE_LIMITER_PREFIX);

						Self::new(None, Some(key), claims.rate_limit)
					}
				}
				_ => Self::new(Some(RateLimiterError::JwtDecoding), None, 0),
			},
		}
	}

	/// Check limit, update Redis and returns information for headers
	fn process(&self, conn: &Database<QueryBased>, expire_in_seconds: i64) -> Result<(i32, i64, i64), RateLimiterError> {
		if let Some(err) = &self.error {
			Err(err.clone())
		} else if self.limit == -1 {
			Ok((self.limit, 0, 0))
		} else {
			let bucket = conn.using(RATE_LIMITER_BUCKET).unwrap();

			let now = Utc::now().timestamp();
			let mut remaining = self.limit as i64 - 1;
			let mut reset = expire_in_seconds;
			let mut expired_at = now + expire_in_seconds;

			let result = bucket.get(&self.key.as_ref().unwrap());

			if result.data.is_some() {
				let data = result.get_object();
				let expired_at_v = data.get("expired_at").ok_or(RateLimiterError::FlinchError {
					message: "Expired At not found in flinch".to_owned()
				})?;
				expired_at = expired_at_v.as_i64().unwrap();
				reset = expired_at - now;

				if reset <= 0 {
					// Expired cache
					// -------------
					expired_at = now + expire_in_seconds;
					reset = expire_in_seconds;
				} else {
					// Valid cache
					// -----------
					let frm = data.get("remaining").ok_or(RateLimiterError::FlinchError {
						message: "Remaining not found in flinch".to_owned()
					})?;
					remaining = frm.as_i64().unwrap();

					if remaining >= 0 {
						remaining -= 1;
					}
				}
			}

			let key = self.key.as_ref().unwrap();
			let mut map = serde_json::Map::new();
			map.insert("remaining".to_owned(), serde_json::Value::Number(serde_json::Number::from(remaining)));
			map.insert("expired_at".to_owned(), serde_json::Value::Number(serde_json::Number::from(expired_at)));
			let doc = QueryBased::from_value(&serde_json::Value::Object(map)).unwrap();

			futures::executor::block_on(async {
				let one_day = chrono::Local::now() + chrono::Duration::days(1);
				let _ = bucket.put(key.to_owned(), doc).await;
				bucket.put_ttl(key.to_owned(), one_day.timestamp()).await;
			});

			Ok((self.limit, remaining, reset))
		}
	}
}