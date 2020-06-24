# This image is made for building static binaries linked against musl
# It includes OpenSSL compiled against musl-gcc
# I think clux/muslrust:latest uses nightly
FROM clux/muslrust AS build
WORKDIR /usr/src

# Update CA Certificates
RUN apt update -y && apt install -y ca-certificates
RUN update-ca-certificates

# Build dependencies and rely on cache if Cargo.toml
# or Cargo.lock haven't changed
RUN USER=root cargo new rmq_monitor
WORKDIR /usr/src/rmq_monitor
COPY Cargo.toml Cargo.lock ./
RUN cargo build --target x86_64-unknown-linux-musl --release

# Copy the source and build the application
COPY src ./src
RUN cargo install --target x86_64-unknown-linux-musl --path .

# Second stage
FROM scratch

# Copy over from first stage the statically-linked binary and CA certificates
COPY --from=build /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/
COPY --from=build /root/.cargo/bin/rmq_monitor .
USER 1000

# need to mount the config as a volume
# example run:
# docker run -it -v (pwd)/your_config.toml:/config/config.toml --rm rmq_monitor:latest
CMD ["./rmq_monitor", "--config", "/config/config.toml"]