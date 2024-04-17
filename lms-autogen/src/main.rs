use anyhow::{anyhow, Result};
use lms_core::config::Config;
use schemars::schema::RootSchema;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::process::exit;

static JSON_SCHEMA_FILE: &str = "../generated/.lmsrc.schema.json";

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    let arg = args.get(1);

    logger_init();

    if arg.is_none() {
        log::error!("An argument required, you can pass either `fix` or `check` argument");
        return;
    }
    match arg.unwrap().as_str() {
        "fix" => {
            let result = mode_fix().await;
            if let Err(e) = result {
                log::error!("{}", e);
                exit(1);
            }
        }
        "check" => {
            let result = mode_check().await;
            if let Err(e) = result {
                log::error!("{}", e);
                exit(1);
            }
        }
        &_ => {
            log::error!("Unknown argument, you can pass either `fix` or `check` argument");
            return;
        }
    }
}

fn logger_init() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
}

fn get_file_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(JSON_SCHEMA_FILE)
}

async fn get_updated_json() -> Result<Value> {
    let schema: RootSchema = schemars::schema_for!(Config);
    let schema = json!(schema);
    Ok(schema)
}

async fn mode_fix() -> Result<()> {
    let path = get_file_path();
    let schema = serde_json::to_string_pretty(&get_updated_json().await?)?;
    log::info!("Updating JSON Schema: {}", path.to_str().unwrap());
    std::fs::create_dir_all(path.parent().unwrap())?;
    tokio::fs::write(path, schema).await?;
    Ok(())
}

async fn mode_check() -> Result<()> {
    let json_schema = get_file_path();
    let content = tokio::fs::read_to_string(json_schema).await?;
    let content = serde_json::from_str::<Value>(&content)?;
    let schema = get_updated_json().await?;
    match content.eq(&schema) {
        true => Ok(()),
        false => Err(anyhow!("Schema mismatch")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_file_path() {
        let path = get_file_path();
        assert_eq!(
            path,
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(JSON_SCHEMA_FILE)
        );
    }

    #[tokio::test]
    async fn test_get_updated_json() {
        let result = get_updated_json().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mode_fix() {
        let result = mode_fix().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mode_check() {
        let result = mode_check().await;
        assert!(result.is_ok());
    }
}
