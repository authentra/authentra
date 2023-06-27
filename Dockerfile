FROM rust:1.70.0-alpine AS chef 
# We only pay the installation cost once, 
# it will be cached from the second build onwards
RUN apk add --no-cache musl-dev pkgconfig openssl-dev protoc clang mold
RUN cargo install cargo-chef
WORKDIR /app

FROM chef AS planner
COPY server/ server/
COPY Cargo.lock .
COPY Cargo.toml .
RUN cargo chef prepare --recipe-path recipe.json --bin server

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
# Notice that we are specifying the --target flag!
RUN cargo chef cook --release --target x86_64-unknown-linux-musl --recipe-path recipe.json --bin authentra
COPY server/ server/
COPY Cargo.lock .
COPY Cargo.toml .
RUN cargo build --release --target x86_64-unknown-linux-musl --bin authentra
RUN strip target/x86_64-unknown-linux-musl/release/authentra

FROM scratch
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/authentra /authentra/server
WORKDIR /authentra
CMD [ "./server" ]