use crate::messages;
use core::panic;
use futures::task;
use rand::prelude::thread_rng;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    io::Error,
    task::{Context, Poll},
    time::{Duration, Instant},
};
use tokio::io::{self};
use tokio_seqpacket::UnixSeqpacket;

pub struct Replica<'a> {
    // values from the log up to this point have been committed (persisted)
    pub commit_index: u16,

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
    pub leader: String,
    pub term: u16,
    pub log: Vec<LogEntry>,
    pub election_timeout: Duration,
    // if an entry exists in the vote_history, then this replica has
    // already voted for someone in that term
    pub vote_history: HashSet<u16>,
    // how many people have voted for me in each term?
    pub vote_tally: HashMap<u16, u16>,
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
            leader: "FFFF".to_string(),
            term: 0,
            commit_index: 0,
        }),
        Err(e) => Err(e),
    };

    return replica;
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct LogEntry {
    term: u16,
    key: String,
    value: String,
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

    pub fn commit(&mut self, k: String, v: String) -> Option<String> {
        self.committed_values.insert(k, v)
    }

    // This should poll the socket - once i figure out how contexts work, this
    // is gonna be super easy
    pub async fn read(&self) -> Poll<messages::Recv> {
        let mut buf = [0u8; 32768];

        // This is jank - the idea is that the typical pattern
        // is for the waker to be called when the socket is ready to be read from
        // But what I want is for the task to continue - I want the election_timer to keep running
        // maybe i should spawn a timer thread?
        // see: https://tokio.rs/tokio/tutorial/async (Search for timer thread)
        let waker = task::noop_waker();
        let mut cx = Context::from_waker(&waker);

        // should be using a timeout on the promise this returns
        let status = self.sock.poll_recv(&mut cx, &mut buf);

        match status {
            Poll::Ready(res) => {
                let len = match res {
                    Ok(len) => len,
                    Err(_) => panic!("Unrecoverable read failure"),
                };

                let recv: messages::Recv = serde_json::from_slice(&buf[0..len]).unwrap();
                Poll::Ready(recv)
            }
            Poll::Pending => Poll::Pending,
        }
    }

    pub async fn start_election(&mut self) -> Result<(), Error> {
        // This is where we start a leader election
        // set my state to candidate
        self.state = ReplicaState::Candidate;
        self.reset_time_of_last_heartbeat();
        // increment my term
        self.term += 1;
        // vote for myself
        self.vote_tally.insert(self.term, 1);
        // mark that I have voted in this term
        self.vote_history.insert(self.term);
        // request votes from replicas
        self.request_vote().await
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

    pub fn should_accept_leader(&mut self, other_term: u16, leader_id: String) {
        if matches!(self.state, ReplicaState::Candidate)
            || matches!(self.state, ReplicaState::Follower)
        {
            if other_term >= self.term {
                self.state = ReplicaState::Follower;
                self.leader = leader_id;
            }
        }
    }

    pub async fn redirect(&self, mid: &str) -> Result<(), Error> {
        self.send_msg(&messages::Send {
            body: self.build_body(&self.leader, mid),
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
        self.vote_history.insert(term);
        self.send_msg(&messages::Send {
            body: self.build_body(dst, "mid"),
            options: messages::SendOptions::Vote { term },
        })
        .await
    }

    pub async fn send_heartbeat(&self, term: u16) -> Result<(), Error> {
        let last_log_term = match self.log.last() {
            Some(entry) => entry.term,
            None => 0,
        };

        let last_log_index = if self.log.len() > 0 {
            self.log.len() - 1
        } else {
            // I'm never going to send an append entry if there's nothing in my log
            panic!("I should never send an append entry if there's nothing in my log");
        };

        self.send_msg(&messages::Send {
            body: self.build_body("FFFF", "append_entry"),
            options: messages::SendOptions::AppendEntry {
                term: term,
                leader_id: self.id.to_string(),
                prev_log_index: last_log_index as u16,
                prev_log_term: last_log_term,
                // heartbeat should send an empty vector
                entries: Vec::new(),
                leader_commit_index: self.commit_index,
            },
        })
        .await
    }

    pub fn reset_time_of_last_heartbeat(&mut self) {
        self.time_of_last_heartbeat = Instant::now();
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

                // index starts at 0
                return (self.log.len() - 1) <= other_last_log_index.into();
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
