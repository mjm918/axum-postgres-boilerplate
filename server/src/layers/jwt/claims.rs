use axum::http::{header, HeaderMap};
use chrono::Utc;
use jsonwebtoken::{Algorithm, decode, DecodingKey, encode, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use tracing::error;
use utility::app_error;
use utility::errors::{AppResult,AppErrorCode,AppError};

#[derive(Debug, Clone)]
pub struct Authenticated {
	pub username: String,
	pub token: String,
}

pub trait SizedStruct: Sized {}
impl SizedStruct for Authenticated {}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
	pub sub: String,
	pub exp: i64,
	pub iat: i64,
	pub nbf: i64,
	pub id: String,

	/// Max number of request by second (-1: unlimited)
	pub rate_limit: i32,
}

impl Claims {
	/// Extract claims from request headers
	pub fn extract_from_request(headers: &HeaderMap, decoding_key: &DecodingKey) -> Option<AppResult<(Self, String)>> {
		headers
			.get(header::AUTHORIZATION)
			.and_then(|h| h.to_str().ok())
			.and_then(|h| {
				let words = h.split("Bearer").collect::<Vec<&str>>();
				words.get(1).map(|w| w.trim())
			})
			.map(|token| Jwt::parse(token, decoding_key))
	}
}

pub struct Jwt {}

impl Jwt {
	/// Generate JWT
	pub fn generate(
		id: String,
		rate_limit: i32,
		encoding_key: &EncodingKey,
		jwt_lifetime: i64,
	) -> AppResult<(String, i64)> {
		let header = Header::new(Algorithm::HS512);
		let now = Utc::now().timestamp_nanos() / 1_000_000_000; // nanosecond -> second
		let expired_at = now + (jwt_lifetime * 3600);

		let payload = Claims {
			sub: id.clone(),
			exp: expired_at,
			iat: now,
			nbf: now,
			id,
			rate_limit,
		};

		let token = encode(&header, &payload, encoding_key).map_err(|err| {
			app_error!(
                AppErrorCode::InternalError,
                "error during JWT encoding",
                format!("error during JWT encoding: {err}")
            )
		})?;

		Ok((token, expired_at))
	}

	/// Parse JWT
	pub fn parse(token: &str, decoding_key: &DecodingKey) -> AppResult<(Claims, String)> {
		let validation = Validation::new(Algorithm::HS512);
		let token_data = decode::<Claims>(token, decoding_key, &validation).map_err(|err| {
			app_error!(
                AppErrorCode::InternalError,
                "error during JWT decoding",
                format!("error during JWT decoding: {err}")
            )
		})?;

		Ok((token_data.claims, format!("{}", token)))
	}
}