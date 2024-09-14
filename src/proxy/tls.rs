use hyper::upgrade::Upgraded;
use hyper_util::rt::TokioIo;
use rcgen::{
    date_time_ymd, BasicConstraints, Certificate, CertificateParams, Ia5String, IsCa, KeyPair,
    SanType,
};
use rustls::pki_types::{PrivateKeyDer, PrivatePkcs8KeyDer};
use std::error::Error;
use std::fs;
use std::path::Path;
use std::sync::{Arc, LazyLock};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_rustls::TlsAcceptor;

const CA_COMMON_NAME: &str = "yatangaki_ca";
pub static TLS_HANDLER: LazyLock<TlsHandler> = LazyLock::new(|| TlsHandler::init().unwrap());

pub struct TlsHandler {
    ca: Certificate,
    ca_private_key: KeyPair,
}

impl TlsHandler {
    pub fn generate() -> Result<Self, Box<dyn Error>> {
        let ca_private_key = KeyPair::generate()?;
        let mut params = CertificateParams::default();
        params.not_before = date_time_ymd(2023, 1, 1);
        params.not_after = date_time_ymd(4096, 1, 1);
        params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        params
            .distinguished_name
            .push(rcgen::DnType::CommonName, CA_COMMON_NAME);

        let ca = params.self_signed(&ca_private_key)?;
        Ok(Self { ca, ca_private_key })
    }

    pub fn parse_pem(pem_certificate: &str, pem_private_key: &str) -> Result<Self, Box<dyn Error>> {
        let params = CertificateParams::from_ca_cert_pem(pem_certificate)?;
        let ca_private_key = KeyPair::from_pem(pem_private_key)?;
        let ca = params.self_signed(&ca_private_key)?;

        Ok(Self { ca, ca_private_key })
    }

    pub async fn upgrade_tls(
        &self,
        host: &str,
        stream: Upgraded,
    ) -> Result<impl AsyncRead + AsyncWrite, Box<dyn Error>> {
        let mut params = CertificateParams::default();
        params.is_ca = rcgen::IsCa::NoCa;
        params.not_before = date_time_ymd(2023, 1, 1);
        params.not_after = date_time_ymd(2048, 1, 1);

        //  TODO: Should also handle IP addresses in SanType
        params.subject_alt_names = vec![SanType::DnsName(Ia5String::try_from(host).unwrap())];
        params
            .distinguished_name
            .push(rcgen::DnType::CommonName, host);

        let leaf_private_key = &self.ca_private_key;
        let certificate = params.signed_by(leaf_private_key, &self.ca, &self.ca_private_key)?;

        let der_key = PrivatePkcs8KeyDer::from(leaf_private_key.serialize_der());
        let tls_config = rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(
                vec![certificate.der().to_owned()],
                PrivateKeyDer::Pkcs8(der_key),
            )
            .unwrap();

        Ok(TlsAcceptor::from(Arc::new(tls_config))
            .accept(TokioIo::new(stream))
            .await?)
    }

    pub fn init() -> Result<Self, Box<dyn Error>> {
        let ca_path = format!("{}/.yatangaki/ca.pem", env!("HOME"));
        let ca_key_path = format!("{}/.yatangaki/ca_key.pem", env!("HOME"));

        if !Path::new(&ca_path).exists() || !Path::new(&ca_key_path).exists() {
            match Self::generate() {
                Ok(tls) => {
                    fs::write(ca_path, tls.ca.pem())?;
                    fs::write(ca_key_path, tls.ca_private_key.serialize_pem())?;
                    Ok(tls)
                }
                Err(e) => Err(e),
            }
        } else {
            let ca_pem = fs::read_to_string(ca_path)?;
            let ca_key_pem = fs::read_to_string(ca_key_path)?;

            Ok(Self::parse_pem(&ca_pem, &ca_key_pem)?)
        }
    }
}
