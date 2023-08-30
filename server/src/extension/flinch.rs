use std::sync::Arc;
use async_trait::async_trait;
use flinch::database::Database;
use flinch::doc::QueryBased;
use flinch::extension::{FlinchDbHelper, JsonMapExt};
use crate::GENERAL_BUCKET;
use crate::layers::jwt::claims::Authenticated;

#[async_trait]
pub trait FlinchHelper {
	async fn get_user(&self, token: &str) -> Option<Authenticated>;
}

#[async_trait]
impl FlinchHelper for Arc<Database<QueryBased>> {
	async fn get_user(&self, token: &str) -> Option<Authenticated> {
		let creds = self.get_object(GENERAL_BUCKET, token);
		match creds.keys().len() > 0 {
			true => {
				let username = creds.get_str("username");
				let token = creds.get_str("token");
				let authenticated = Authenticated {
					username,
					token
				};
				Some(authenticated)
			}
			false => None,
		}
	}
}