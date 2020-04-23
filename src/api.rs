use reqwest::blocking::Client;
use serde_derive::{Serialize, Deserialize};
use anyhow::{anyhow, Context, Result};

#[derive(Deserialize, Debug)]
pub struct QueuesInfo {
    pub consumers: u64,
    pub memory: u64,
    pub messages: u64,
    pub messages_ready: u64,
    pub messages_unacknowledged: u64,
    pub name: String,
    pub state: String,
}

#[derive(Serialize, Debug)]
pub struct SlackMsg {
    pub username: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon_url: Option<String>,
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

pub fn get_queue_info(protocol: &str, host: &str, port: &str, username: &str, password: &str) -> Result<Vec<QueuesInfo>> {
    let client = Client::new();
    let url = format!("{}://{}:{}/api/queues", protocol, host, port);
    
    let response = client
        .get(&url)
        .basic_auth(username, Some(password.to_owned()))
        .send()?;
    
    if response.status() != 200 {
        return Err(anyhow!("RabbitMQ API Error: HTTP {}", response.status()));
    }

    let body: Vec<QueuesInfo> = response.json().context("Error parsing queues info API response")?;

    Ok(body)
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