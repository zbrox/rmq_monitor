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
    thresholds: Thresholds,
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
    expire_seconds: u64,
}

#[derive(Deserialize, Debug)]
struct SlackConfig {
    webhook_url: String,
    channel: String,
    screen_name: String,
}

#[derive(Deserialize, Debug)]
struct Thresholds {
    ready: Option<u64>,
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
        let queues_info = api::get_queue_info(
            &config.rabbitmq.protocol,
            &config.rabbitmq.host,
            &config.rabbitmq.port,
            &config.rabbitmq.username,
            &config.rabbitmq.password,
        )?;

        if let Some(threshold_ready) = config.thresholds.ready {
            let (_, errors): (Vec<_>, Vec<_>) = queues_info.into_iter()
                .filter(|qi| threshold_ready < qi.messages_ready)
                .map(|qi| {
                    api::SlackMsg {
                        username: config.slack.screen_name.clone(),
                        channel: format!("#{}", &config.slack.channel),
                        text: Some(format!("Queue {name} has passed a threshold of {threshold} ready messages. Currently at {number}.", 
                            name = &qi.name,
                            threshold = threshold_ready,
                            number = qi.messages_ready,
                        )),
                        icon_url: None,
                        attachments: None,
                    }
                })
                .map(|msg| {
                    log::info!(
                        "Sending message to #{}",
                        &config.slack.channel,
                    );
                    api::send_slack_msg(&config.slack.webhook_url, &msg)
                })
                .partition(|r| r.is_ok());

            let errors: Vec<anyhow::Error> = errors.into_iter().map(Result::unwrap_err).collect();
            errors.iter().for_each(|e| {
                log::error!("Error sending Slack message: {}", e);
            });
        }

        log::info!(
            "Check passed, sleeping for {}s",
            &config.settings.poll_seconds
        );
        thread::sleep(sleep_time);
    }

    Ok(())
}
