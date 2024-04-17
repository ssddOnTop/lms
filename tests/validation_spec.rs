#[cfg(test)]
mod validation_spec {
    // tests all the invalid configs
    #[tokio::test]
    async fn validate() -> anyhow::Result<()> {
        let runtime = lms::cli::rt::init();
        let reader = lms_core::config::reader::ConfigReader::init(runtime);
        for entry in std::fs::read_dir("tests/invalid_configs")? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                continue;
            }

            let path = path.to_str().unwrap();
            if let Ok(config) = reader.read(path).await {
                let blueprint = lms_core::blueprint::Blueprint::try_from(config);
                assert!(blueprint.is_err(), "Expected error for {}", path);
            }
        }
        Ok(())
    }
}
