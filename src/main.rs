mod config;
mod rmq;
mod slack;
mod utils;

use anyhow::Result;
use async_std::task;
use human_panic::setup_panic;
use std::path::PathBuf;
use std::time;
use structopt::StructOpt;
use utils::{check_loop, read_config};

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
    let read_config_task = task::spawn(async move { read_config(&config_path).await });

    let config = task::block_on(read_config_task)?;

    log::info!(
        "Read config file from {}",
        &args.config_path.to_str().unwrap_or_default()
    );
    log::debug!("Config loaded: {:?}", config);

    let sleep_time = time::Duration::from_secs(config.settings.poll_seconds);
    task::block_on(check_loop(
        sleep_time,
        config.rabbitmq,
        config.slack,
        config.triggers,
    ))
}
