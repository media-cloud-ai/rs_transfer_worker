FROM rust:1.47-buster as builder

ADD . /src
WORKDIR /src

RUN apt-get update && \
    apt-get install -y \
        libssl-dev \
        && \
    cargo build --verbose --release && \
    cargo install --path .

FROM debian:buster
COPY --from=builder /usr/local/cargo/bin/transfer_worker /usr/bin

RUN apt update && \
    apt install -y \
        libssl1.1 \
        ca-certificates

ENV AMQP_QUEUE job_transfer
CMD transfer_worker
