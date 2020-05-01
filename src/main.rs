mod config;
mod rmq;
mod slack;
mod utils;

use anyhow::{Result};
use human_panic::setup_panic;
use rmq::{get_queue_info, QueueStat};
use slack::{send_multiple_slack_msgs, SlackMsg};
use std::path::PathBuf;
use std::{thread, time};
use structopt::StructOpt;
use async_std::task;
use utils::read_config;
use config::{QueueName, Trigger, TriggerFieldname};

#[derive(Debug, StructOpt)]
struct Cli {
    /// Path to the config.toml
    #[structopt(long = "config", short = "c", default_value = "config.toml")]
    config_path: PathBuf,
}

fn main() -> Result<()> {
    setup_panic!();
    let args = Cli::from_args();

    env_logger::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let config_path = args.config_path.clone();
    let read_config_task = task::spawn(async move {
        read_config(&config_path).await
    });

    let config = task::block_on(read_config_task)?;

    log::info!(
        "Read config file from {}",
        &args.config_path.to_str().unwrap_or_default()
    );
    log::debug!("Config loaded: {:?}", config);

    let sleep_time = time::Duration::from_secs(config.settings.poll_seconds);
    loop {
        log::info!(
            "Checking queue info at {}:{}",
            &config.rabbitmq.host,
            &config.rabbitmq.port
        );
        let queue_info = get_queue_info(
            &config.rabbitmq.protocol,
            &config.rabbitmq.host,
            &config.rabbitmq.port,
            &config.rabbitmq.username,
            &config.rabbitmq.password,
        )?;
        log::debug!("Fetched queue info: {:?}", queue_info);

        let mut active_trigger_registry: Vec<(&QueueName, &TriggerFieldname)> = vec![];
        let msgs: Vec<SlackMsg> = config.triggers.iter()
            .map(|t| {
                let msgs: Vec<SlackMsg> = queue_info.iter()
                    .filter(|qi| check_trigger_applicability(t, &qi.name, &qi.stat))
                    .filter(|qi| qi.stat.value > t.data().threshold)
                    .map(|qi| {
                        if active_trigger_registry.contains(&(&qi.name, t.field_name())) {
                            return None;
                        }
                        active_trigger_registry.push((&qi.name, t.field_name()));
                        Some(SlackMsg {
                            username: config.slack.screen_name.clone(),
                            channel: format!("#{}", &config.slack.channel),
                            text: Some(format!("Queue {name} has passed a threshold of {threshold} {trigger_type}. Currently at {number}.", 
                                name = &qi.name,
                                threshold = t.data().threshold,
                                number = qi.stat.value,
                                trigger_type = t.name(),
                            )),
                            icon_url: config.slack.icon_url.clone(),
                            icon_emoji: config.slack.icon_emoji.clone(),
                            attachments: None,
                        })
                    })
                    .filter_map(|v| v)
                    .collect();
                return msgs;
            })
            .flat_map(|msgs| msgs)
            .collect();

        send_multiple_slack_msgs(&config.slack.webhook_url, &msgs)?;

        active_trigger_registry.clear();

        log::info!(
            "Check passed, sleeping for {}s",
            &config.settings.poll_seconds
        );
        thread::sleep(sleep_time);
    }
}

fn check_trigger_applicability(trigger: &Trigger, queue_name: &str, stat: &QueueStat) -> bool {
    if let Some(trigger_queue_name) = &trigger.data().queue {
        return trigger_queue_name == queue_name && trigger.field_name() == stat.name;
    } else {
        return trigger.field_name() == stat.name;
    }
}
