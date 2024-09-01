use rcgen::{
    date_time_ymd, BasicConstraints, Certificate, CertificateParams, Ia5String, IsCa, KeyPair,
    SanType,
};
use rustls::pki_types::{PrivateKeyDer, PrivatePkcs8KeyDer};
use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::sync::{Arc, Mutex};
use tokio_rustls::TlsAcceptor;

#[derive(Clone)]
pub struct CertificateStore {
    inner: Arc<Mutex<InnerCertificateStore>>,
}

struct InnerCertificateStore {
    certificate_authority_keypair: KeyPair,
    certificate_authority: Certificate,
    entity_certificates: HashMap<String, TlsAcceptor>,
}

impl CertificateStore {
    pub fn save_certificate(&self) -> Result<(), Box<dyn Error>> {
        let inner = self.inner.lock().unwrap();
        let pem = inner.certificate_authority.pem();
        let ca_path = format!("{}/.yatangaki/ca.pem", env!("HOME"));
        let ca_key_path = format!("{}/.yatangaki/ca_key.pem", env!("HOME"));

        fs::write(ca_path, pem)?;
        fs::write(
            ca_key_path,
            inner.certificate_authority_keypair.serialize_pem(),
        )?;

        Ok(())
    }

    pub fn load_certificate() -> Result<Self, Box<dyn Error>> {
        let ca_path = format!("{}/.yatangaki/ca.pem", env!("HOME"));
        let ca_key_path = format!("{}/.yatangaki/ca_key.pem", env!("HOME"));

        let params = CertificateParams::from_ca_cert_pem(&fs::read_to_string(ca_path)?)?;
        let certificate_authority_keypair = KeyPair::from_pem(&fs::read_to_string(ca_key_path)?)?;
        let certificate_authority = params.self_signed(&certificate_authority_keypair)?;

        Ok(Self {
            inner: Arc::new(Mutex::new(InnerCertificateStore {
                certificate_authority_keypair,
                certificate_authority,
                entity_certificates: HashMap::default(),
            })),
        })
    }

    pub fn generate() -> Result<Self, rcgen::Error> {
        let certificate_authority_keypair = KeyPair::generate()?;
        let mut params = CertificateParams::default();
        params.not_before = date_time_ymd(2023, 1, 1);
        params.not_after = date_time_ymd(4096, 1, 1);
        params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        params
            .distinguished_name
            .push(rcgen::DnType::CommonName, "yatangaki_ca");

        let certificate_authority = params.self_signed(&certificate_authority_keypair)?;

        Ok(Self {
            inner: Arc::new(Mutex::new(InnerCertificateStore {
                certificate_authority_keypair,
                certificate_authority,
                entity_certificates: HashMap::default(),
            })),
        })
    }

    pub fn tls_acceptor(&mut self, host: &str) -> Result<TlsAcceptor, rcgen::Error> {
        let mut inner = self.inner.lock().unwrap();

        if !inner.entity_certificates.contains_key(host) {
            let san = SanType::DnsName(Ia5String::try_from(host).unwrap());
            let mut params = CertificateParams::default();
            params.is_ca = rcgen::IsCa::NoCa;
            params.not_before = date_time_ymd(2023, 1, 1);
            params.not_after = date_time_ymd(2048, 1, 1);

            //  TODO: Should also handle IP addresses in SanType
            params.subject_alt_names = vec![san];
            params
                .distinguished_name
                .push(rcgen::DnType::CommonName, host);

            let keys = KeyPair::generate()?;
            let certificate = params.signed_by(
                &keys,
                &inner.certificate_authority,
                &inner.certificate_authority_keypair,
            )?;

            let der_key = PrivatePkcs8KeyDer::from(keys.serialize_der());
            let tls_config = rustls::ServerConfig::builder()
                .with_no_client_auth()
                .with_single_cert(
                    vec![certificate.der().to_owned()],
                    PrivateKeyDer::Pkcs8(der_key),
                )
                .unwrap();

            let acceptor = TlsAcceptor::from(Arc::new(tls_config));
            inner.entity_certificates.insert(host.into(), acceptor);
        }

        Ok(inner.entity_certificates.get(host).cloned().unwrap())
    }
}
