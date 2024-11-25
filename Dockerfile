# Etapa 1: Construcción
FROM rust:1.82 AS builder
RUN apt-get update && apt-get install --no-install-recommends -y build-essential libprotobuf-dev protobuf-compiler cmake \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY . .
RUN cargo install --locked --path .

# Ejecución
FROM debian:bookworm-slim

RUN apt-get update && apt-get install --no-install-recommends -y build-essential libprotobuf-dev protobuf-compiler cmake ca-certificates \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app

# Copia el binario desde la etapa de construcción
COPY --from=builder /usr/local/cargo/bin/sui-swap /usr/local/bin/sui-swap
COPY ./tokens.json .

EXPOSE 8080

CMD ["sui-swap", "-s", "0.0.0.0:8080"]