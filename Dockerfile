FROM rust:latest AS builder

RUN rustup target add x86_64-unknown-linux-musl
RUN apt update && apt install -y musl-tools musl-dev
RUN update-ca-certificates
RUN cargo install sqlx-cli

WORKDIR /chat-app

COPY Cargo.toml .env .
COPY migrations/ ./migrations
COPY templates/ ./templates
COPY src/ ./src
RUN touch db.sqlite
RUN sqlx migrate run

RUN cargo build --target x86_64-unknown-linux-musl --release

RUN mkdir out
RUN cp target/x86_64-unknown-linux-musl/release/chat-app out/chat-app
RUN touch out/db.sqlite

FROM scratch

WORKDIR /chat-app

COPY --from=builder /chat-app/out/* ./

EXPOSE 43561
CMD ["/chat-app/chat-app"]
