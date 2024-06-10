# Use Docker build arguments to set the target architecture
ARG TARGETARCH
FROM rust:latest AS builder

# Set the appropriate target based on the architecture
ENV RUST_TARGET=x86_64-unknown-linux-musl
RUN if [ "$TARGETARCH" = "arm64" ]; then \
        RUST_TARGET=aarch64-unknown-linux-musl; \
    fi

# Install dependencies
RUN apt update && apt install -y \
    musl-tools \
    musl-dev \
    clang \
    cmake \
    gcc \
    gcc-aarch64-linux-gnu \
    gcc-x86-64-linux-gnu
RUN update-ca-certificates
RUN rustup target add ${RUST_TARGET}
RUN cargo install sqlx-cli

WORKDIR /chat-app

COPY Cargo.toml .
COPY migrations/ ./migrations
COPY templates/ ./templates
COPY src/ ./src

RUN echo "DATABASE_URL=sqlite://db.sqlite" > .env
RUN touch db.sqlite
RUN sqlx migrate run

# Build the project
RUN cargo build --target ${RUST_TARGET} --release

RUN mkdir out
RUN cp target/${RUST_TARGET}/release/chat-app out/chat-app
RUN touch out/db.sqlite

FROM scratch

WORKDIR /chat-app

COPY --from=builder /chat-app/out/* ./

EXPOSE 43561
CMD ["/chat-app/chat-app"]
