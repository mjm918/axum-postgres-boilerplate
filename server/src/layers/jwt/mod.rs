pub mod claims;

use std::sync::Arc;
use std::task::{Context, Poll};

use axum::body::{Body, boxed, Full};
use axum::http::{Request, StatusCode};
use axum::response::Response;
use futures::future::BoxFuture;
use tower::{Layer, Service};
use crate::extension::flinch::FlinchHelper;
use crate::layers::jwt::claims::Claims;
use crate::state::SharedState;
use crate::util::body_from_parts;

#[derive(Clone)]
pub struct JwtLayer {
	pub state: SharedState,
}

impl<S> Layer<S> for JwtLayer {
	type Service = JwtMiddleware<S>;

	fn layer(&self, inner: S) -> Self::Service {
		JwtMiddleware {
			inner,
			state: self.state.clone(),
		}
	}
}

#[derive(Clone)]
pub struct JwtMiddleware<S> {
	inner: S,
	state: SharedState,
}

impl<S> Service<Request<Body>> for JwtMiddleware<S>
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

	fn call(&mut self, mut request: Request<Body>) -> Self::Future {
		let is_authorized =
			match Claims::extract_from_request(request.headers(), &self.state.config.jwt_decoding_key.clone()) {
				Some(data) => {
					let is_ok = data.is_ok();
					if is_ok {
						let parsed_token = data.unwrap();
						let parsed_token = parsed_token.1;
						let flinch = Arc::clone(&self.state.flinch);
						let authenticated = tokio::task::block_in_place(move || {
							tokio::runtime::Handle::current().block_on(async move {
								flinch.get_user(parsed_token.as_str()).await
							})
						});
						if let Some(user) = authenticated {
							request.extensions_mut().insert(user);
						}
					}
					is_ok
				}
				_ => false,
			};

		let future = self.inner.call(request);
		Box::pin(async move {
			let mut response = Response::default();
			response = match is_authorized {
				true => {
					let res: Response = future.await?;
					res
				}
				false => {
					let (mut parts, _body) = response.into_parts();
					let msg = body_from_parts(&mut parts, StatusCode::UNAUTHORIZED, "Unauthorized", None);
					Response::from_parts(parts, boxed(Full::from(msg)))
				}
			};

			Ok(response)
		})
	}
}