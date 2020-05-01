use anyhow::{Context, Result};
use async_std::{fs};
use std::path::PathBuf;
use toml;
use crate::config::Config;

pub async fn read_config(path: &PathBuf) -> Result<Config> {
    let config_contents: String = fs::read_to_string(path).await.with_context(|| {
        format!(
            "Could not read config {}",
            path.as_path().display().to_string()
        )
    })?;

    let config: Config = toml::from_str(&config_contents).context("Could not parse TOML config")?;
    Ok(config)
}
