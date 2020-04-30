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

### Docker image

There's a minimal Docker image [published on Docker hub](https://hub.docker.com/r/zbrox/rmq_monitor). The size of the image is only 11Mb.

To use it you only need to mount a volume with your config file. Example:

```
docker run -it -v (pwd)/your_config.toml:/config/config.toml --rm zbrox/rmq_monitor:latest
```

