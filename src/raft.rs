use crate::messages;
use rand::{prelude::thread_rng, Rng};
use std::{collections::HashMap, io::Error};
use tokio::io::{self};
use tokio_seqpacket::UnixSeqpacket;

pub struct Replica<'a> {
    committed_values: HashMap<String, String>,
    id: &'a String,
    sock: UnixSeqpacket,
    colleagues: &'a Vec<String>,
    state: ReplicaState,
    leader: &'a str,
    term: u16,
    log: Vec<LogEntry<'a>>,
    pub election_timeout: f32,
}

pub struct LogEntry<'a> {
    term: u16,
    key: &'a str,
    value: &'a str,
}

#[derive(PartialEq)]
enum ReplicaState {
    Follower,
    Leader,
    Candidate,
}

pub async fn new<'a>(
    replica_id: &'a String,
    colleague_ids: &'a Vec<String>,
) -> Result<Replica<'a>, Error> {
    let conn_attempt = UnixSeqpacket::connect(replica_id).await;
    let mut rng = thread_rng();

    let replica = match conn_attempt {
        Ok(sock) => Ok(Replica {
            election_timeout: rng.gen_range(0.15..0.30),
            log: Vec::new(),
            committed_values: HashMap::new(),
            id: replica_id,
            sock,
            colleagues: colleague_ids,
            state: ReplicaState::Follower,
            leader: "FFFF",
            term: 0,
        }),
        Err(e) => Err(e),
    };

    return replica;
}

impl<'a> Replica<'a> {
    pub fn is_leader(&self) -> bool {
        return matches!(self.state, ReplicaState::Leader);
    }

    pub fn build_body(&self, dst: &str, mid: &str) -> messages::Body {
        messages::Body {
            src: self.id.to_string(),
            dst: dst.to_string(),
            leader: self.leader.to_string(),
            id: mid.to_string(),
        }
    }
    pub async fn read(&self) -> Result<messages::Recv, Error> {
        let mut buf = [0u8; 32768];

        // should be using a timeout on the promise this returns
        let len = self.sock.recv(&mut buf).await?;

        let recv: Result<messages::Recv, serde_json::Error> = serde_json::from_slice(&buf[0..len]);
        return match recv {
            Ok(body) => Ok(body),
            Err(e) => Err(io::Error::from(e)),
        };
    }

    pub async fn send_fail(&self, dst: &str, mid: &str) -> Result<(), Error> {
        let msg: Result<Vec<u8>, serde_json::Error> = serde_json::to_vec(&messages::Send {
            body: self.build_body(dst, mid),
            options: messages::SendOptions::Fail,
        });

        let mut buf = match msg {
            Ok(b) => b,
            Err(e) => return Err(io::Error::from(e)),
        };

        let success = self.sock.send(&mut buf).await;

        match success {
            Ok(_) => Ok(()),
            Err(e) => Err(e),
        }
    }

    pub async fn redirect(&self, dst: &str, mid: &str) -> Result<(), Error> {
        let msg: Result<Vec<u8>, serde_json::Error> = serde_json::to_vec(&messages::Send {
            body: self.build_body(dst, mid),
            options: messages::SendOptions::Redirect,
        });

        let mut buf = match msg {
            Ok(b) => b,
            Err(e) => return Err(io::Error::from(e)),
        };

        let success = self.sock.send(&mut buf).await;

        match success {
            Ok(_) => Ok(()),
            Err(e) => Err(e),
        }
    }

    pub async fn get(&self, key: &str, dst: &str, mid: &str) -> Result<(), Error> {
        let lookup = self.committed_values.get(key);

        match lookup {
            Some(v) => {
                let msg = serde_json::to_vec(&messages::Send {
                    body: self.build_body(dst, mid),
                    options: messages::SendOptions::ReadOk {
                        value: v.to_string(),
                    },
                });

                let mut buf = match msg {
                    Ok(b) => b,
                    Err(e) => return Err(io::Error::from(e)),
                };

                let success = self.sock.send(&mut buf).await;

                match success {
                    Ok(_) => Ok(()),
                    Err(e) => Err(e),
                }
            }
            None => self.send_fail(dst, mid).await,
        }
    }

    pub fn as_least_as_long(&self, other_last_log_index: u16, other_last_log_term: u16) -> bool {
        let last_log_entry = self.log.last();

        match last_log_entry {
            Some(entry) => {
                entry.term <= other_last_log_term && self.log.len() <= other_last_log_index
            }
            None => true,
        }
    }
}
