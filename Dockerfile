ARG RUST_VERSION=1.88
ARG APP_BIN=Jasper

################################################################################
# app building stage
################################################################################
FROM rust:${RUST_VERSION}-bookworm AS builder
ARG APP_BIN

RUN apt-get update && apt-get install -y --no-install-recommends \
    cmake \
    pkg-config \
    libopus-dev \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY src ./src

RUN cargo build --locked --release --bin ${APP_BIN}

################################################################################
# app running stage
################################################################################
FROM debian:bookworm-slim AS runtime
ARG APP_BIN

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libopus0 \
    && rm -rf /var/lib/apt/lists/*

RUN useradd --create-home --uid 10001 appuser

WORKDIR /app

COPY --from=builder /app/target/release/${APP_BIN} /usr/local/bin/jasper

USER appuser

CMD ["/usr/local/bin/jasper"]
