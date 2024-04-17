use crate::config::Config;
use lms_auth::auth::AuthProvider;
use totp_rs::{Secret, TOTP};

pub struct Blueprint {
    pub port: u16,
    pub auth: AuthProvider,
}

impl TryFrom<Config> for Blueprint {
    type Error = anyhow::Error;

    fn try_from(config: Config) -> Result<Self, Self::Error> {
        validate_config(&config)?;
        let port = config.port.unwrap_or(19194);

        let totp = TOTP::new(
            config.auth.totp.algo.unwrap_or_default().into_totp(),
            config.auth.totp.digits.unwrap_or(6),
            1,
            config.auth.totp.period.unwrap_or(30),
            Secret::Raw(config.auth.totp.totp_secret_key.as_bytes().to_vec()).to_bytes()?,
        )?;

        let auth = AuthProvider::init(config.auth.auth_url, totp, config.auth.aes_key)?;
        Ok(Self { port, auth })
    }
}

fn validate_config(config: &Config) -> anyhow::Result<()> {
    if config.auth.auth_url.is_empty() {
        return Err(anyhow::anyhow!("auth_url is required"));
    }
    if config.auth.aes_key.is_empty() || config.auth.aes_key.len() != 16 {
        return Err(anyhow::anyhow!(
            "aes_key is required and must be 16 bytes long"
        ));
    }
    if config.auth.totp.totp_secret_key.is_empty() || config.auth.totp.totp_secret_key.len() < 16 {
        return Err(anyhow::anyhow!(
            "totp_key is required and must be at least 16 bytes long"
        ));
    }
    Ok(())
}
