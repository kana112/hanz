# ------------------------------
# Stage 1. Build an app
# ------------------------------
FROM rust:1.96.0 AS builder

WORKDIR /app
COPY . .
RUN cargo build --release

# ------------------------------
# Stage 2. Build for runtime
# ------------------------------
FROM dhi.io/debian-base:trixie

ARG GIT_REVISION
ARG BUILD_DATE
ARG VERSION

LABEL org.opencontainers.image.title="hanz" \
      org.opencontainers.image.description="CLI tool for finding potentially unnecessary files" \
      org.opencontainers.image.url="https://kana112.github.io/hanz/" \
      org.opencontainers.image.source="https://github.com/kana112/hanz" \
      org.opencontainers.image.version=${VERSION} \
      org.opencontainers.image.revision=${GIT_REVISION} \
      org.opencontainers.image.created=${BUILD_DATE} \
      org.opencontainers.image.licenses="GPL-3.0-only"

COPY --from=builder /app/target/release/hanz /app/hanz
WORKDIR /opt

ENTRYPOINT [ "/app/hanz" ]
