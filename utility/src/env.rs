#![allow(deprecated)]
use color_eyre::Result;
use serde::{Deserialize, Serialize};
use tracing::error;

/// Represents configuration structure
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Variables {
	/// Environment: `developement` or `production`
	pub environment: String,

	/// Logs used by App, sqlx, etc.
	pub rust_log: String,

	/// Path of log files
	pub log_path: String,
	pub log_file: String,

	/// Postgres Config
	pub db_path: String,
	pub db_user: String,
	pub db_pw: String,
	pub db_port: u16,
	pub db_timeout: u64,

	/// TLS
	pub tls_policy: String,
	pub tls_cert_path: String,
	pub tls_key_path: String,

	///JWT
	pub jwt_secret_key: String,
	pub jwt_lifetime: i64,

	/// CORS
	pub cors_allow_origin: String,

	/// Limiter
	pub limiter_enabled: bool,
	pub limiter_requests_by_second: i32,
	pub limiter_expire_in_seconds: i64,
	pub limiter_white_list: String,

	/// Prometheus metrics enabled
	pub prometheus_metrics_enabled: bool,

	/// Basic Auth
	pub basic_user: String,
	pub basic_pw: String,

	/// Server URL
	pub server_url: String,
	pub server_port: u16,
	pub request_timeout: u64,
}

impl Default for Variables {
	fn default() -> Self {
		Self {
			environment: format!("production"),
			rust_log: format!("trace,sqlx=error,config=trace"),
			log_path: format!("./qaswa-log"),
			log_file: format!("qaswa"),
			db_path: format!("./qaswa-data"),
			db_user: format!("postgres"),
			db_pw: format!("password"),
			db_port: 5432,
			db_timeout: 15,
			tls_policy: format!("native"),
			tls_cert_path: format!("./certs/ssl.cert"),
			tls_key_path: format!("./certs/ssl.key"),
			jwt_secret_key: format!("eyJhbGciOiJIUzI1NiJ9.eyJSb2xlIjoiQWRtaW4iLCJJc3N1ZXIiOiJNb2hhbW1hZCBKdWxmaWthciBNYWhtdWQiLCJVc2VybmFtZSI6ImVhc3lzYWxlcyIsImV4cCI6MTY4NzUwMTQ2MiwiaWF0IjoxNjg3NTAxNDYyfQ.P3jftRAbhqteL69AVfEh6QlOT-_Bn8XRQfSjfZccbfo"),
			jwt_lifetime: 24,
			cors_allow_origin: format!("*"),
			limiter_enabled: true,
			limiter_requests_by_second: 100,
			limiter_expire_in_seconds: 30,
			limiter_white_list: "".to_string(),
			prometheus_metrics_enabled: true,
			basic_user: "root".to_string(),
			basic_pw: "qaswa123@".to_string(),
			server_url: "0.0.0.0".to_string(),
			server_port: 9097,
			request_timeout: 10,
		}
	}
}

impl Variables {
	/// from_env loads configuration from environment variables
	pub fn from_env() -> Result<Variables> {
		dotenvy::dotenv().ok();
		match config::Config::builder()
			.add_source(config::Environment::default())
			.build() {
			Ok(cfg) => {
				Ok(cfg.try_deserialize()?)
			}
			Err(err) => {
				error!("config error - {:?}",err);
				Ok(Variables::default())
			}
		}
	}
}

impl ToString for Variables {
	fn to_string(&self) -> String {
		serde_json::to_string(self).unwrap()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn config() {
		let config = Variables::from_env();
		assert!(config.is_ok(), "{:?}", config.err());
	}
}