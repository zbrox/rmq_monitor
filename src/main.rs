mod config;
mod rmq;
mod slack;
mod utils;

use anyhow::Result;
use async_std::stream;
use async_std::task;
use futures::{
    future::FutureExt,
    stream::{futures_unordered::FuturesUnordered, StreamExt},
};
use human_panic::setup_panic;
use smol_str::SmolStr;
use std::{collections::HashMap, path::PathBuf, sync::Arc, thread, time::Duration};
use structopt::StructOpt;

use config::{read_config, RabbitMqConfig, SlackConfig, Trigger};
use rmq::get_queue_info;
use slack::send_slack_msg;
use utils::{
    build_msgs_for_trigger, get_unix_timestamp, has_msg_expired, ExpirationStatus, MsgExpirationLog,
};

#[derive(Debug, StructOpt)]
struct Cli {
    /// Path to the config.toml
    #[structopt(long = "config", short = "c", default_value = "config.toml")]
    config_path: PathBuf,

    /// Print debug logs
    #[structopt(short = "v", long = "verbose")]
    verbose: bool,
}

fn main() -> Result<()> {
    setup_panic!();
    let args = Cli::from_args();

    let log_filter = if args.verbose {
        "rmq_monitor=debug,surf"
    } else {
        "rmq_monitor=info"
    };
    env_logger::from_env(env_logger::Env::default().default_filter_or(log_filter)).init();

    let config = read_config(&args.config_path)?;

    log::info!(
        "Read config file from {}. Checking queue info every {}s.",
        &args.config_path.to_str().unwrap_or_default(),
        &config.settings.poll_seconds,
    );
    log::debug!("Config loaded: {:?}", config);

    let sleep_time = Duration::from_secs(config.settings.poll_seconds);
    task::block_on(check_loop(
        sleep_time,
        config.settings.msg_expiration_seconds,
        config.rabbitmq,
        config.slack,
        config.triggers,
    ))
}

pub async fn check_loop(
    poll_interval: Duration,
    expiration_in_seconds: u64,
    rmq_config: RabbitMqConfig,
    slack_config: SlackConfig,
    triggers: Vec<Trigger>,
) -> Result<()> {
    let mut interval = stream::interval(poll_interval);

    let mut sent_msgs_registry: MsgExpirationLog = HashMap::new();

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
                let queue_trigger_type = (SmolStr::new(&msg.metadata.queue_name), SmolStr::new(&msg.metadata.trigger_type));
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
                            log::info!("Sent message to {} about {}", msg.channel, msg.metadata.queue_name);
                            log::debug!(
                                "Slack message body {:?}, sent on {:?}",
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
