FROM rust:1-slim-buster

WORKDIR /app

# create empty main.rs to allow "build" command to download and compile dependencies in a separate layer.
# note that I am not building the actual code yet
COPY Cargo.toml /app/
RUN \
  mkdir /app/src && \
  echo 'fn main() {}' > /app/src/main.rs && \
  cargo build && \
  rm -r src/

COPY src /app/src
RUN cargo build

COPY tests /app/tests
RUN cargo test
