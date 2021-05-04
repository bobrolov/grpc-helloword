FROM rust:latest

RUN rustup update
RUN rustup component add rustfmt

WORKDIR /usr/src/app

COPY Cargo.toml Cargo.toml
RUN mkdir src/

