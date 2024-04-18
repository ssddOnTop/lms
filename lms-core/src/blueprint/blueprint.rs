use std::net::IpAddr;

use anyhow::anyhow;
use totp_rs::{Algorithm, Secret, TOTP};

use crate::authdb::auth_actors::Users;
use lms_auth::auth::AuthProvider;
use lms_auth::local_crypto::hash_256;

use crate::config::config_module::ConfigModule;

#[derive(Debug, Clone)]
pub struct Blueprint {
    pub server: Server,
    pub auth: AuthProvider,
    pub users: Users,
}

#[derive(Debug, Clone)]
pub struct Server {
    pub port: u16,
    pub hostname: IpAddr,
    pub token: TOTP,
}

impl TryFrom<ConfigModule> for Blueprint {
    type Error = anyhow::Error;

    fn try_from(config_module: ConfigModule) -> Result<Self, Self::Error> {
        let cfg = config_module.clone();
        let port = config_module.config.server.port.unwrap_or(19194);
        let hostname = config_module
            .config
            .server
            .host
            .unwrap_or("0.0.0.0".to_string());
        let hostname = if hostname.eq("localhost") {
            "0.0.0.0".parse()
        } else {
            hostname.parse()
        }?;

        let timeout_key = format!(
            "{}{}",
            config_module.config.auth.totp.totp_secret, config_module.config.auth.aes_key
        );
        let auth = AuthProvider::init(
            config_module.config.auth.auth_db_path,
            config_module.config.auth.totp.into_totp()?,
            hash_256(&config_module.config.auth.aes_key),
        )?;

        validate_config(cfg, &auth)?;

        let server = Server {
            port,
            hostname,
            token: TOTP::new(
                Algorithm::SHA1,
                8,
                1,
                config_module.config.server.request_timeout.unwrap_or(86400),
                Secret::Raw(timeout_key.as_bytes().to_vec()).to_bytes()?,
            )?,
        };

        Ok(Self {
            server,
            auth,
            users: config_module.users.unwrap(),
        })
    }
}

fn validate_config(config: ConfigModule, _auth_provider: &AuthProvider) -> anyhow::Result<()> {
    if config.users.is_none() {
        return Err(anyhow!(
            "Users not found in config, Initiate the server with `Init` Command"
        ));
    }

    if !config.auth.auth_db_path.starts_with("http") {
        // TODO FIXME
        // we can't perform std::fs here. lms-core must be platform independent.
        // proposal: create ConfigModule with resolve function which initiates file for auth db.
        // we still need to figure out how to handle the file path.

        /*let pb = PathBuf::from(&config.auth.auth_db_path);
        if !pb.exists() {
            return Err(anyhow!("Auth DB path is not a valid URL or file path"));
        } else {
            let users = auth_provider.decrypt_aes(std::fs::read_to_string(&config.auth.auth_db_path)?).map_err(|_| anyhow!("Failed to decrypt Auth DB with given key"))?;
            let _: Users = serde_json::from_str(&users).map_err(|_| anyhow!("Failed to parse Auth DB"))?;
        }*/
    } else {
        url::Url::parse(&config.auth.auth_db_path)
            .map_err(|_| anyhow!("Invalid URL for AuthDB"))?;
        /*
        TODO FIXME
        let req = reqwest::Client::new().post(url).body(reqwest::Body::default());
        let res = req.send().await?;
        if !res.status().is_success() {
            return Err(anyhow!("Failed to connect to Auth DB"));
        }*/
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
