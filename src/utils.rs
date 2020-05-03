use crate::config::{Config, RabbitMqConfig, SlackConfig, Trigger};
use crate::rmq::{get_queue_info, QueueInfo, QueueStat};
use crate::slack::{send_slack_msg, SlackMsg, SlackMsgMetadata};
use anyhow::{Context, Result};
use async_std::{fs, prelude::*, stream};
use std::path::PathBuf;
use std::thread;
use std::time::Duration;
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

pub async fn check_loop(
    poll_interval: Duration,
    rmq_config: RabbitMqConfig,
    slack_config: SlackConfig,
    triggers: Vec<Trigger>,
) -> Result<()> {
    let mut interval = stream::interval(poll_interval);
    // let mut active_trigger_registry: Vec<(&QueueName, &TriggerFieldname)> = vec![];
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
