use crate::config::{Config, RabbitMqConfig, SlackConfig, Trigger};
use crate::rmq::{get_queue_info, QueueInfo, QueueStat};
use crate::slack::{send_slack_msg, SlackMsg, SlackMsgMetadata};
use anyhow::{Context, Result};
use async_std::{fs, stream};
use futures::{
    future::FutureExt,
    stream::{futures_unordered::FuturesUnordered, StreamExt},
};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

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
        trigger_queue_name == queue_name && trigger.field_name() == stat.name
    } else {
        trigger.field_name() == stat.name
    }
}

type QueueTriggerType = String;
type UnixTimestamp = u64;
type MsgExpirationLog = HashMap<QueueTriggerType, UnixTimestamp>;

fn get_unix_timestamp() -> Result<UnixTimestamp> {
    Ok(SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs())
}

fn queue_trigger_name(queue_name: &str, trigger_name: &str) -> String {
    format!("{}:{}", queue_name, trigger_name)
}

pub async fn check_loop(
    poll_interval: Duration,
    expiration_in_seconds: u64,
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

        triggers
            .iter()
            .map(|trigger| build_msgs_for_trigger(&queue_info, &trigger, &slack_config))
            .flatten()
            .filter_map(|msg| {
                let queue_trigger_type =
                    queue_trigger_name(&msg.metadata.queue_name, &msg.metadata.trigger_type);
                let current_ts = get_unix_timestamp().ok()?;
                match has_msg_expired(
                    &mut sent_msgs_registry,
                    queue_trigger_type,
                    current_ts,
                    expiration_in_seconds,
                ) {
                    Ok(ExpirationStatus::Expired) => {
                        log::debug!(
                            "Message for queue {} of type {} has expired (expiration time is {}s). Resending...",
                            &msg.metadata.queue_name,
                            &msg.metadata.trigger_type,
                            &expiration_in_seconds,
                        );
                        Some(msg)
                    }
                    Ok(ExpirationStatus::NotExpired) => {
                        log::debug!(
                            "Last message for queue {} of type {} was sent too recently. Skipping sending this one...",
                            &msg.metadata.queue_name,
                            &msg.metadata.trigger_type
                        );
                        None
                    }
                    Ok(ExpirationStatus::NotSentYet) => {
                        log::debug!(
                            "Haven't yet sent a message for queue {} of type {} has expired. Saved in log.",
                            &msg.metadata.queue_name,
                            &msg.metadata.trigger_type
                        );
                        Some(msg)
                    }
                    Err(error) => {
                        log::error!("Unexpected error with msgs: {}", error);
                        None
                    }
                }
            })
            .map(|msg| {
                let msg = Arc::new(msg);
                send_slack_msg(&slack_config.webhook_url, Arc::clone(&msg))
                .map(move |x| {
                        match x {
                        Ok(_) => {
                            log::info!("Sent message to {}", msg.channel);
                            log::debug!(
                                "Slack message body {:#?}, sent on {:?}",
                                msg,
                                thread::current().id()
                            );
                        }
                        Err(e) => log::error!("Error sending Slack message: {}", e),
                    };
                })
            })
            .collect::<FuturesUnordered<_>>()
            .collect::<Vec<()>>().await;

        log::info!("Check passed, sleeping for {}s", &poll_interval.as_secs(),);
    }
    Ok(())
}

enum ExpirationStatus {
    Expired,
    NotSentYet,
    NotExpired,
}

fn has_msg_expired(
    msg_expiration_log: &mut MsgExpirationLog,
    queue_trigger_type: QueueTriggerType,
    current_ts: UnixTimestamp,
    expiration_in_seconds: u64,
) -> Result<ExpirationStatus> {
    match msg_expiration_log.get(&queue_trigger_type) {
        Some(ts) => {
            if ts + expiration_in_seconds < current_ts {
                *msg_expiration_log
                    .get_mut(&queue_trigger_type)
                    .expect("No such entry in sent msgs log") = current_ts;
                Ok(ExpirationStatus::Expired)
            } else {
                Ok(ExpirationStatus::NotExpired)
            }
        }
        None => {
            msg_expiration_log.insert(
                queue_trigger_type,
                get_unix_timestamp().context("Cannot get UNIX timestamp")?,
            );
            Ok(ExpirationStatus::NotSentYet)
        }
    }
}

fn build_msgs_for_trigger(
    queue_info: &[QueueInfo],
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
                    current_value: qi.stat.value,
                    trigger_type: trigger.name().into(),
                },
            })
        })
        .filter_map(|v| v)
        .collect();

    msgs
}
