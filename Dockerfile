FROM rust:1.33-stretch as builder

ADD . ./

RUN apt update && \
    apt install -y libssl-dev && \
    cargo build --verbose --release && \
    cargo install --path .

FROM debian:stretch
COPY --from=builder /usr/local/cargo/bin/ftp_worker /usr/bin

RUN apt update && apt install -y libssl1.1 ca-certificates

ENV AMQP_QUEUE job_ftp
CMD ftp_worker
