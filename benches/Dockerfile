FROM rust:1.56-buster as builder

ADD . /src
WORKDIR /src

RUN apt-get update && \
    apt-get install -y \
        libssl-dev \
        && \
    cargo build --verbose --release

ENTRYPOINT ["cargo", "bench"]
