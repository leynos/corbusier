FROM rust:1.94-slim-bookworm AS build
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential libpq-dev perl pkg-config && rm -rf /var/lib/apt/lists/*
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY src/ src/
COPY migrations/ migrations/
RUN cargo build --release --bin corbusier

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends \
    libpq5 ca-certificates && rm -rf /var/lib/apt/lists/*
RUN groupadd -r corbusier && useradd -r -g corbusier corbusier
COPY --from=build /build/target/release/corbusier /usr/local/bin/corbusier
USER corbusier
EXPOSE 8080
ENTRYPOINT ["corbusier"]
