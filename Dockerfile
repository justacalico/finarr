# syntax=docker/dockerfile:1

# Finarr all-in-one image: Flutter web bundle -> Rust binary -> slim
# runtime. The backend embeds frontend/dist/ at compile time via
# include_dir!, so the web UI ships inside the single binary.

FROM debian:bookworm-slim AS frontend
ARG FLUTTER_VERSION=3.44.6
RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        ca-certificates \
        curl \
        git \
        unzip \
        xz-utils \
    && rm -rf /var/lib/apt/lists/* \
    && git clone --depth 1 --branch "$FLUTTER_VERSION" \
        https://github.com/flutter/flutter.git /opt/flutter
ENV PATH="/opt/flutter/bin:$PATH"
RUN flutter precache --web
WORKDIR /app
COPY flutter/pubspec.yaml flutter/pubspec.* flutter/
RUN cd flutter && flutter pub get
COPY flutter/ flutter/
COPY scripts/build-flutter.sh scripts/build-flutter.sh
RUN bash scripts/build-flutter.sh

FROM rust:slim-bookworm AS backend
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY build.rs ./
COPY src/ src/
COPY migrations/ migrations/
COPY --from=frontend /app/frontend/dist/ frontend/dist/
RUN cargo build --release --locked

FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --create-home --shell /usr/sbin/nologin finarr \
    && install -d -o finarr -g finarr /data
COPY --from=backend /app/target/release/finarr /usr/local/bin/finarr
COPY scripts/docker-entrypoint.sh /usr/local/bin/entrypoint
WORKDIR /home/finarr
ENV FINARR_DATA_DIR=/data \
    FINARR_HOST=0.0.0.0 \
    FINARR_PORT=8787
EXPOSE 8787
EXPOSE 4242
EXPOSE 4242/udp
VOLUME ["/data"]
ENTRYPOINT ["entrypoint"]
