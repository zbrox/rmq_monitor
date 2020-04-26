mod api;

use anyhow::Result;
use human_panic::setup_panic;
use quicli::prelude::*;
use std::fs;
use std::path::PathBuf;
use std::{thread, time};
use structopt::StructOpt;
use toml;

#[derive(Debug, StructOpt)]
struct Cli {
    /// Path to the config.toml
    #[structopt(long = "config", short = "c", default_value = "config.toml")]
    config_path: PathBuf,
}

#[derive(Deserialize, Debug)]
struct Config {
    rabbitmq: RabbitMqConfig,
    settings: MonitorSettings,
    slack: SlackConfig,
    triggers: Vec<Trigger>,
}

#[derive(Deserialize, Debug)]
struct RabbitMqConfig {
    protocol: String,
    host: String,
    username: String,
    password: String,
    port: String,
    vhost: String,
}

#[derive(Deserialize, Debug)]
struct MonitorSettings {
    poll_seconds: u64,
    alert_expire_seconds: u64,
}

#[derive(Deserialize, Debug)]
struct SlackConfig {
    webhook_url: String,
    channel: String,
    screen_name: String,
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
enum Trigger {
    #[serde(rename = "ready_msgs")]
    ReadyMsgs(TriggerData),
}

impl Trigger {
    fn data(&self) -> &TriggerData {
        match self {
            Trigger::ReadyMsgs(data) => data,
        }
    }

    fn field_name(&self) -> &'static str {
        match *self {
            Trigger::ReadyMsgs(_) => "messages_ready",
        }
    }

    fn name(&self) -> &'static str {
        match *self {
            Trigger::ReadyMsgs(_) => "ready messages",
        }
    }
}

#[derive(Deserialize, Debug)]
struct TriggerData {
    threshold: u64,
    queue: Option<String>,
}

#[derive(Deserialize, Debug)]
enum TriggerType {
    Ready,
}

fn main() -> Result<()> {
    setup_panic!();
    let args = Cli::from_args();

    env_logger::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let config_str = fs::read_to_string(&args.config_path)?;
    let config: Config = toml::from_str(&config_str)?;

    log::info!(
        "Read config file from {}",
        &args.config_path.to_str().unwrap_or_default()
    );

    let sleep_time = time::Duration::from_secs(config.settings.poll_seconds);
    loop {
        log::info!(
            "Checking queue info at {}:{}",
            &config.rabbitmq.host,
            &config.rabbitmq.port
        );
        let queue_info = api::get_queue_info(
            &config.rabbitmq.protocol,
            &config.rabbitmq.host,
            &config.rabbitmq.port,
            &config.rabbitmq.username,
            &config.rabbitmq.password,
        )?;

        let queue_info_flat: Vec<(&str, &str, &api::QueueStat)> = queue_info
            .iter()
            .flat_map(|qi| {
                let r: Vec<(&str, &str, &api::QueueStat)> = qi
                    .stats
                    .iter()
                    .map(|stat| (qi.name.as_str(), qi.state.as_str(), stat))
                    .collect();
                r
            })
            .collect();
        let msgs: Vec<api::SlackMsg> = config.triggers.iter()
            .map(|t| {
                let msgs: Vec<api::SlackMsg> = queue_info_flat.iter()
                    .filter(|(queue_name, _, stat)| check_trigger_applicability(t, queue_name, stat))
                    .filter(|(_, _, stat)| stat.value > t.data().threshold)
                    .map(|(queue_name, _, stat)| {
                        api::SlackMsg {
                            username: config.slack.screen_name.clone(),
                            channel: format!("#{}", &config.slack.channel),
                            text: Some(format!("Queue {name} has passed a threshold of {threshold} {trigger_type}. Currently at {number}.", 
                                name = &queue_name,
                                threshold = t.data().threshold,
                                number = stat.value,
                                trigger_type = t.name(),
                            )),
                            icon_url: None,
                            attachments: None,
                        }
                    })
                    .collect();
                return msgs;
            })
            .flat_map(|msgs| msgs)
            .collect();

        msgs.iter()
            .map(|msg| {
                log::info!("Sending message to #{}", &config.slack.channel,);
                api::send_slack_msg(&config.slack.webhook_url, msg)
            })
            .for_each(|v| {
                match v {
                    Ok(_) => {}
                    Err(e) => log::error!("Error sending Slack message: {}", e),
                };
            });

        log::info!(
            "Check passed, sleeping for {}s",
            &config.settings.poll_seconds
        );
        thread::sleep(sleep_time);
    }

    Ok(())
}

fn check_trigger_applicability(trigger: &Trigger, queue_name: &str, stat: &api::QueueStat) -> bool {
    if let Some(trigger_queue_name) = &trigger.data().queue {
        return trigger_queue_name == queue_name && trigger.field_name() == stat.name;
    } else {
        return trigger.field_name() == stat.name;
    }
}