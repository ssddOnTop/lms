use std::net::IpAddr;

use anyhow::anyhow;
use totp_rs::{Algorithm, Secret, TOTP};

use crate::authdb::auth_actors::{Authority, Users};
use crate::config;
use lms_auth::auth::AuthProvider;

use crate::config::config_module::ConfigModule;

#[derive(Debug, Clone)]
pub struct Blueprint {
    pub server: Server,
    pub extensions: Extensions,
}

#[derive(Debug, Clone)]
pub struct Extensions {
    pub users: Users,
    pub auth: AuthProvider,
}

#[derive(Debug, Clone)]
pub struct Server {
    pub port: u16,
    pub hostname: IpAddr,
    pub token: TOTP,
}

impl TryFrom<config::Server> for Server {
    type Error = anyhow::Error;

    fn try_from(server: config::Server) -> Result<Self, Self::Error> {
        let hostname = server.host.unwrap_or("0.0.0.0".to_string());
        let hostname = if hostname.eq("localhost") {
            "0.0.0.0".parse()
        } else {
            hostname.parse()
        }?;

        let port = server.port.unwrap_or(19194);

        Ok(Server {
            port,
            hostname,
            token: TOTP::new(
                Algorithm::SHA1,
                8,
                1,
                server.request_timeout.unwrap_or(86400),
                Secret::Raw(server.timeout_key.unwrap().as_bytes().to_vec()).to_bytes()?,
            )?,
        })
    }
}

impl TryFrom<config::config_module::Extensions> for Extensions {
    type Error = anyhow::Error;

    fn try_from(ext: config::config_module::Extensions) -> Result<Self, Self::Error> {
        Ok(Self {
            users: ext
                .users
                .ok_or_else(|| anyhow!("Users not found in config"))?,
            auth: ext
                .auth
                .ok_or_else(|| anyhow!("Auth Provider not found in config"))?,
        })
    }
}

impl TryFrom<ConfigModule> for Blueprint {
    type Error = anyhow::Error;

    fn try_from(mut config_module: ConfigModule) -> Result<Self, Self::Error> {
        let cfg = config_module.clone();

        config_module.config.server.timeout_key =
            Some(config_module.config.server.timeout_key.unwrap_or(format!(
                "{}{}",
                config_module.config.auth.totp.totp_secret, config_module.config.auth.aes_key
            )));

        validate_config(cfg, &config_module.extensions.auth)?;

        let server = Server::try_from(config_module.config.server)?;

        Ok(Self {
            server,
            extensions: Extensions::try_from(config_module.extensions)?,
        })
    }
}

fn validate_config(
    config: ConfigModule,
    _auth_provider: &Option<AuthProvider>,
) -> anyhow::Result<()> {
    if _auth_provider.is_none() {
        return Err(anyhow!(
            "Auth Provider not found in config, Initiate the server with `Init` Command"
        ));
    }
    if config.extensions.users.is_none() {
        return Err(anyhow!(
            "Users not found in config, Initiate the server with `Init` Command"
        ));
    }

    if config.auth.auth_db_path.starts_with("http") {
        url::Url::parse(&config.auth.auth_db_path)
            .map_err(|_| anyhow!("Invalid URL for AuthDB"))?;
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
    let batches = &config.batches;
    for batch in batches {
        for course in batch.courses.iter() {
            if !config.courses.contains_key(course) {
                return Err(anyhow::anyhow!("Course {} not found in courses", course));
            }
        }
    }

    if let Some(users) = config.extensions.users.as_ref() {
        for user in users.get_all().values() {
            if let Some(batch) = user.batch.as_ref() {
                if !config.batches.iter().any(|b| b.id == *batch) {
                    return Err(anyhow::anyhow!(
                        "Invalid natch {} for user: {}",
                        batch,
                        user.name
                    ));
                }
            }
            if user.authority.eq(&Authority::Student) && user.batch.is_none() {
                return Err(anyhow::anyhow!("Batch not found for student: {:?}", user));
            }
        }
    }

    Ok(())
}
