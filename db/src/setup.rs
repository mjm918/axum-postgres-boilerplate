pub use pg_embed::postgres::{PgEmbed, PgSettings};
use pg_embed::pg_enums::{PgAuthMethod, PgServerStatus};
use pg_embed::pg_fetch::{PgFetchSettings, PG_V15};
use std::time::Duration;
use std::path::{Path};
use sqlx::{Pool, Postgres};
use sqlx::postgres::PgPoolOptions;
use utility::errors::AppResult;
use utility::killport::KillPortSignalOptions;

pub type PgServer = PgEmbed;
pub type PgDb = Pool<Postgres>;
pub type PgResultSet = Vec<serde_json::Value>;

pub async fn install_postgres() -> AppResult<PgServer> {
	let config = utility::env::Variables::from_env()?;
	let database_dir = Path::new(config.db_path.as_str());

	let pg_settings = PgSettings {
		database_dir: database_dir.to_path_buf(),
		port: config.db_port.clone(),
		user: config.db_user,
		password: config.db_pw,
		auth_method: PgAuthMethod::Plain,
		persistent: true,
		timeout: Some(Duration::from_secs(config.db_timeout)),
		migration_dir: None,
	};

	let fetch_settings = PgFetchSettings{
		version: PG_V15,
		..Default::default()
	};

	let mut pg = PgServer::new(pg_settings, fetch_settings).await?;
	let status = *pg.server_status.lock().await;
	tracing::trace!("pgServerStatus - {status:?}");

	let need_setup = match status {
		PgServerStatus::Uninitialized => true,
		PgServerStatus::Failure => true,
		_ => false
	};
	tracing::trace!("pg need setup ? {}",&need_setup);
	if need_setup {
		pg.setup().await?;
	}

	let need_startup = match status {
		PgServerStatus::Uninitialized => true,
		PgServerStatus::Initializing => false,
		PgServerStatus::Initialized => false,
		PgServerStatus::Starting => false,
		PgServerStatus::Started => false,
		PgServerStatus::Stopping => true,
		PgServerStatus::Stopped => true,
		PgServerStatus::Failure => true,
	};
	tracing::trace!("pg need startup ? {}",&need_startup);
	if need_startup {
		match utility::killport::kill_processes_by_port(config.db_port, KillPortSignalOptions::SIGKILL) {
			Ok(_) => {}
			Err(err) => tracing::error!("{:?}",err)
		};
		std::thread::sleep(Duration::from_secs(3));
		pg.start_db().await?;
	}
	Ok(pg)
}

pub async fn create_db(pg: &PgServer, name: &str) -> AppResult<()> {
	match pg.database_exists(name).await {
		Ok(created) => {
			if !created {
				pg.create_database(name).await?;
			}
		}
		Err(_) => {}
	};
	Ok(())
}

pub async fn get_connection(uri: &str) -> AppResult<PgDb> {
	let db = PgPoolOptions::new()
		.max_connections(50)
		.min_connections(10)
		.connect(uri).await?;
	Ok(db)
}

#[cfg(test)]
mod tests {
	use sqlx::Executor;
	use crate::ops;
	use super::*;
	#[tokio::test]
	async fn test() {
		let pgs = install_postgres().await;
		assert!(pgs.is_ok(),"{:?}",pgs.err());

		let pgs = pgs.unwrap();
		let created = create_db(&pgs, "test").await;

		assert!(created.is_ok(),"{:?}",created.err());

		let uri = pgs.full_db_uri("test");
		let conn = get_connection(uri.as_str()).await;
		assert!(conn.is_ok(),"{:?}",conn.err());

		let pg = conn.unwrap();
		let res = pg.execute("SELECT 1").await;
		assert!(res.is_ok(),"{:?}",res.err());

		let sd = ops::show_databases(&pg).await;
		assert!(sd.is_ok(),"{:?}",sd.err());

		let rs = sd.unwrap();
		println!("{rs:?}");

		let td = ops::show_tables(&pg).await;
		assert!(td.is_ok(),"{:?}",td.err());
	}
}