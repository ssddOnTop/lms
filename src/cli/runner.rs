use crate::cli::commands::{Cli, Command};
use crate::cli::{self, rt};
use clap::Parser;
use lms_auth::local_crypto::hash_256;
use lms_core::app_ctx::AppContext;
use lms_core::authdb::auth_actors::User;
use lms_core::authdb::auth_db::user_entry;
use lms_core::blueprint::Blueprint;
use lms_core::config::reader::ConfigReader;
use lms_core::runtime::TargetRuntime;

pub async fn fork_run() -> anyhow::Result<()> {
    logger_init();
    let cli = Cli::parse();
    let runtime = rt::init();

    run(cli, runtime).await
}

async fn run(cli: Cli, runtime: TargetRuntime) -> anyhow::Result<()> {
    let config_reader = ConfigReader::init(runtime.clone());
    match cli.command {
        Command::Start { config_path } => {
            let config = config_reader.read(config_path).await?;
            let server = cli::server::Server::new(config);
            server.fork_start().await?;
        }
        Command::Check { config_path } => {
            let config = config_reader.read(config_path).await?;
            let blueprint = Blueprint::try_from(config);
            match blueprint {
                Ok(_) => {
                    log::info!("Config is valid");
                }
                Err(e) => {
                    log::error!("Invalid config: {}", e)
                }
            }
        }
        Command::Init {
            config_path,
            username,
            name,
            password,
            authority,
            print,
            batch,
        } => {
            let config_module = config_reader.read(config_path).await?;
            let blueprint = Blueprint::try_from(config_module)?;
            let mut users = blueprint.extensions.users.clone();
            let app_context = AppContext { blueprint, runtime };

            users.insert(User {
                username,
                name,
                password: hash_256(password),
                authority,
                batch,
            });

            if print.unwrap_or_default() {
                display(serde_json::to_string_pretty(&users).unwrap());
            }

            // TODO: Fix user_entry to take single user
            user_entry(&app_context, users)
                .await
                .map_err(|e| anyhow::anyhow!("Unable to create user with error: {}", e))?;
        }
    }
    Ok(())
}

fn display<T: AsRef<str>>(content: T) {
    println!("{}", content.as_ref());
}

fn logger_init() {
    // set the log level
    const LONG_ENV_FILTER_VAR_NAME: &str = "LMS_LOG_LEVEL";
    // Select which env variable to use for the log level filter. This is because filter_or doesn't allow picking between multiple env_var for the filter value
    let filter_env_name =
        std::env::var(LONG_ENV_FILTER_VAR_NAME).unwrap_or(LONG_ENV_FILTER_VAR_NAME.to_string());

    // use the log level from the env if there is one, otherwise use the default.
    let env = env_logger::Env::new().filter_or(filter_env_name, "info");

    env_logger::Builder::from_env(env).init();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_run_check() {
        logger_init();

        let config_path = PathBuf::from("examples/config.json")
            .to_str()
            .unwrap()
            .to_string();
        let cli = Cli {
            command: Command::Check { config_path },
        };
        let runtime = rt::init();
        assert!(run(cli, runtime).await.is_ok())
    }
}
