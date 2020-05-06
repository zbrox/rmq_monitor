use anyhow::{Context, Result};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use smol_str::SmolStr;

use crate::config::{SlackConfig, Trigger};
use crate::rmq::{QueueInfo, QueueStat};
use crate::slack::{SlackMsg, SlackMsgMetadata};

pub fn check_trigger_applicability(trigger: &Trigger, queue_name: &str, stat: &QueueStat) -> bool {
    if let Some(trigger_queue_name) = &trigger.data().queue {
        trigger_queue_name == queue_name && trigger.stat_type() == stat.stat_type
    } else {
        trigger.stat_type() == stat.stat_type
    }
}

pub type UnixTimestamp = u64;

pub fn get_unix_timestamp() -> Result<UnixTimestamp> {
    Ok(SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs())
}

pub enum ExpirationStatus {
    Expired,
    NotSentYet,
    NotExpired,
}

pub type QueueName = SmolStr;
pub type TriggerType = SmolStr;
pub type MsgExpirationLog = HashMap<(QueueName, TriggerType), UnixTimestamp>;

pub fn has_msg_expired(
    msg_expiration_log: &mut MsgExpirationLog,
    queue_trigger_type: (QueueName, TriggerType),
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

pub fn build_msgs_for_trigger(
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
