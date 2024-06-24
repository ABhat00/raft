FROM rust:latest as builder

RUN apt-get update -y \
    && apt-get install -y python2 \
    && apt-get install -y pip \
    && ln -s /usr/bin/python2 /usr/bin/python

RUN pip install --upgrade pip setuptools wheel


RUN cd / &&\
    cargo new raft 
WORKDIR /raft

COPY Cargo.toml Cargo.lock ./
RUN cargo build
RUN rm -r src/

COPY tests/ tests/
COPY run.py test.py ./
COPY ./src src
RUN touch src/main.rs && cargo build

RUN mv target/debug/raft 3700kvstore
