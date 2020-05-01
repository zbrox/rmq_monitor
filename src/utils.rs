use anyhow::{Context, Result};
use async_std::{fs};
use std::path::PathBuf;
use toml;
use crate::config::{Config, Trigger};
use crate::rmq::{QueueStat};

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

pub fn check_trigger_applicability(trigger: &Trigger, queue_name: &str, stat: &QueueStat) -> bool {
    if let Some(trigger_queue_name) = &trigger.data().queue {
        return trigger_queue_name == queue_name && trigger.field_name() == stat.name;
    } else {
        return trigger.field_name() == stat.name;
    }
}


pub async fn check_loop() -> Result<()> {
    
    Ok(())
}