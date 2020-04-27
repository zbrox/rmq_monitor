use reqwest::blocking::Client;
use serde_derive::{Serialize};
use anyhow::{anyhow, Result};

#[derive(Serialize, Debug)]
pub struct SlackMsg {
    pub username: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon_emoji: Option<String>,
    pub channel: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attachments: Option<Vec<SlackAttachment>>,
}

#[derive(Serialize, Debug)]
pub struct SlackAttachment {
    pub fallback: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    pub pretext: String,
    pub author_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author_link: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author_icon: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mrkdwn_in: Option<Vec<String>>,
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumb_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fields: Option<Vec<SlackMsgField>>,
}

#[derive(Serialize, Debug)]
pub struct SlackMsgField {
    pub title: String,
    pub value: String,
    pub short: bool,
}

#[derive(Serialize, Debug)]
struct SlackMsgPayload {
    payload: String,
}

pub fn send_slack_msg(webhook_url: &str, msg: &SlackMsg) -> Result<()> {
    let client = Client::new();
    
    let response = client
        .post(webhook_url)
        .json(msg)
        .send()?;

    if response.status() != 200 {
        return Err(anyhow!("Slack API Error: HTTP {} {}", response.status(), response.text()?));
    }

    Ok(())
}

pub fn send_multiple_slack_msgs(webhook_url: &str, msgs: &Vec<SlackMsg>) -> Result<()> {
    msgs.iter()
        .map(|msg| {
            log::debug!("Message body {:?}", msg);
            log::info!("Sending message to {}", msg.channel);
            send_slack_msg(webhook_url, msg)
        })
        .for_each(|v| {
            match v {
                Ok(_) => {}
                Err(e) => log::error!("Error sending Slack message: {}", e),
            };
        });
    
    Ok(())
}