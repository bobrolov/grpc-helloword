FROM rust as planner
WORKDIR app
RUN cargo install cargo-chef
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM rust as cacher
WORKDIR app
RUN cargo install cargo-chef
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

FROM rust as builder
WORKDIR app
COPY . .
COPY --from=cacher /app/target target
COPY --from=cacher /usr/local/cargo /usr/local/cargo
RUN rustup component add rustfmt
RUN cargo build --release --bin grpc-server --bin grpc-client --bin postgres-test

FROM rust as runtime
WORKDIR app
#COPY --from=builder /app/target/release/grpc-server /usr/local/bin
#COPY --from=builder /app/target/release/grpc-client /usr/local/bin
#COPY --from=builder /app/target/release/postgres-test /usr/local/bin
COPY --from=builder /app/target/release/ /usr/local/bin
#ENTRYPOINT ["./usr/local/bin/grpc-server"]

