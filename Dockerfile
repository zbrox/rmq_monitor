# Need to build with musl, this image comes with the musl target
# and openssl linked against musl
# It doesn't have 1.43.0 so we gonna use the default latest
# which I think uses nightly
FROM clux/muslrust AS build
WORKDIR /usr/src

# Build dependencies and rely on cache if Cargo.toml
# or Cargo.lock haven't changed
RUN USER=root cargo new rmq_monitor
WORKDIR /usr/src/rmq_monitor
COPY Cargo.toml Cargo.lock ./
RUN cargo build --target x86_64-unknown-linux-musl --release

# Copy the source and build the application.
COPY src ./src
RUN cargo install --target x86_64-unknown-linux-musl --path .

# Copy the statically-linked binary into a scratch container.
FROM scratch

COPY --from=build /root/.cargo/bin/rmq_monitor .
USER 1000

# need to mount the config as a volume
# example run:
# docker run -it -v (pwd)/your_config.toml:/config/config.toml --rm rmq_monitor:latest
CMD ["./rmq_monitor", "--config", "/config/config.toml"]