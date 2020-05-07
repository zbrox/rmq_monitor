use serde_derive::Deserialize;
use anyhow::{Context, Result};
use std::fs::read_to_string;
use std::path::PathBuf;

use crate::rmq::StatType;

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
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Trigger {
    ConsumersTotal(TriggerData),
    MemoryTotal(TriggerData),
    MessagesTotal(TriggerData),
    MessagesReady(TriggerData),
    MessagesUnacknowledged(TriggerData),
    MessagesTotalRate(TriggerData),
    MessagesReadyRate(TriggerData),
    MessagesUnacknowledgedRate(TriggerData),
}

impl Trigger {
    pub fn data(&self) -> &TriggerData {
        match self {
            Trigger::ConsumersTotal(data) => data,
            Trigger::MemoryTotal(data) => data,
            Trigger::MessagesTotal(data) => data,
            Trigger::MessagesReady(data) => data,
            Trigger::MessagesUnacknowledged(data) => data,
            Trigger::MessagesTotalRate(data) => data,
            Trigger::MessagesReadyRate(data) => data,
            Trigger::MessagesUnacknowledgedRate(data) => data,
        }
    }

    pub fn stat_type(&self) -> StatType {
        match *self {
            Trigger::ConsumersTotal(_) => StatType::ConsumersTotal,
            Trigger::MemoryTotal(_) => StatType::MemoryTotal,
            Trigger::MessagesTotal(_) => StatType::MessagesTotal,
            Trigger::MessagesReady(_) => StatType::MessagesReady,
            Trigger::MessagesUnacknowledged(_) => StatType::MessagesUnacknowledged,
            Trigger::MessagesTotalRate(_) => StatType::MessagesTotalRate,
            Trigger::MessagesReadyRate(_) => StatType::MessagesReadyRate,
            Trigger::MessagesUnacknowledgedRate(_) => StatType::MessagesUnacknowledgedRate,
        }
    }

    pub fn name(&self) -> &'static str {
        match *self {
            Trigger::ConsumersTotal(_) => "total number of consumers",
            Trigger::MemoryTotal(_) => "memory consumption",
            Trigger::MessagesTotal(_) => "total number of messages",
            Trigger::MessagesReady(_) => "ready messages",
            Trigger::MessagesUnacknowledged(_) => "unacknowledged messages",
            Trigger::MessagesTotalRate(_) => "total message rate",
            Trigger::MessagesReadyRate(_) => "ready message rate",
            Trigger::MessagesUnacknowledgedRate(_) => "unacknowledged message rate",
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct TriggerData {
    pub threshold: f64,

    #[serde(default = "default_trigger_when")]
    pub trigger_when: TriggerWhen,

    pub queue: Option<String>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum TriggerWhen {
    Above,
    Below,
}

fn default_trigger_when() -> TriggerWhen {
    TriggerWhen::Above
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