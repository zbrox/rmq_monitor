use crate::config::{Config, RabbitMqConfig, SlackConfig, Trigger};
use crate::rmq::{get_queue_info, QueueInfo, QueueStat};
use crate::slack::{send_slack_msg, SlackMsg, SlackMsgMetadata};
use anyhow::{Context, Result};
use async_std::{fs, prelude::*, stream};
use std::collections::HashMap;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};
use toml;

pub async fn read_config(path: &PathBuf) -> Result<Config> {
    let config_contents: String = fs::read_to_string(path).await.with_context(|| {
        format!(
            "Could not read config {}",
            path.as_path().display().to_string()
        )
    })?;

    let config: Config = toml::from_str(&config_contents).context("Could not parse TOML config")?;
    Ok(config)
}

pub fn check_trigger_applicability(trigger: &Trigger, queue_name: &str, stat: &QueueStat) -> bool {
    if let Some(trigger_queue_name) = &trigger.data().queue {
        return trigger_queue_name == queue_name && trigger.field_name() == stat.name;
    } else {
        return trigger.field_name() == stat.name;
    }
}

type QueueTriggerType = String;
type UnixTimestamp = u64;

fn get_unix_timestamp() -> Result<UnixTimestamp> {
    Ok(SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs())
}

fn queue_trigger_name(queue_name: &str, trigger_name: &str) -> String {
    format!("{}:{}", queue_name, trigger_name)
}

pub async fn check_loop(
    poll_interval: Duration,
    rmq_config: RabbitMqConfig,
    slack_config: SlackConfig,
    triggers: Vec<Trigger>,
) -> Result<()> {
    let mut interval = stream::interval(poll_interval);

    let mut sent_msgs_registry: HashMap<QueueTriggerType, UnixTimestamp> = HashMap::new();

    while let Some(_) = interval.next().await {
        log::info!(
            "Checking queue info at {}://{}:{}",
            &rmq_config.protocol,
            &rmq_config.host,
            &rmq_config.port
        );

        let queue_info = get_queue_info(
            &rmq_config.protocol,
            &rmq_config.host,
            &rmq_config.port,
            &rmq_config.username,
            &rmq_config.password,
        )
        .await?;

        log::debug!("Fetched queue info: {:?}", queue_info);

        let msgs: Vec<SlackMsg> = triggers
            .iter()
            .map(|trigger| build_msgs_for_trigger(&queue_info, &trigger, &slack_config))
            .flat_map(|msgs| msgs)
            .collect();

        for msg in msgs {
            let queue_trigger_type =
                queue_trigger_name(&msg.metadata.queue_name, &msg.metadata.trigger_type);

            if !sent_msgs_registry.contains_key(&queue_trigger_type) {
                log::debug!(
                    "Haven't yet sent a message for queue {} of type {} has expired. Logging in registry.",
                    &msg.metadata.queue_name,
                    &msg.metadata.trigger_type
                );
                sent_msgs_registry.insert(
                    queue_trigger_type,
                    get_unix_timestamp().context("Cannot get UNIX timestamp")?,
                );
            } else if let Some(ts) = sent_msgs_registry.get(&queue_trigger_type) {
                let current_ts = get_unix_timestamp().context("Cannot get UNIX timestamp")?;
                if ts + (60 * 10) < current_ts {
                    log::debug!(
                        "Time since last message for queue {} of type {} has expired. Valid for sending again.",
                        &msg.metadata.queue_name,
                        &msg.metadata.trigger_type
                    );
                    *sent_msgs_registry
                        .get_mut(&queue_trigger_type)
                        .expect("No such entry in sent msgs log") = current_ts;
                } else {
                    log::debug!(
                        "Last message for queue {} of type {} was sent too recently. Skipping sending this one...",
                        &msg.metadata.queue_name,
                        &msg.metadata.trigger_type
                    );
                    continue;
                }
            }

            match send_slack_msg(&slack_config.webhook_url, &msg).await {
                Ok(_) => {
                    log::info!("Sent message to {}", msg.channel);
                    log::debug!(
                        "Slack message body {:#?}, sent on {:?}",
                        msg,
                        thread::current().id()
                    );
                }
                Err(e) => log::error!("Error sending Slack message: {}", e),
            }
        }

        log::info!("Check passed, sleeping for {}s", &poll_interval.as_secs(),);
    }
    Ok(())
}

fn build_msgs_for_trigger(
    queue_info: &Vec<QueueInfo>,
    trigger: &Trigger,
    slack_config: &SlackConfig,
) -> Vec<SlackMsg> {
    let msgs: Vec<SlackMsg> = queue_info
        .iter()
        .filter(|qi| check_trigger_applicability(trigger, &qi.name, &qi.stat))
        .filter(|qi| qi.stat.value > trigger.data().threshold)
        .map(|qi| {
            Some(SlackMsg {
                username: slack_config.screen_name.clone(),
                channel: format!("#{}", &slack_config.channel),
                icon_url: slack_config.icon_url.clone(),
                icon_emoji: slack_config.icon_emoji.clone(),
                metadata: SlackMsgMetadata {
                    queue_name: qi.name.clone(),
                    threshold: trigger.data().threshold,
                    current_value: qi.stat.value.clone(),
                    trigger_type: trigger.name().into(),
                },
            })
        })
        .filter_map(|v| v)
        .collect();

    msgs
}
