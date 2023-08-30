use lazy_static::lazy_static;

mod extractor;
mod certs;
mod validator;
mod util;
mod server;
mod state;
mod layers;
mod handlers;
mod setup;
mod logger;
mod routes;
mod controller;
pub mod cli;
mod extension;

pub const APP_NAME: &str = "qaswa";
pub const RATE_LIMITER_BUCKET: &str = "rate-limiter-rate";
pub const GENERAL_BUCKET: &str = "general-bucket";
pub const SECONDS_DURATION_BUCKETS: &[f64; 11] = &[0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0];

lazy_static! {
    pub static ref TEMPLATES: Result<tera::Tera, tera::Error> = {
        let mut tera = tera::Tera::new("templates/**/*")?;
        tera.autoescape_on(vec![".html", ".txt"]);
        Ok(tera)
    };
}
