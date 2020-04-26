use reqwest::blocking::Client;
use serde_derive::{Serialize, Deserialize};
use anyhow::{anyhow, Context, Result};

#[derive(Deserialize, Debug)]
pub struct QueuesInfo {
    pub name: String,
    pub state: String,
    pub stats: Vec<QueueStat>,
}
#[derive(Deserialize, Debug, Clone)]
pub struct QueueStat {
    pub name: String,
    pub value: u64,
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

    let mut json: serde_json::Value = response.json().context("Error parsing queues info API response")?;
    preprocess_queues_info_json(&mut json);

    let queue_info: Vec<QueuesInfo> = serde_json::from_value(json).context("Error parsing queues info API response")?;

    Ok(queue_info)
}

fn preprocess_queues_info_json(json: &mut serde_json::Value) {
    let list = match json.as_array_mut() {
        Some(list) => list,
        None => return,
    };

    for i in list {
        let object = match i.as_object_mut() {
            Some(object) => object,
            None => return,
        };
        let mut stats = Vec::new();
        for k in vec!["consumers", "memory", "messages", "messages_ready", "messages_unacknowledged"] {
            if object.contains_key(k) {
                stats.push(serde_json::json!({"name": k, "value": object[k]}))
            }
        }
        object.insert("stats".into(), serde_json::json!(stats));
    }
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