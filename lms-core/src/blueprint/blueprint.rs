use crate::config::Config;

use lms_auth::auth::AuthProvider;
use lms_auth::local_crypto::hash_128;
use std::net::IpAddr;
use totp_rs::{Secret, TOTP};

#[derive(Debug, Clone)]
pub struct Blueprint {
    pub port: u16,
    pub hostname: IpAddr,
    pub auth: AuthProvider,
}

impl TryFrom<Config> for Blueprint {
    type Error = anyhow::Error;

    fn try_from(config: Config) -> Result<Self, Self::Error> {
        validate_config(&config)?;
        let port = config.server.port.unwrap_or(19194);
        let hostname = config.server.host.unwrap_or("0.0.0.0".to_string());
        let hostname = if hostname.eq("localhost") {
            "0.0.0.0".parse()
        } else {
            hostname.parse()
        }?;

        let totp = TOTP::new(
            config.auth.totp.algo.unwrap_or_default().into_totp(),
            config.auth.totp.digits.unwrap_or(6),
            1,
            config.auth.totp.period.unwrap_or(30),
            Secret::Raw(config.auth.totp.totp_secret.as_bytes().to_vec()).to_bytes()?,
        )?;
        let auth = AuthProvider::init(
            config.auth.auth_url,
            totp,
            hash_128(config.auth.aes_key)[..32].to_string(),
        )?;
        Ok(Self {
            port,
            hostname,
            auth,
        })
    }
}

fn validate_config(config: &Config) -> anyhow::Result<()> {
    if url::Url::parse(&config.auth.auth_url).is_err() || !config.auth.auth_url.starts_with("http")
    {
        return Err(anyhow::anyhow!("auth_url is required"));
    }
    if config.auth.aes_key.is_empty() || config.auth.aes_key.len() < 8 {
        return Err(anyhow::anyhow!(
            "aes_key is required and must be 8 characters long"
        ));
    }
    if config.auth.totp.totp_secret.is_empty() || config.auth.totp.totp_secret.len() < 8 {
        return Err(anyhow::anyhow!(
            "totp_key is required and must be at least 8 bytes long"
        ));
    }
    Ok(())
}
