use std::sync::{Arc, Mutex};
use flinch::database::Database;
use flinch::doc::QueryBased;
use db::setup::{PgDb, PgServer};
use utility::env::Variables;
use crate::util::ConfigState;

pub type SharedState = Arc<State>;

pub struct State {
	pub env: Variables,
	pub flinch: Arc<Database<QueryBased>>,
	pub config: ConfigState,
	pub pg_server: Arc<Mutex<PgServer>>,
	pub pg: Arc<PgDb>
}

impl State {
	pub fn init(env: Variables, flinch: Arc<Database<QueryBased>>, pg_server: Arc<Mutex<PgServer>>, pg: Arc<PgDb>) -> Self {
		Self { env: env.clone(), config: ConfigState::from(env), flinch, pg_server, pg }
	}
}