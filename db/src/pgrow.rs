use sqlx::postgres::{PgRow, PgValueRef, Postgres};
use sqlx::{Column, Decode, Row, TypeInfo, ValueRef};
use serde_json::{Value};

use serde::{Serialize, Serializer};
use serde::ser::{SerializeMap, SerializeSeq};

pub fn read_header(row: &PgRow) -> Vec<String> {
	let columns = row.columns();
	let mut headers = vec![];
	for c in columns {
		headers.push(format!("{}",c.name()));
	}
	headers
}

pub fn read_row(row: &PgRow) -> Vec<Value> {
	let columns = row.columns();
	let mut result: Vec<Value> = Vec::with_capacity(columns.len());
	for c in columns {
		let value = row.try_get_raw(c.ordinal()).unwrap();
		let value = SerPgValueRef(value);
		let value = serde_json::to_value(&value).unwrap();
		result.push(value);
	}
	result
}

/// Can be used with serialize_with
pub fn serialize_pgvalueref<S>(value: &PgValueRef, s: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
{
	if value.is_null() {
		return s.serialize_none();
	}
	let value = value.clone();
	let info = value.type_info();
	let name = info.name();
	match name {
		"BOOL" => {
			let v: bool = Decode::<Postgres>::decode(value).unwrap();
			s.serialize_bool(v)
		}
		"INT2" => {
			let v: i16 = Decode::<Postgres>::decode(value).unwrap();
			s.serialize_i16(v)
		}
		"INT4" => {
			let v: i32 = Decode::<Postgres>::decode(value).unwrap();
			s.serialize_i32(v)
		}
		"INT8" => {
			let v: i64 = Decode::<Postgres>::decode(value).unwrap();
			s.serialize_i64(v)
		}
		"FLOAT4" => {
			let v: f32 = Decode::<Postgres>::decode(value).unwrap();
			s.serialize_f32(v)
		}
		"FLOAT8" => {
			let v: f64 = Decode::<Postgres>::decode(value).unwrap();
			s.serialize_f64(v)
		}
		#[cfg(feature = "decimal")]
		"NUMERIC" => {
			let v: sqlx::types::Decimal = Decode::<Postgres>::decode(value).unwrap();
			s.serialize_str(&v.to_string())
		}
		"CHAR" | "VARCHAR" | "TEXT" | "\"CHAR\"" => {
			let v: String = Decode::<Postgres>::decode(value).unwrap();
			s.serialize_str(&v)
		}
		"BYTEA" => {
			let v: Vec<u8> = Decode::<Postgres>::decode(value).unwrap();
			s.serialize_some(&v)
		}
		"JSON" | "JSONB" => {
			let v: Value = Decode::<Postgres>::decode(value).unwrap();
			s.serialize_some(&v)
		}
		"TIMESTAMP" => {
			let v: sqlx::types::chrono::NaiveDateTime = Decode::<Postgres>::decode(value).unwrap();
			let v = v.format("%Y-%m-%dT%H:%M:%S.%f").to_string();
			s.serialize_str(&v)
		}
		"TIMESTAMPTZ" => {
			use sqlx::types::chrono;
			let v: chrono::DateTime::<chrono::Utc> = Decode::<Postgres>::decode(value).unwrap();
			s.serialize_str(&v.to_rfc3339())
		}
		"DATE" => {
			use sqlx::types::chrono;
			let v: chrono::NaiveDate = Decode::<Postgres>::decode(value).unwrap();
			s.serialize_str(&v.to_string())
		}
		"TIME" => {
			use sqlx::types::chrono;
			let v: chrono::NaiveTime = Decode::<Postgres>::decode(value).unwrap();
			s.serialize_str(&v.to_string())
		}
		"UUID" => {
			let v: String = Decode::<Postgres>::decode(value).unwrap();
			s.serialize_str(&v)
		}
		_ => {
			let v: String = Decode::<Postgres>::decode(value).unwrap();
			s.serialize_str(&v)
		}
	}
}

/// Can be used with serialize_with
pub fn serialize_pgrow_as_vec<S>(x: &PgRow, s: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
{
	let cols = x.columns();
	let mut seq = s.serialize_seq(Some(cols.len()))?;
	for c in cols {
		let c: PgValueRef = x.try_get_raw(c.ordinal()).unwrap();
		let c = SerPgValueRef(c);
		seq.serialize_element(&c)?;
	}
	seq.end()
}

/// Can be used with serialize_with
pub fn serialize_pgrow_as_map<S>(x: &PgRow, s: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
{
	let cols = x.columns();
	let mut map = s.serialize_map(Some(cols.len()))?;
	for col in cols {
		let c: PgValueRef = x.try_get_raw(col.ordinal()).unwrap();
		let c = SerPgValueRef(c);
		map.serialize_entry(col.name(), &c)?;
	}
	map.end()
}

/// SerVecPgRow::from(pg_row) will make your row serialize as a vector.
#[derive(Serialize)]
pub struct SPgRowVec(
	#[serde(serialize_with = "serialize_pgrow_as_vec")]
	PgRow
);

/// SerMapPgRow::from(pg_row) will make your row serialize as a map.
/// If you have multiple columns with the same name, the last one will win.
#[derive(Serialize)]
pub struct SPgRowMap(
	#[serde(serialize_with = "serialize_pgrow_as_map")]
	PgRow
);

impl From<PgRow> for SPgRowMap {
	fn from(row: PgRow) -> Self {
		SPgRowMap(row)
	}
}

impl std::ops::Deref for SPgRowMap {
	type Target = PgRow;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl std::ops::DerefMut for SPgRowMap {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.0
	}
}

impl Into<PgRow> for SPgRowMap {
	fn into(self) -> PgRow {
		self.0
	}
}

/// SerPgValueRef::from(pg_value_ref) will make your value serialize as its closest serde type.
#[derive(Serialize)]
pub struct SerPgValueRef<'r>(
	#[serde(serialize_with = "serialize_pgvalueref")]
	PgValueRef<'r>,
);

impl From<PgRow> for SPgRowVec {
	fn from(row: PgRow) -> Self {
		SPgRowVec(row)
	}
}

impl std::ops::Deref for SPgRowVec {
	type Target = PgRow;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl std::ops::DerefMut for SPgRowVec {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.0
	}
}

impl Into<PgRow> for SPgRowVec {
	fn into(self) -> PgRow {
		self.0
	}
}