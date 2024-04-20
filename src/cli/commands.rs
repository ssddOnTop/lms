use clap::{Parser, Subcommand};
use lms_core::authdb::auth_actors::Authority;

const VERSION: &str = match option_env!("APP_VERSION") {
    Some(version) => version,
    _ => "0.1.0-dev",
};
#[derive(Parser)]
#[command(name ="lms", version = VERSION)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Starts the SpacePls server on the configured port
    Start {
        /// Path for the configuration file or http(s) link to config file.
        #[arg(required = true)]
        config_path: String,
    },
    /// Checks the configuration file for errors
    Check {
        /// Path for the configuration file or http(s) link to config file.
        #[arg(required = true)]
        config_path: String,
    },
    /// Initializes the configuration file
    /// It helps in creating initial Admins, Faculties, and Students
    Init {
        /// Path for the configuration file or http(s) link to config file.
        #[arg(required = true)]
        config_path: String,
        #[arg(short, long)]
        username: String,
        #[arg(short, long)]
        name: String,
        #[arg(short, long)]
        password: String,
        #[arg(short, long)]
        authority: Authority,
        #[arg(short, long)]
        batch: Option<String>,

        #[arg(long)]
        print: Option<bool>,
    },
}
