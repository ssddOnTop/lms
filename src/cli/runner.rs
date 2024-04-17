use crate::cli::commands::{Cli, Command};
use crate::cli::{self, rt};
use clap::Parser;
use lms_core::blueprint::Blueprint;
use lms_core::config::reader::ConfigReader;
pub async fn run() -> anyhow::Result<()> {
    logger_init();
    let cli = Cli::parse();
    let runtime = rt::init();

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
    }
    Ok(())
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
