use anyhow::{anyhow, bail, Result};
use serde::Serializer;
use serde_derive::Serialize;
use std::sync::Arc;


#[derive(Serialize, Debug, Clone)]
pub struct SlackMsg {
    pub username: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon_emoji: Option<String>,
    pub channel: String,

    #[serde(serialize_with = "slack_metadata_to_msg_text", rename = "text")]
    pub metadata: SlackMsgMetadata,
}

fn slack_metadata_to_msg_text<S>(metadata: &SlackMsgMetadata, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let text = format!(
        "Queue *{name}* has passed a threshold of {threshold} {trigger_type}. Currently at *{number}*.",
        name = metadata.queue_name,
        threshold = metadata.threshold,
        trigger_type = metadata.trigger_type,
        number = metadata.current_value,
    );
    s.serialize_str(&text)
}

#[derive(Debug, Clone)]
pub struct SlackMsgMetadata {
    pub queue_name: String,
    pub threshold: f64,
    pub current_value: f64,
    pub trigger_type: String,
}

pub async fn send_slack_msg(webhook_url: &str, msg: Arc<SlackMsg>) -> Result<()> {
    let mut response = match surf::post(webhook_url).body_json(&msg.as_ref()).map_err(anyhow::Error::msg)?.await {
        Ok(response) => response,
        Err(error) => bail!(error),
    };

    if response.status() != 200 {
        let body_string = match response.body_string().await {
            Ok(body_string) => body_string,
            Err(error) => bail!(error),
        };
        return Err(anyhow!(
            "Slack API Error: HTTP {} {}",
            response.status(),
            body_string
        ));
    }

    Ok(())
}
