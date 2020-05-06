use serde_derive::Deserialize;
use anyhow::{Context, Result};
use std::fs::read_to_string;
use std::path::PathBuf;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub rabbitmq: RabbitMqConfig,
    pub settings: MonitorSettings,
    pub slack: SlackConfig,
    pub triggers: Vec<Trigger>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct RabbitMqConfig {
    #[serde(default = "default_protocol")]
    pub protocol: String,
    pub host: String,
    pub username: String,
    pub password: String,
    pub port: String,
    pub vhost: String,
}

fn default_protocol() -> String {
    "https".into()
}

#[derive(Deserialize, Debug)]
pub struct MonitorSettings {
    pub poll_seconds: u64,
    #[serde(default = "default_expiration")]
    pub msg_expiration_seconds: u64,
}

fn default_expiration() -> u64 {
    600
}

#[derive(Deserialize, Debug)]
pub struct SlackConfig {
    pub webhook_url: String,
    pub channel: String,
    pub screen_name: String,
    pub icon_url: Option<String>,
    pub icon_emoji: Option<String>,
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
pub enum Trigger {
    #[serde(rename = "consumers_total")]
    ConsumersTotal(TriggerData),

    #[serde(rename = "memory_total")]
    MemoryTotal(TriggerData),

    #[serde(rename = "messages_total")]
    MessagesTotal(TriggerData),

    #[serde(rename = "messages_ready")]
    ReadyMsgs(TriggerData),

    #[serde(rename = "messages_unacknowledged")]
    UnacknowledgedMsgs(TriggerData),
}

impl Trigger {
    pub fn data(&self) -> &TriggerData {
        match self {
            Trigger::ConsumersTotal(data) => data,
            Trigger::MemoryTotal(data) => data,
            Trigger::MessagesTotal(data) => data,
            Trigger::ReadyMsgs(data) => data,
            Trigger::UnacknowledgedMsgs(data) => data,
        }
    }

    pub fn field_name(&self) -> &'static str {
        match *self {
            Trigger::ConsumersTotal(_) => "consumers",
            Trigger::MemoryTotal(_) => "memory",
            Trigger::MessagesTotal(_) => "messages",
            Trigger::ReadyMsgs(_) => "messages_ready",
            Trigger::UnacknowledgedMsgs(_) => "messages_unacknowledged",
        }
    }

    pub fn name(&self) -> &'static str {
        match *self {
            Trigger::ConsumersTotal(_) => "total number of consumers",
            Trigger::MemoryTotal(_) => "memory consumption",
            Trigger::MessagesTotal(_) => "total number of messages",
            Trigger::ReadyMsgs(_) => "ready messages",
            Trigger::UnacknowledgedMsgs(_) => "unacknowledged messages",
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct TriggerData {
    pub threshold: u64,
    pub queue: Option<String>,
}

pub fn read_config(path: &PathBuf) -> Result<Config> {
    let config_contents: String = read_to_string(path).with_context(|| {
        format!(
            "Could not read config {}",
            path.as_path().display().to_string()
        )
    })?;

    let config: Config = toml::from_str(&config_contents).context("Could not parse TOML config")?;
    Ok(config)
}