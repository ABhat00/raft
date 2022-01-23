use crate::messages;
use std::io::Error;
use tokio::io::{self};
use tokio_seqpacket::UnixSeqpacket;

pub struct Replica<'a> {
    id: &'a String,
    sock: UnixSeqpacket,
}

pub async fn new<'a>(replica_id: &'a String) -> Result<Replica<'a>, Error> {
    let conn_attempt = UnixSeqpacket::connect(replica_id).await;

    let replica = match conn_attempt {
        Ok(sock) => Ok(Replica {
            id: replica_id,
            sock,
        }),
        Err(e) => Err(e),
    };

    return replica;
}

impl<'a> Replica<'a> {
    pub async fn read(&self) -> Result<messages::Body, Error> {
        let mut buf = [0u8; 32768];

        let len = self.sock.recv(&mut buf).await?;

        let recv: Result<messages::Body, serde_json::Error> = serde_json::from_slice(&buf[0..len]);
        return match recv {
            Ok(body) => Ok(body),
            Err(e) => Err(io::Error::from(e)),
        };
    }
}
