use axum::Router;
use axum::routing::{get};
use crate::{controller, layers};
use crate::state::SharedState;

/// Return API routes list
pub fn api(state: SharedState) -> Router<SharedState> {
	Router::new()
		.route("/health-check", get(controller::web::health_check))
		.route("/ok", get(controller::web::say_ok))
		// Protected routes
		.nest("/", protected().layer(layers::jwt::JwtLayer { state }))
}

fn protected() -> Router<SharedState> {
	Router::new()
}