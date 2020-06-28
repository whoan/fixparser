FROM rust:1-slim-buster

WORKDIR /app

COPY Cargo.toml /app/
# create empty lib.rs to allow "build" command to download and compile dependencies in a separate layer.
# note that I am not building the actual code yet
RUN mkdir /app/src && \
  echo > /app/src/lib.rs && \
  cargo build && \
  rm -r src/

# Avoid test errors for having two linkages
RUN sed -i '/crate-type/d' Cargo.toml

# build actual code
COPY src /app/src
RUN cargo build

# test
COPY tests /app/tests
RUN cargo test
