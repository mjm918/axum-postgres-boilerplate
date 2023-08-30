use std::ops::Add;

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Password strength
pub enum PasswordStrength {
	Dangerous,
	// 0..40
	VeryWeak,
	// 40..60
	Weak,
	// 60..80
	Good,
	// 80..90
	Strong,
	// 90..95
	VeryStrong,
	// 95..99
	Invulnerable, // 99..100
}

const PASSWORD_STRENGTH_DANGEROUS: f64 = 0.0;
const PASSWORD_STRENGTH_VERY_WEAK: f64 = 40.0;
const PASSWORD_STRENGTH_WEAK: f64 = 60.0;
const PASSWORD_STRENGTH_GOOD: f64 = 80.0;
const PASSWORD_STRENGTH_STRONG: f64 = 90.0;
const PASSWORD_STRENGTH_VERY_STRONG: f64 = 95.0;
const PASSWORD_STRENGTH_INVULNERABLE: f64 = 99.0;

/// Used to test if a password is strong enought
pub struct PasswordScorer {}

impl PasswordScorer {
	/// Valid that a password is strong enough
	pub fn valid(password: &str, strength: PasswordStrength) -> bool {
		let score = passwords::scorer::score(&passwords::analyzer::analyze(password));
		match strength {
			PasswordStrength::Dangerous => PASSWORD_STRENGTH_DANGEROUS <= score,
			PasswordStrength::VeryWeak => PASSWORD_STRENGTH_VERY_WEAK <= score,
			PasswordStrength::Weak => PASSWORD_STRENGTH_WEAK <= score,
			PasswordStrength::Good => PASSWORD_STRENGTH_GOOD <= score,
			PasswordStrength::Strong => PASSWORD_STRENGTH_STRONG <= score,
			PasswordStrength::VeryStrong => PASSWORD_STRENGTH_VERY_STRONG <= score,
			PasswordStrength::Invulnerable => PASSWORD_STRENGTH_INVULNERABLE <= score,
		}
	}
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PasswordReset {
	#[serde(skip_serializing)]
	pub user_id: String,
	pub token: String,
	pub expired_at: DateTime<Utc>,
}

impl PasswordReset {
	/// Create a new password recovery
	pub fn new(user_id: String, expiration_duration: i64) -> Self {
		let now = Utc::now();

		Self {
			user_id,
			token: Uuid::new_v4().to_string(),
			expired_at: now.add(Duration::hours(expiration_duration)),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_passwords_score() {
		// Not valid
		assert!(!PasswordScorer::valid("", PasswordStrength::Strong));
		assert!(!PasswordScorer::valid("azerty", PasswordStrength::Strong));
		assert!(!PasswordScorer::valid("azerty", PasswordStrength::Strong));

		// Valid
		assert!(PasswordScorer::valid("", PasswordStrength::Dangerous));
		assert!(PasswordScorer::valid("azerty", PasswordStrength::Dangerous));
		assert!(PasswordScorer::valid("Wl6,Ak4;6a", PasswordStrength::Good));
		assert!(PasswordScorer::valid(
			"WlH5Y;8!fs81#6,Ak4;6a(HJ27hgh6g=1",
			PasswordStrength::Invulnerable,
		));
		assert!(PasswordScorer::valid("Wl6,Ak4;6a", PasswordStrength::Dangerous));
	}
}