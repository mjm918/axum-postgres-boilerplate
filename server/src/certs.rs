#![allow(clippy::complexity, clippy::style, clippy::pedantic)]

use std::convert::TryInto;
use std::fs;

use acme_lib::{Directory, DirectoryUrl};
use acme_lib::create_p384_key;
use acme_lib::persist::FilePersist;
use rcgen::{Certificate, CertificateParams, date_time_ymd, DistinguishedName, DnType};

pub fn init_ssl_certs(policy: &str) -> Result<(), Box<dyn std::error::Error>> {
	match policy {
		"acme" => acme_setup(),
		"rcgen" => rcgen_setup(),
		_ => domain_existing()
	}
}

#[allow(unused)]
fn domain_existing() -> Result<(), Box<dyn std::error::Error>> {
	Ok(())
}

#[allow(unused)]
fn acme_setup() -> Result<(), Box<dyn std::error::Error>> {
	let url = DirectoryUrl::LetsEncrypt;

	let persist = FilePersist::new(".");
	let dir = Directory::from_url(persist, url)?;

	let acc = dir.account("zack@eztech.com.my")?;
	let mut ord_new = acc.new_order("easysalesperson.com:8087", &[])?;

	let ord_csr = loop {
		if let Some(ord_csr) = ord_new.confirm_validations() {
			break ord_csr;
		}

		let auths = ord_new.authorizations()?;

		let chall = auths[0].http_challenge();

		let token = chall.http_token();
		let dir_path = ".well-known/acme-challenge/";
		fs::create_dir_all(dir_path)?;

		let path = format!(".well-known/acme-challenge/{}", token);
		fs::File::create(path)?;

		let proof = chall.http_proof();
		chall.validate(5000)?;

		ord_new.refresh()?;
	};

	let pkey_pri = create_p384_key();
	let ord_cert =
		ord_csr.finalize_pkey(pkey_pri, 5000)?;

	let cert = ord_cert.download_and_save_cert()?;
	let pkey = cert.private_key();
	let crt = cert.certificate();

	let _ = fs::remove_dir_all("certs/");

	fs::create_dir_all("certs/")?;

	fs::File::create("certs/cert.pem")?;
	fs::File::create("certs/key.pem")?;

	fs::write("certs/cert.pem", &crt.as_bytes())?;
	fs::write("certs/key.pem", &pkey.as_bytes())?;

	Ok(())
}

fn rcgen_setup() -> Result<(), Box<dyn std::error::Error>> {
	let mut din = DistinguishedName::new();
	din.push(DnType::CommonName, "easysalesperson.com");
	din.push(DnType::CountryName, "Malaysia");
	din.push(DnType::OrganizationName, "EasyTech International Sdn. Bhd.");
	din.push(DnType::StateOrProvinceName, "Selangor");

	let mut params: CertificateParams = Default::default();
	params.not_before = date_time_ymd(2021, 05, 19);
	params.not_after = date_time_ymd(2023, 12, 31);
	params.distinguished_name = din;

	params.alg = &rcgen::PKCS_RSA_SHA256;

	let pkey: openssl::pkey::PKey<_> = openssl::rsa::Rsa::generate(2048)?.try_into()?;
	let key_pair_pem = String::from_utf8(pkey.private_key_to_pem_pkcs8()?)?;
	let key_pair = rcgen::KeyPair::from_pem(&key_pair_pem)?;
	params.key_pair = Some(key_pair);

	let cert = Certificate::from_params(params)?;
	let pem_serialized = cert.serialize_pem()?;

	let _ = fs::remove_dir_all("certs/");

	fs::create_dir_all("certs/")?;

	fs::File::create("certs/cert.pem")?;
	fs::File::create("certs/key.pem")?;

	fs::write("certs/cert.pem", &pem_serialized.as_bytes())?;
	fs::write("certs/key.pem", &cert.serialize_private_key_pem().as_bytes())?;

	Ok(())
}

#[cfg(test)]
mod tests {
	use crate::certs::init_ssl_certs;

	#[test]
	fn test() {
		let ssl = init_ssl_certs("native");
		assert!(ssl.is_ok(), "{:?}", ssl.err());
	}
}