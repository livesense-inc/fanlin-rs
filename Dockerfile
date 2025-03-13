FROM rust:1-bookworm AS builder
WORKDIR /usr/src/app
COPY . .
RUN cargo build --profile container

FROM debian:bookworm AS allocator
ARG VERSION=5.3.0
RUN set -eux; \
  apt-get update; \
  apt-get install -y --no-install-recommends ca-certificates wget bzip2 make build-essential; \
  wget https://github.com/jemalloc/jemalloc/releases/download/$VERSION/jemalloc-$VERSION.tar.bz2; \
  tar -jxvf jemalloc-$VERSION.tar.bz2; \
  cd jemalloc-$VERSION; \
  ./configure; \
  make; \
  make install

# https://github.com/GoogleContainerTools/distroless
# https://console.cloud.google.com/gcr/images/distroless/GLOBAL
FROM gcr.io/distroless/cc-debian12:nonroot-amd64
COPY --from=builder /usr/src/app/target/container/fanlin-rs /usr/local/bin/fanlin-rs
COPY --from=allocator /usr/local/lib/libjemalloc.so.2 /usr/local/lib/
ENV LD_PRELOAD=/usr/local/lib/libjemalloc.so.2
ENTRYPOINT ["/usr/local/bin/fanlin-rs"]
CMD ["--help"]
