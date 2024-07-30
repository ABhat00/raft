use crossbeam::select;
use crossbeam_channel::{Receiver, Sender};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    io::Error,
    time::{Duration, Instant},
};
use tokio::io::{self};

use super::{
    loggers,
    messages::{
        self,
        replica_messages::{RaftReceive, RaftSend},
    },
    orchestrator::MessageDestination,
    timer::Timer,
    RaftError,
};

pub struct Replica<'a> {
    // values from the log up to this point have been committed (persisted)
    commit_index: u16,
    response_tx: Sender<(RaftSend, MessageDestination)>,
    message_rx: Receiver<RaftReceive>,
    timer: Timer,
    // these are the values that are persisted
    committed_values: HashMap<String, String>,
    id: &'a str,

    colleagues: &'a Vec<String>,
    state: ReplicaState,
    leader: String,
    term: u16,
    log: Vec<LogEntry>,
    last_applied: u16,

    // if an entry exists in the vote_history, then this replica has
    // already voted for someone in that term
    vote_history: HashSet<u16>,
    // how many people have voted for me in each term?
    vote_tally: HashMap<u16, u16>,
    logger: Box<dyn loggers::Logger>,
}

#[derive(PartialEq)]
pub enum ReplicaState {
    Follower,
    Leader,
    Candidate,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct LogEntry {
    term: u16,
    key: String,
    value: String,
}

pub trait RaftReplica<'a> {
    async fn new(
        replica_id: &'a str,
        colleague_ids: &'a Vec<String>,
        election_timeout: Duration,
        tx: Sender<(RaftSend, MessageDestination)>,
        rx: Receiver<RaftReceive>,
    ) -> Self;

    async fn run_replica(&mut self) -> Result<(), RaftError>;
}

impl<'a> RaftReplica<'a> for Replica<'a> {
    async fn new(
        replica_id: &'a str,
        colleague_ids: &'a Vec<String>,
        election_timeout: Duration,
        tx: Sender<(RaftSend, MessageDestination)>,
        rx: Receiver<RaftReceive>,
    ) -> Replica<'a> {
        Replica {
            timer: Timer::new(election_timeout),
            vote_tally: HashMap::new(),
            vote_history: HashSet::new(),
            response_tx: tx,
            message_rx: rx,
            log: Vec::new(),
            committed_values: HashMap::new(),
            id: replica_id,
            colleagues: colleague_ids,
            state: ReplicaState::Follower,
            leader: "FFFF".to_string(),
            term: 0,
            commit_index: 0,
            last_applied: 0,
            logger: Box::new(loggers::new_file_logger(format!("logs/{}", replica_id))),
        }
    }

    async fn run_replica(&mut self) -> Result<(), RaftError> {
        loop {
            select! {
                // if we get a message, handle it
                recv(self.message_rx) -> msg => (),
                // if we get a message from the
                recv(self.timer.get_rx()) -> msg => ()
            }
        }
    }
}

impl<'a> Replica<'a> {
    pub fn is_leader(&self) -> bool {
        matches!(self.state, ReplicaState::Leader)
    }

    fn build_body(&self, dst: &str, mid: &str) -> messages::Body {
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

    pub async fn read(&self) -> messages::ServerMessage {
        let mut buf = [0u8; 32768];

        self.sock
            .recv(&mut buf)
            .await
            // This is strange, i don't want to map to an io error here
            .and_then(|len| serde_json::from_slice(&buf[0..len]).map_err(io::Error::from))
            .unwrap()
    }

    pub async fn start_election(&mut self) -> Result<(), Error> {
        // This is where we start a leader election
        // set my state to candidate
        self.reset_time_of_last_heartbeat();
        self.state = ReplicaState::Candidate;
        // increment my term
        self.term += 1;
        self.logger
            .log(format!("Starting an election in term {}", self.term))?;
        // vote for myself
        self.vote_tally.insert(self.term, 1);
        // mark that I have voted in this term
        self.vote_history.insert(self.term);
        // request votes from replicas
        self.request_vote().await
    }

    pub async fn send_fail(&self, dst: &str, mid: &str) -> Result<(), Error> {
        self.send_msg(&messages::ClientMessage {
            body: self.build_body(dst, mid),
            options: messages::ClientOptions::Fail,
        })
        .await?;

        Ok(())
    }

    pub async fn request_vote(&self) -> Result<(), Error> {
        let last_log_term = match self.log.last() {
            Some(entry) => entry.term,
            None => 0,
        };

        self.send_msg(&messages::ServerMessage {
            body: self.build_body("FFFF", "mid"),
            options: messages::ServerOptions::RequestVote {
                term: self.term,
                last_log_index: self.log.len() as u16,
                last_log_term,
            },
        })
        .await?;

        Ok(())
    }

    pub fn should_accept_leader(&mut self, other_term: u16) -> bool {
        // This is dumb
        // what happens if two nodes both think that they're leaders, and one
        // gets an append entry from the other

        // I don't know if this first ! matches statement should be here
        !matches!(self.state, ReplicaState::Leader) && other_term >= self.term
    }

    pub async fn redirect(&self, dst: &str, mid: &str) -> Result<(), Error> {
        self.send_msg(&messages::ClientMessage {
            body: self.build_body(dst, mid),
            options: messages::ClientOptions::Redirect,
        })
        .await?;

        Ok(())
    }

    pub async fn get(&self, key: &str, dst: &str, mid: &str) -> Result<(), Error> {
        let lookup = self.committed_values.get(key);

        match lookup {
            Some(v) => {
                self.send_msg(&messages::ClientMessage {
                    body: self.build_body(dst, mid),
                    options: messages::ClientOptions::ReadOk {
                        value: v.to_string(),
                    },
                })
                .await?;
                Ok(())
            }
            None => self.send_fail(dst, mid).await,
        }
    }

    pub async fn send_ok(&self, dst: &str, mid: &str) -> Result<(), Error> {
        self.send_msg(&messages::ClientMessage {
            body: self.build_body(dst, mid),
            options: messages::ClientOptions::WriteOK,
        })
        .await?;

        Ok(())
    }

    pub async fn vote(&mut self, dst: &str, term: u16) -> Result<(), Error> {
        self.vote_history.insert(term);
        self.send_msg(&messages::ServerMessage {
            body: self.build_body(dst, &format!("vote for {}", dst)),
            options: messages::ServerOptions::Vote { term },
        })
        .await?;

        Ok(())
    }

    pub async fn send_heartbeat(&self, term: u16) -> Result<(), Error> {
        let last_log_term = match self.log.last() {
            Some(entry) => entry.term,
            None => 0,
        };

        let last_log_index = if !self.log.is_empty() {
            self.log.len() - 1
        } else {
            // I'm never going to send an append entry if there's nothing in my log
            0
        };

        self.send_msg(&messages::ServerMessage {
            body: self.build_body("FFFF", "append_entry"),
            options: messages::ServerOptions::AppendEntry {
                term,
                leader_id: self.id.to_string(),
                prev_log_index: last_log_index as u16,
                prev_log_term: last_log_term,
                // heartbeat should send an empty vector
                entries: Vec::new(),
                leader_commit_index: self.commit_index,
            },
        })
        .await?;

        Ok(())
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
                (self.log.len() - 1) <= other_last_log_index.into()
            }
            // If our log is empty, then our last log index is 0, which means
            // everyone is at least as long as us
            None => true,
        }
    }

    // abstract away sending messages
    async fn send_msg<T>(&self, msg: &T) -> Result<usize, Error>
    where
        T: Serialize,
    {
        let buf: Vec<u8> = serde_json::to_vec(msg)?;
        self.sock.send(&buf).await
    }
}
