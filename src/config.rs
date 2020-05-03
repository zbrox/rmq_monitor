use serde_derive::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub rabbitmq: RabbitMqConfig,
    pub settings: MonitorSettings,
    pub slack: SlackConfig,
    pub triggers: Vec<Trigger>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct RabbitMqConfig {
    pub protocol: String,
    pub host: String,
    pub username: String,
    pub password: String,
    pub port: String,
    pub vhost: String,
}

#[derive(Deserialize, Debug)]
pub struct MonitorSettings {
    pub poll_seconds: u64,
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

#[derive(Deserialize, Debug)]
pub enum TriggerType {
    Ready,
}
