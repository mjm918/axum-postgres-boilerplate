use std::future::ready;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use axum::error_handling::HandleErrorLayer;
use axum::{middleware, Router};
use axum::routing::get;
use tower::ServiceBuilder;
use tower_http::ServiceBuilderExt;
use tower_http::services::ServeDir;
use tracing::{error, info, trace};
use utility::env::Variables;
use utility::errors::AppResult;
use crate::{APP_NAME, handlers, routes};
use crate::certs::init_ssl_certs;
use crate::layers::auth::BasicAuthLayer;
use crate::layers::prometheus::PrometheusMetric;
use crate::layers::rate_limiter::RateLimiterLayer;
use crate::setup::{get_flinch, graceful_shutdown, shutdown_signal};
use crate::state::{SharedState, State};
use crate::util::MakeRequestUuid;

pub async fn serve() -> AppResult<()> {
	color_eyre::install()?;

	let settings = Variables::from_env()?;
	// Init Flinch Db
	let mem_db = get_flinch().await;
	// Setup Postgres
	trace!("installing or starting postgres...");
	let mut pg_server = db::setup::install_postgres().await?;
	match pg_server.create_database(APP_NAME).await {
		Ok(_) => {}
		Err(err) => {
			error!("{:?}",err);
		}
	}
	let pg_uri = pg_server.full_db_uri(APP_NAME);
	let pg = Arc::new(db::setup::get_connection(pg_uri.as_str()).await?);
	info!("postgres uri {}",pg_uri);
	// Tracing
	// -------
	crate::logger::init(&settings.environment, &settings.log_path, &settings.log_file)?;

	// CORS
	// ----
	let cors = crate::util::cors(&settings);

	// Layers
	// ------
	let layers = ServiceBuilder::new()
		.set_x_request_id(MakeRequestUuid)
		.layer(crate::layers::logger::LoggerLayer)
		.layer(HandleErrorLayer::new(handlers::timeout_error))
		.timeout(Duration::from_secs(settings.request_timeout))
		.propagate_x_request_id();

	let pg_server_locked = Arc::new(Mutex::new(pg_server));
	let state = SharedState::new(State::init(settings.clone(), mem_db, Arc::clone(&pg_server_locked), pg));
	// Routing - API
	// -------------
	let mut app = Router::new()
		.nest("/", routes::api(state.clone()).layer(cors));

	// Prometheus metrics
	// ------------------
	if settings.prometheus_metrics_enabled {
		let handle = PrometheusMetric::get_handle()?;
		app = app
			.nest(
				"/metrics",
				Router::new().route(
					"/",
					get(move || ready(handle.render())).layer(BasicAuthLayer::new(
						&settings.basic_user,
						&settings.basic_pw,
					)),
				),
			)
			.route_layer(middleware::from_fn(PrometheusMetric::get_layer));
	}

	// Rate limiter
	// ------------
	if settings.limiter_enabled {
		app = app
			.layer(RateLimiterLayer::new(
				state.clone(),
				settings.limiter_requests_by_second.to_owned(),
				settings.limiter_expire_in_seconds.to_owned(),
				settings.limiter_white_list.clone(),
			));
	}

	app = app
		.fallback_service(ServeDir::new("templates/html").append_index_html_on_directories(true)) // FIXME: static_file_error not work this Axum 0.6.9!
		.layer(middleware::from_fn(crate::util::override_http_errors))
		.layer(layers);
	let app = app.with_state(state);

	// Start server
	// ------------
	let addr = format!("{}:{}", settings.server_url, settings.server_port);
	info!("starting server on {}...", &addr);

	match init_ssl_certs(settings.tls_policy.as_str()) {
		Ok(_) => {}
		Err(err) => {
			error!("SSL init error - {:?}",err);
		}
	}

	if &settings.environment == "development" || settings.tls_policy.eq("none") {
		let server =
			axum::Server::bind(&addr.parse()?)
				.serve(app.into_make_service_with_connect_info::<SocketAddr>());
		Ok(server.with_graceful_shutdown(shutdown_signal(Arc::clone(&pg_server_locked))).await?)

	} else {
		let tls_config = axum_server::tls_rustls::RustlsConfig::from_pem_file(
			PathBuf::from(&settings.tls_cert_path),
			PathBuf::from(&settings.tls_key_path),
		).await?;
		let gch = axum_server::Handle::new();

		tokio::spawn(graceful_shutdown(gch.clone(),Arc::clone(&pg_server_locked)));

		let server = axum_server::bind_rustls(addr.parse()?, tls_config)
			.handle(gch)
			.serve(app.into_make_service_with_connect_info::<SocketAddr>());
		Ok(server.await?)
	}
}