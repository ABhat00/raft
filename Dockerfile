FROM rust:latest 

RUN apt-get update -y \
    && apt-get install -y python2 \
    && apt-get install -y pip \
    && ln -s /usr/bin/python2 /usr/bin/python

RUN pip install --upgrade pip setuptools wheel


RUN cd / &&\
    cargo new raft 
WORKDIR /raft

ADD Cargo.toml /raft/Cargo.toml
RUN cargo build
RUN rm src/*.rs

COPY tests/ tests/
COPY run.py test.py .
COPY src/ src/
RUN cargo clean && cargo build

RUN mv target/debug/raft 3700kvstore

CMD ["./run.py", "tests/simple-1.json"]
