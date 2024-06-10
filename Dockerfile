FROM rust:latest AS builder

RUN \
  if [ "$TARGETARCH" = "amd64" ]; then \
    export TARGET="x86_64-unknown-linux-musl"; \
  elif [ "$TARGETARCH" = "arm64" ]; then \
    export TARGET="aarch64-unknown-linux-musl"; \
  else \
    echo "Unsupported target arch" && exit 1; \
  fi

RUN rustup target add $TARGET
RUN apt update && apt install -y musl-tools musl-dev clang cmake
RUN update-ca-certificates
RUN cargo install sqlx-cli

WORKDIR /chat-app

COPY Cargo.toml .
COPY migrations/ ./migrations
COPY templates/ ./templates
COPY src/ ./src

RUN echo "DATABASE_URL=sqlite://db.sqlite" > .env
RUN touch db.sqlite
RUN sqlx migrate run

RUN cargo build --target $TARGET --release

RUN mkdir out
RUN cp target/$TARGET/release/chat-app out/chat-app
RUN touch out/db.sqlite

FROM scratch

WORKDIR /chat-app

COPY --from=builder /chat-app/out/* ./

EXPOSE 43561
CMD ["/chat-app/chat-app"]
