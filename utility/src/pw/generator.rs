pub fn generate_password() -> String {
	let opt = passwords::PasswordGenerator {
		length: 12,
		numbers: true,
		lowercase_letters: true,
		uppercase_letters: true,
		symbols: false,
		spaces: false,
		exclude_similar_characters: true,
		strict: true,
	};
	let pw = opt.generate_one().unwrap();
	format!("{}$", pw)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test() {
		println!("{}", generate_password());
	}
}