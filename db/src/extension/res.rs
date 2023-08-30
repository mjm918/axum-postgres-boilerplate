use serde_json::Value;
use sqlx::postgres::PgRow;
use crate::pgrow::SPgRowMap;

pub trait ResultSet {
	fn result_array(self) -> Vec<Value>;
}

impl ResultSet for Vec<PgRow> {
	fn result_array(self) -> Vec<Value> {
		let mut q = vec![];
		for row in self {
			let v = SPgRowMap::from(row);
			match serde_json::to_value(v) {
				Ok(value) => {
					q.push(value);
				}
				Err(err) => {
					tracing::error!("PgExtension: serde_json {:?}",err);
				}
			}
		}
		q
	}
}