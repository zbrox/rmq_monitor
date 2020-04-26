use reqwest::blocking::Client;
use serde_derive::{Deserialize};
use anyhow::{anyhow, Context, Result};

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

pub fn get_queue_info(protocol: &str, host: &str, port: &str, username: &str, password: &str) -> Result<Vec<QueueInfo>> {
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
    let processed_json = preprocess_queues_info_json(&mut json).context("Error while processing RabbitMQ API response")?;

    let queue_info: Vec<QueueInfo> = serde_json::from_value(processed_json).context("Error parsing queues info API response")?;

    Ok(queue_info)
}

// fn flatten_queue_info(queue_info: Vec<QueuesInfo>) -> Vec<(String, String, QueueStat)> {
//     queue_info
//         .iter()
//         .flat_map(|qi| {
//             let r: Vec<(&str, &str, &QueueStat)> = qi
//                 .stats
//                 .iter()
//                 .map(|stat| (qi.name.as_str(), qi.state.as_str(), stat))
//                 .collect();
//             r
//         })
//         .collect()
// }

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