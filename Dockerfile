# BUILD CONTAINER

FROM rust:1.85 AS build

ENV CARGO_NET_GIT_FETCH_WITH_CLI=true

RUN USER=root cargo new --bin pagenine

# Build dependencies separately for layer caching.
WORKDIR /pagenine
COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml
RUN cargo build --release

# Clean the temporary project.
RUN rm src/*.rs
RUN rm ./target/release/deps/pagenine*
ADD . ./

# Do the actual build.
RUN cargo build --release


# RUNTIME CONTAINER

FROM debian:bookworm-slim

ENV PAGENINE_BOARD=
ENV PAGENINE_TITLE=
ENV PAGENINE_NO_BUMP_LIMIT=false
ENV PAGENINE_PUSHOVER_APPLICATION_API_TOKEN=
ENV PAGENINE_PUSHOVER_USER_KEY=

RUN apt-get update && apt-get install -y \
    ca-certificates \
    openssl \
    && rm -rf /var/lib/apt/lists/*

COPY --from=build /pagenine/target/release/pagenine .

ENTRYPOINT ["./pagenine"]
