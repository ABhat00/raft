use crate::messages;
use rand::prelude::thread_rng;
use rand::Rng;
use std::{
    collections::{HashMap, HashSet},
    io::Error,
    task::Poll,
    time::{Duration, Instant},
};
use tokio::io::{self};
use tokio_seqpacket::UnixSeqpacket;

pub struct Replica<'a> {
    // these are the values that are persisted
    committed_values: HashMap<String, String>,
    id: &'a String,
    pub sock: UnixSeqpacket,
    // This is the time of the last append entry we've received - every time I get
    // an append entry message, I reset the time_of_last_heartbeat to SystemTime::now
    // If  self.time_of_last_heartbeat.elapsed(election_timeout)> election_timeout, we need to do something
    pub time_of_last_heartbeat: Instant,
    pub colleagues: &'a Vec<String>,
    pub state: ReplicaState,
    leader: &'a str,
    pub term: u16,
    log: Vec<LogEntry<'a>>,
    pub election_timeout: Duration,
    // if an entry exists in the vote_history, then this replica has
    // already voted for someone in that term
    pub vote_history: HashSet<u16>,
    // how many people have voted for me in each term?
    pub vote_tally: HashMap<u16, u16>,
}

pub struct LogEntry<'a> {
    term: u16,
    key: &'a str,
    value: &'a str,
}

#[derive(PartialEq)]
pub enum ReplicaState {
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
    let ms: u64 = rng.gen_range(150..=300);

    let replica = match conn_attempt {
        Ok(sock) => Ok(Replica {
            time_of_last_heartbeat: Instant::now(),
            vote_tally: HashMap::new(),
            vote_history: HashSet::new(),
            // election timeout in milliseconds
            election_timeout: Duration::from_millis(ms),
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
            mid: mid.to_string(),
        }
    }

    // This should poll the socket - once i figure out how contexts work, this
    // is gonna be super easy
    pub async fn read(&self) -> Poll<messages::Recv> {
        let mut buf = [0u8; 32768];

        // should be using a timeout on the promise this returns
        let len = self.sock.recv(&mut buf).await?;

        let recv: Result<messages::Recv, serde_json::Error> = serde_json::from_slice(&buf[0..len]);
        return match recv {
            Ok(body) => Ok(body),
            Err(e) => Err(io::Error::from(e)),
        };
    }

    pub async fn start_election(&mut self) -> () {
        // This is where we start a leader election
        // set my state to candidate
        self.state = ReplicaState::Candidate;
        // increment my term
        self.term += 1;
        // vote for myself
        self.vote_tally.insert(self.term, 1);
        // mark that I have voted in this term
        self.vote_history.insert(self.term);
        // requeset votes from replicas
        self.request_vote().await;
    }

    pub async fn send_fail(&self, dst: &str, mid: &str) -> Result<(), Error> {
        self.send_msg(&messages::Send {
            body: self.build_body(dst, mid),
            options: messages::SendOptions::Fail,
        })
        .await
    }

    pub async fn request_vote(&self) -> Result<(), Error> {
        let last_log_term = match self.log.last() {
            Some(entry) => entry.term,
            None => 0,
        };

        self.send_msg(&messages::Send {
            body: self.build_body("FFFF", "mid"),
            options: messages::SendOptions::RequestVote {
                term: self.term,
                last_log_index: self.log.len() as u16,
                last_log_term: last_log_term,
            },
        })
        .await
    }

    pub async fn redirect(&self, mid: &str) -> Result<(), Error> {
        self.send_msg(&messages::Send {
            body: self.build_body(self.leader, mid),
            options: messages::SendOptions::Redirect,
        })
        .await
    }

    pub async fn get(&self, key: &str, dst: &str, mid: &str) -> Result<(), Error> {
        let lookup = self.committed_values.get(key);

        match lookup {
            Some(v) => {
                self.send_msg(&messages::Send {
                    body: self.build_body(dst, mid),
                    options: messages::SendOptions::ReadOk {
                        value: v.to_string(),
                    },
                })
                .await
            }
            None => self.send_fail(dst, mid).await,
        }
    }

    pub async fn vote(&self, dst: &str, term: u16) -> Result<(), Error> {
        self.send_msg(&messages::Send {
            body: self.build_body(dst, "mid"),
            options: messages::SendOptions::Vote { term },
        })
        .await
    }

    // This tells us if the other log is at least as long as ours
    pub fn as_least_as_long(&self, other_last_log_index: u16, other_last_log_term: u16) -> bool {
        /*
            Per the RAFT paper
            "If the logs have last entries with different terms, then
            the log with the later term is more up-to-date. If the logs
            end with the same term, then whichever log is longer is
            more up-to-date."
        */
        let last_log_entry = self.log.last();

        match last_log_entry {
            Some(entry) => {
                if entry.term != other_last_log_term {
                    return entry.term <= other_last_log_term;
                }

                return self.log.len() <= other_last_log_index.into();
            }
            // If our log is empty, then our last log index is 0, which means
            // everyone is at least as long as us
            None => true,
        }
    }

    pub fn election_timeout_elapsed(&self) -> bool {
        return self.time_of_last_heartbeat.elapsed() > self.election_timeout;
    }

    // abstract away sending messages
    async fn send_msg(&self, msg: &messages::Send) -> Result<(), Error> {
        let as_bytes: Result<Vec<u8>, serde_json::Error> = serde_json::to_vec(msg);

        let mut buf = match as_bytes {
            Ok(b) => b,
            Err(e) => return Err(io::Error::from(e)),
        };

        let success = self.sock.send(&mut buf).await;

        match success {
            Ok(_) => Ok(()),
            Err(e) => Err(e),
        }
    }
}
