use utility::errors::AppResult;
use crate::extension::res::ResultSet;
use crate::setup::{PgDb, PgResultSet};

pub async fn show_databases(pg: &PgDb) -> AppResult<PgResultSet> {
	let rows = sqlx::query("SELECT datname as database FROM pg_database WHERE datistemplate = false;")
		.fetch_all(pg)
		.await?
		.result_array();
	Ok(rows)
}

pub async fn show_tables(pg: &PgDb) -> AppResult<PgResultSet> {
	let rows = sqlx::query("SELECT * FROM pg_catalog.pg_tables WHERE schemaname='public';")
		.fetch_all(pg)
		.await?
		.result_array();
	Ok(rows)
}

pub async fn exec_any_sql(pg: &PgDb, sql: &str) -> AppResult<PgResultSet> {
	tracing::warn!("Executing SQL - {}",sql);
	let rows = sqlx::query(sql)
		.fetch_all(pg)
		.await?
		.result_array();
	Ok(rows)
}