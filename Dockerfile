FROM rust:latest as builder

WORKDIR /usr/src/app

COPY Cargo.toml Cargo.lock ./

COPY src ./src

RUN cargo build --release

FROM gcr.io/distroless/cc-debian12

WORKDIR /usr/local/bin

COPY --from=builder /usr/src/app/target/release/ferrugem .

EXPOSE 9999

ENTRYPOINT ["./ferrugem"]
