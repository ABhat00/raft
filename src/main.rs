use clap::Parser;
use raft::{Replica, ReplicaState};
use std::io::Result;
use std::time::{Duration, SystemTime};
use tokio::time::timeout;

mod messages;
mod raft;

#[derive(Parser)]
struct Args {
    machine_id: String,
    replica_ids: Vec<String>,
}

// What do I have left to do?
// TODO: Implement Leader Elections
//   - init replicas with randomized timeouts (Done)
//   - identify missing leaders (time since last append entry > randomized timeout) (Done)
//   - start an election - send out a RequestVote RPC to all messages and
//     transition my state to candidate
//   - vote / respond to RequestVote (I vote for anyone as long as their log
//     is at least as long as mine and I haven't voted for someone in the same term)
//   - If I receive an append entry from a higher term than mine, I transition from
//     candidate to follower TODO!!!!
//   - If I get a majority of votes, transition to state leader and start sending out append entries
//     (Just need the append entries here)

// TODO 6/7
// - Fix this to use poll_recv instead of an async timeout
// - Implement append entry - heartbeats, and if I receive an append entry from a term higher than mine (AS a candidate),  i switch back to follower
// - Implement the Vote RPC
#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let mut m = raft::new(&args.machine_id, &args.replica_ids).await?;

    loop {
        let attempt_read = m.read();

        // This timeout could cause problems - it resets if we get any message,
        // not just messages from the leader. This means that illegitimate leaders
        // and clients can send us messages, and the timeout will reset. Could lock us here.

        // the leader could also miss a heartbeat b/c a message was redirected from a follower

        // We need to find a way to make sure that the timeout only resets on
        // appendEntry messages from the actual leader

        // I think I need to rewrite this to use poll_select, but I should look into it more
        match timeout(Duration::from_millis(m.election_timeout), attempt_read).await {
            Ok(msg) => {
                let recv_msg = msg?;

                println!("{:?}", recv_msg);
                let body = recv_msg.body;

                match recv_msg.options {
                    messages::RecvOptions::Put { key, value } => {
                        if m.is_leader() {
                            m.send_fail(&body.src, &body.mid).await?;
                        } else {
                            m.redirect(&body.src, &body.mid).await?;
                        }
                    }
                    messages::RecvOptions::Get { key } => {
                        if m.is_leader() {
                            m.get(&key, &body.src, &body.mid).await?;
                        } else {
                            m.redirect(&body.src, &body.mid).await?;
                        }
                    }
                    messages::RecvOptions::RequestVote {
                        replica_id: id,
                        term,
                        last_log_index,
                        last_log_term,
                    } => {
                        // See if we should vote
                        if !m.vote_history.contains(&term)
                            && m.as_least_as_long(last_log_index, last_log_term)
                        {
                            m.vote(term).await?
                        }
                    }
                    messages::RecvOptions::Vote { replica_id, term } => {
                        if matches!(m.state, ReplicaState::Candidate) {
                            // 1. tally the vote
                            let num_votes_in_term = m.vote_tally.entry(m.term);
                            *num_votes_in_term += 1;

                            // 2. see if we're the leader yet
                            if num_votes_in_term >= (m.colleagues.len() / 2) + 1 {
                                // 3. Change our status to leader
                                m.state = ReplicaState::Leader;

                                // 4. Send an append entry
                                todo!("send append entry (this is really just a heartbeat)")
                            }
                        }
                    }
                }
            }
            Err(err) => {
                if m.is_leader() {
                    // send heartbeat
                    todo!("send heartbeat")
                } else {
                    // This is where we start a leader election
                    m.state = ReplicaState::Candidate;
                    m.vote_tally.insert(m.term, 1);
                    m.vote_history.insert(m.term);
                    m.request_vote().await?
                }
            }
        }
    }
}
