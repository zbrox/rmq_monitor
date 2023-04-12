# RabbitMQ monitor

![Build badge](https://github.com/zbrox/rmq_monitor/workflows/Build/badge.svg)

This is a simple tool which monitors RabbitMQ queues and notifies via Slack (legacy webhooks) when certain thresholds are met.

## Installation

This is published on [crates.io](https://crates.io/) so if you have `cargo` setup you can just do:

```sh
cargo install rmq_monitor
```

## Options

```txt
    -c, --config <config>    Your TOML config file (default is config.toml)
```

### Config

The tool uses a [TOML](https://github.com/toml-lang/toml) config file. If you don't pass any `--config` argument it will look for a `config.toml` in the working directory.

### Triggers

Triggers can be activated by a value either being above or below the given threshold. The default is above, but if you add `trigger_when = "below"` to the trigger configuration it will be triggered when the given value falls below what you specify.

Here's a sample trigger definition to put in a config.toml file:

```toml
[[triggers]]
type = "messages_ready"
threshold = 10000
queue = "sent_images"
```

This trigger will activate and send a message when a queue called `sent_images` goes above 10000 ready messages.

### Available triggers

Here are the currently available triggers and their type field. If you put an invalid type for a trigger `rmq_monitor` won't start up and print out the error due to inability to parse the config.

- **Total number of consumers** (`type = "consumers_total"`) - How many consumers are currently consuming from the queue
- **Total memory** (`type = "memory_total"`) - Total memory used by the queue
- **Total number of messages** (`type = "messages_total"`) - The total number of messages currently on the queue
- **Number of ready messages** (`type = "messages_ready"`) - The number of messages available to consumers, ready to be delivered
- **Number of unacknowledged messages** (`type = "messages_unacknowledged"`) - The number of messages delivered to a consumer but not yet acked
- **Number of redelivered messages** (`type = "messages_redelivered"`) - The number of redelivered messages (due to being rejected)
- **Total rate of messages** (`type = "messages_total_rate"`) - The rate (*per second*) at which messages move in and out of the queue
- **Rate of ready messages** (`type = "messages_ready_rate"`) - The rate (*per second*) at which ready messages change
- **Rate of unacknowledged messages** (`type = "messages_unacknowledged_rate"`) - The rate (*per second*) at which unacknowledged messages change
- **Publishing rate** (`type = "messages_publish_rate"`) - The rate (*per second*) at which messages are published on the queue
- **Delivery rate** (`type = "messages_delivery_rate"`) - The rate (*per second*) at which messages are delivered by the queue
- **Redelivery rate** (`type = "messages_redeliver_rate"`) - The rate (*per second*) at which messages are redelivered to the queue (because of rejection)

### Docker image

There's a minimal Docker image [published on Docker hub](https://hub.docker.com/r/zbrox/rmq_monitor). The size of the image is only 11Mb.

To use it you only need to mount a volume with your config file. The container will be looking for the config file in `/config/config.toml` so mount it there. Example:

```sh
docker run -it -v (pwd)/your_config.toml:/config/config.toml --rm zbrox/rmq_monitor:latest
```

or if you're running in kubernetes and it's easier to mount the whole folder:

```sh
docker run -it -v (pwd)/where_i_keep_configs:/config --rm zbrox/rmq_monitor:latest
```

Obviously in this case you have to name the config file in that folder also `config.toml`. Later I'll add a container variable to be able to change that name as well.
