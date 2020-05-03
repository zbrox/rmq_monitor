use serde_derive::{Serialize};
use anyhow::{anyhow, Result, bail};
use surf;

#[derive(Serialize, Debug, Clone)]
pub struct SlackMsg {
    pub username: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon_emoji: Option<String>,
    pub channel: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

#[derive(Serialize, Debug, Clone)]
pub struct SlackMsgField {
    pub title: String,
    pub value: String,
    pub short: bool,
}

#[derive(Serialize, Debug)]
struct SlackMsgPayload {
    payload: String,
}

pub async fn send_slack_msg(webhook_url: &str, msg: &SlackMsg) -> Result<()> {
    let mut response = match surf::post(webhook_url).body_json(msg)?.await {
        Ok(response) => response,
        Err(error) => bail!(error),
    };
    
    if response.status() != 200 {
        let body_string = match response.body_string().await {
            Ok(body_string) => body_string,
            Err(error) => bail!(error),
        };
        return Err(anyhow!("Slack API Error: HTTP {} {}", response.status(), body_string));
    }

    Ok(())
}