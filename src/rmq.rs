use anyhow::{anyhow, bail, Context, Result};
use serde_derive::Deserialize;
use serde_json::{json, Value as JsonValue};
use base64::engine::Engine as _;
use base64::engine::general_purpose::STANDARD as Base64StandardEngine;

#[derive(Deserialize, Debug)]
pub struct QueueInfo {
    pub name: String,
    pub state: String,
    pub stat: QueueStat,
}

#[derive(Deserialize, Debug)]
pub struct QueueStat {
    pub stat_type: StatType,
    pub value: f64,
}

#[derive(Deserialize, Debug, PartialEq)]
pub enum StatType {
    ConsumersTotal,
    MemoryTotal,
    MessagesTotal,
    MessagesReady,
    MessagesUnacknowledged,
    MessagesTotalRate,
    MessagesReadyRate,
    MessagesUnacknowledgedRate,
    MessagesPublishRate,
    MessagesDeliveryRate,
    MessagesRedelivered,
    MessagesRedeliverRate,
}

impl StatType {
    fn variants() -> &'static [StatType] {
        &[
            StatType::ConsumersTotal,
            StatType::MemoryTotal,
            StatType::MessagesTotal,
            StatType::MessagesReady,
            StatType::MessagesUnacknowledged,
            StatType::MessagesTotalRate,
            StatType::MessagesReadyRate,
            StatType::MessagesUnacknowledgedRate,
            StatType::MessagesPublishRate,
            StatType::MessagesDeliveryRate,
            StatType::MessagesRedelivered,
            StatType::MessagesRedeliverRate,
        ]
    }

    fn json_path(&self) -> &str {
        match &self {
            StatType::ConsumersTotal => "consumers",
            StatType::MemoryTotal => "memory", 
            StatType::MessagesTotal => "messages", 
            StatType::MessagesReady => "messages_ready", 
            StatType::MessagesUnacknowledged => "messages_unacknowledged", 
            StatType::MessagesTotalRate => "messages_details.rate",
            StatType::MessagesReadyRate => "messages_ready_details.rate",
            StatType::MessagesUnacknowledgedRate => "messages_unacknowledged_details.rate",
            StatType::MessagesPublishRate => "message_stats.publish_details.rate",
            StatType::MessagesDeliveryRate => "message_stats.deliver_get_details.rate",
            StatType::MessagesRedelivered => "message_stats.redeliver", 
            StatType::MessagesRedeliverRate => "message_stats.redeliver_details.rate",
        }
    }

    fn to_str(&self) -> &str {
        match &self {
            StatType::ConsumersTotal => "ConsumersTotal",
            StatType::MemoryTotal => "MemoryTotal", 
            StatType::MessagesTotal => "MessagesTotal", 
            StatType::MessagesReady => "MessagesReady", 
            StatType::MessagesUnacknowledged => "MessagesUnacknowledged", 
            StatType::MessagesTotalRate => "MessagesTotalRate",
            StatType::MessagesReadyRate => "MessagesReadyRate",
            StatType::MessagesUnacknowledgedRate => "MessagesUnacknowledgedRate",
            StatType::MessagesPublishRate => "MessagesPublishRate",
            StatType::MessagesDeliveryRate => "MessagesDeliveryRate",
            StatType::MessagesRedelivered => "MessagesRedelivered",
            StatType::MessagesRedeliverRate => "MessagesRedeliverRate",
        }
    }
}

fn basic_auth_token(username: &str, password: &str) -> String {
    let combined = format!("{}:{}", username, password);
    let octet = combined.as_bytes();
    Base64StandardEngine.encode(octet)
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
        .header("Authorization", format!("Basic {}", token))
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

fn get_by_path<'a>(path: &str, json_value: &'a JsonValue) -> Option<&'a JsonValue> {
    let path_breakdown: Vec<&str> = path.split('.').collect();
    let initial: Option<&JsonValue> = json_value.get(path_breakdown[0]);

    path_breakdown
        .iter()
        .skip(1)
        .try_fold(initial, |current, key| {
            if let Some(v) = current {
                Ok(v.get(key))
            } else {
                bail!("No such value found for path: {}", path);
            }
        })
        .unwrap_or_else(|_| None)
}

fn build_queue_info_json_values(rmq_api_queue_item: &JsonValue) -> Result<Vec<JsonValue>> {
    let mut queue_info = Vec::new();

    for k in StatType::variants().iter() {
        let value = get_by_path(k.json_path(), rmq_api_queue_item);
        if value.is_none() {
            continue;
        }

        queue_info.push(json!({
            "name": rmq_api_queue_item.get("name"),
            "state": rmq_api_queue_item.get("state"),
            "stat": {
                "stat_type": k.to_str(),
                "value": value,
            }
        }));
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
        .map(|queue_item| build_queue_info_json_values(queue_item))
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
