FROM rust:1-bookworm AS builder
WORKDIR /usr/src/app
COPY . .
RUN cargo build --profile container

# https://github.com/GoogleContainerTools/distroless
# https://console.cloud.google.com/gcr/images/distroless/GLOBAL
FROM gcr.io/distroless/cc-debian12:nonroot-amd64
COPY --from=builder /usr/src/app/target/container/fanlin-rs /usr/local/bin/fanlin-rs
ENTRYPOINT ["/usr/local/bin/fanlin-rs"]
CMD ["--help"]
