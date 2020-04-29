# RabbitMQ monitor

![](https://github.com/zbrox/rmq_monitor/workflows/Build/badge.svg)

This is a simple tool which monitors RabbitMQ and notifies via Slack (legacy webhooks) when certain thresholds are met.

## Options

```
    -c, --config <config>    Your TOML config file (default is config.toml)
```

### Config

The tool uses a [TOML](https://github.com/toml-lang/toml) config file. If you don't pass any `--config` argument it will look for a `config.toml` in the working directory.

There's an example included in this repo called `config_sample.toml`.

