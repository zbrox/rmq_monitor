use serde_derive::{Deserialize};
use anyhow::{anyhow, Context, Result, bail};
use surf;
use base64;

#[derive(Deserialize, Debug)]
pub struct QueueInfo {
    pub name: String,
    pub state: String,
    pub stat: QueueStat,
}

#[derive(Deserialize, Debug, Clone)]
pub struct QueueStat {
    pub name: String,
    pub value: u64,
}

fn basic_auth_token(username: &str, password: &str) -> String {
    let combined = format!("{}:{}", username, password);
    let octet = combined.as_bytes();
    let base64_encoded = base64::encode(octet);

    base64_encoded
}

pub async fn get_queue_info(protocol: &str, host: &str, port: &str, username: &str, password: &str) -> Result<Vec<QueueInfo>> {
    let url = format!("{}://{}:{}/api/queues", protocol, host, port);
    let token = basic_auth_token(username, password);
    let mut response = match surf::get(url)
        .set_header("Authorization", format!("Basic {}", token))
        .await {
            Ok(response) => response,
            Err(error) => bail!(error),
        };
    
    if response.status() != 200 {
        return Err(anyhow!("RabbitMQ API Error: HTTP {}", response.status()));
    }

    let response_body: String = match response.body_string().await {
        Ok(body) => body.into(),
        Err(error) => bail!(error),
    };
    
    let mut json: serde_json::Value = serde_json::from_str(&response_body)
        .context("Error parsing JSON from queues info API response")?;

    let processed_json = preprocess_queues_info_json(&mut json).context("Error while processing RabbitMQ API response")?;

    let queue_info: Vec<QueueInfo> = serde_json::from_value(processed_json).context("Error parsing queues info API response")?;

    Ok(queue_info)
}

fn preprocess_queues_info_json(json: &mut serde_json::Value) -> Option<serde_json::Value> {
    let list = match json.as_array_mut() {
        Some(list) => list,
        None => return None,
    };

    let mut queue_info = Vec::new();

    for i in list {
        let object = match i.as_object_mut() {
            Some(object) => object,
            None => return None,
        };
        
        for k in vec!["consumers", "memory", "messages", "messages_ready", "messages_unacknowledged"] {
            if object.contains_key(k) {
                queue_info.push(serde_json::json!({
                    "name": object["name"],
                    "state": object["state"],
                    "stat": {
                        "name": k,
                        "value": object[k]
                    }
                }));
            }
        }
    }

    if let Ok(qi) = serde_json::to_value(queue_info) {
        return Some(qi);
    } else {
        return None;
    }
}