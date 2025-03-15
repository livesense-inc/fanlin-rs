FROM rust:1-bookworm AS apps
WORKDIR /usr/src/app
COPY . .
RUN cargo build --profile container

FROM debian:bookworm AS deps
ARG VERSION=5.3.0
RUN set -eux; \
  apt-get update; \
  apt-get install -y --no-install-recommends \
    ca-certificates \
    wget \
    bzip2 \
    make \
    build-essential \
    liblcms2-dev \
    ; \
  wget https://github.com/jemalloc/jemalloc/releases/download/$VERSION/jemalloc-$VERSION.tar.bz2; \
  tar -jxvf jemalloc-$VERSION.tar.bz2; \
  cd jemalloc-$VERSION; \
  ./configure; \
  make; \
  make install

# https://github.com/GoogleContainerTools/distroless
# https://console.cloud.google.com/gcr/images/distroless/GLOBAL
FROM gcr.io/distroless/cc-debian12:nonroot-amd64
COPY --from=apps --chmod=755 /usr/src/app/target/container/fanlin-rs /usr/local/bin/fanlin
COPY --from=apps --chmod=644 /usr/src/app/profiles/default.icc       /var/lib/fanlin/
COPY --from=deps --chmod=644 /usr/local/lib/libjemalloc.so.2         /lib/x86_64-linux-gnu/
COPY --from=deps --chmod=644 /lib/x86_64-linux-gnu/liblcms2.so.*     /lib/x86_64-linux-gnu/
COPY --from=deps --chmod=644 /lib/x86_64-linux-gnu/libssl.so.*       /lib/x86_64-linux-gnu/
COPY --from=deps --chmod=644 /lib/x86_64-linux-gnu/libcrypto.so.*    /lib/x86_64-linux-gnu/
ENV LD_PRELOAD=/lib/x86_64-linux-gnu/libjemalloc.so.2
ENTRYPOINT ["/usr/local/bin/fanlin"]
CMD ["--help"]
