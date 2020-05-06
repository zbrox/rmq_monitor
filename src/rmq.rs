use anyhow::{anyhow, bail, Context, Result};
use serde_derive::Deserialize;
use serde_json::{json, Value as JsonValue};

#[derive(Deserialize, Debug)]
pub struct QueueInfo {
    pub name: String,
    pub state: String,
    pub stat: QueueStat,
}

#[derive(Deserialize, Debug)]
pub struct QueueStat {
    pub name: String,
    pub value: u64,
}

fn basic_auth_token(username: &str, password: &str) -> String {
    let combined = format!("{}:{}", username, password);
    let octet = combined.as_bytes();
    base64::encode(octet)
}

pub async fn get_queue_info(
    protocol: &str,
    host: &str,
    port: &str,
    username: &str,
    password: &str,
) -> Result<Vec<QueueInfo>> {
    let url = format!("{}://{}:{}/api/queues", protocol, host, port);
    let token = basic_auth_token(username, password);
    let mut response = match surf::get(url)
        .set_header("Authorization", format!("Basic {}", token))
        .await
    {
        Ok(response) => response,
        Err(error) => bail!(error),
    };

    if response.status() != 200 {
        return Err(anyhow!("RabbitMQ API Error: HTTP {}", response.status()));
    }

    let response_body: String = match response.body_string().await {
        Ok(body) => body,
        Err(error) => bail!(error),
    };

    let mut json: JsonValue = serde_json::from_str(&response_body)
        .context("Error parsing JSON from queues info API response")?;

    let processed_json = preprocess_queues_info_json(&mut json)
        .context("Error while processing RabbitMQ API response")?;

    let queue_info: Vec<QueueInfo> =
        serde_json::from_value(processed_json).context("Error parsing queues info API response")?;

    Ok(queue_info)
}

const QUEUE_INFO_KEYS: [&str; 5] = [
    "consumers",
    "memory",
    "messages",
    "messages_ready",
    "messages_unacknowledged",
];

fn form_queue_info_json_values(rmq_api_queue_item: &mut JsonValue) -> Result<Vec<JsonValue>> {
    let mut queue_info = Vec::new();

    let object = match rmq_api_queue_item.as_object_mut() {
        Some(object) => object,
        None => bail!("Could not parse queue info item in RabbitMQ API response"),
    };

    for k in QUEUE_INFO_KEYS.iter() {
        if object.contains_key(*k) {
            queue_info.push(json!({
                "name": object["name"],
                "state": object["state"],
                "stat": {
                    "name": k,
                    "value": object[*k]
                }
            }));
        }
    }

    Ok(queue_info)
}

fn preprocess_queues_info_json(json: &mut JsonValue) -> Option<JsonValue> {
    let list = match json.as_array_mut() {
        Some(list) => list,
        None => return None,
    };

    let queue_info: Vec<JsonValue> = list
        .iter_mut()
        .map(|queue_item| form_queue_info_json_values(queue_item))
        .filter_map(Result::ok)
        .flatten()
        .collect();

    if let Ok(qi) = serde_json::to_value(queue_info) {
        Some(qi)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {}
