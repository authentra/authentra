FROM rust:1.67.1-alpine AS chef 
# We only pay the installation cost once, 
# it will be cached from the second build onwards
RUN apk add --no-cache musl-dev pkgconfig openssl-dev protoc clang mold
RUN cargo install cargo-chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json --bin server

FROM node:18-alpine AS frontend-build
COPY frontend .
RUN npm install
RUN npm run build

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
# Notice that we are specifying the --target flag!
RUN cargo chef cook --release --target x86_64-unknown-linux-musl --recipe-path recipe.json --bin authust_server
COPY model/ model/
COPY storage/ storage/
COPY server/ server/
COPY policy-engine/ policy-engine/
COPY Cargo.lock .
COPY Cargo.toml .
RUN cargo build --release --target x86_64-unknown-linux-musl --bin authust_server

FROM scratch AS runtime
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/authust_server /authust/server
COPY --from=frontend-build dist/ /authust/dist/
WORKDIR /authust
CMD [ "./server" ]