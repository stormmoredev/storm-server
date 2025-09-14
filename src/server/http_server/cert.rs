use std::{error::Error, fs::File, io::BufReader, sync::Arc};

use rustls::crypto::aws_lc_rs::sign::any_supported_type;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use rustls::{
    server::ResolvesServerCertUsingSni,
    sign::CertifiedKey,
    ServerConfig,
};

use crate::conf::Conf;


fn load_certs(path: &str) -> Vec<CertificateDer<'static>> {
    let file = File::open(path).expect("Cannot open certificate file");
    let mut reader = BufReader::new(file);

    rustls_pemfile::certs(&mut reader)
        .collect::<Result<Vec<_>, _>>()
        .expect("Cannot read certs")
}

fn load_private_key(path: &str) -> PrivateKeyDer {
    let file = File::open(path).expect("Cannot open private key file");
    let mut reader = BufReader::new(file);
    let keys = rustls_pemfile::pkcs8_private_keys(&mut reader)
        .collect::<Result<Vec<_>, _>>()
        .expect("Cannot read private key");

    PrivateKeyDer::from(keys.into_iter().next().expect("No private key found"))
}

pub fn build_tls_config(configurations: &[Conf]) -> Result<ServerConfig, Box<dyn Error>> {
    let mut resolver = ResolvesServerCertUsingSni::new();

    for c in configurations {
        let cert_chain = load_certs(&c.https_pub_cert);
        let key_der = load_private_key(&c.https_private_key);

        // any_supported_type z aws-lc-rs
        let signing_key = any_supported_type(&key_der).expect("invalid private key");
        let ck = CertifiedKey::new(cert_chain, signing_key);

        // add() zwraca Result<(), rustls::Error>
        resolver.add(&c.domain, ck)?;
    }

    let config = ServerConfig::builder()
        .with_no_client_auth()
        .with_cert_resolver(Arc::new(resolver));

    Ok(config)
}