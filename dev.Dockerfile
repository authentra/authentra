FROM rust:1.68.0-alpine AS chef 
# We only pay the installation cost once, 
# it will be cached from the second build onwards
ENV CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse
RUN apk add --no-cache musl-dev pkgconfig openssl-dev protoc clang mold
RUN cargo install cargo-chef
WORKDIR /app

FROM chef AS planner
COPY model/ model/
COPY storage/ storage/
COPY server/ server/
COPY policy-engine/ policy-engine/
COPY Cargo.lock .
COPY Cargo.toml .
ENV CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse
RUN cargo chef prepare --recipe-path recipe.json --bin server

FROM node:18-alpine AS frontend-build
COPY frontend .
RUN npm install
RUN npm run build

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
ENV CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse
# Notice that we are specifying the --target flag!
RUN cargo chef cook --target x86_64-unknown-linux-musl --recipe-path recipe.json --bin authentra
COPY model/ model/
COPY storage/ storage/
COPY server/ server/
COPY policy-engine/ policy-engine/
COPY Cargo.lock .
COPY Cargo.toml .
RUN cargo build --target x86_64-unknown-linux-musl --bin authentra
RUN strip target/x86_64-unknown-linux-musl/debug/authentra

FROM scratch AS runtime
COPY --from=builder /app/target/x86_64-unknown-linux-musl/debug/authentra /authentra/server
COPY --from=frontend-build dist/ /authentra/dist/
WORKDIR /authentra
CMD [ "./server" ]