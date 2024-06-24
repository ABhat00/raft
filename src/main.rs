use clap::Parser;
use messages::ServerOptions;
use replica::ReplicaState;
use std::io::Result;

mod loggers;
mod messages;
mod replica;
mod timer;

#[derive(Parser)]
struct Args {
    machine_id: String,
    replica_ids: Vec<String>,
}

// things i need to do to get this to work:
/*
    1. fix the timer. Let's run this on a separate thread, then have a channel shared to notify the replica. Then test leader elections and make sure that a simple test
       case is working properly
    2. create separate polling functions for leader, follower, and candidate. These state's mean different things, so the replicas should use different functions
    3. implement log replication - notes above.
    4. actually respect
*/
#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let mut m = replica::new(&args.machine_id, &args.replica_ids).await;

    // computed by the orchestrator - needs to be passed into the replica
    let required_vote_threshold: u16 = ((m.colleagues.len() / 2) + 1).try_into().unwrap();

    loop {
        let message = m.read().await;
        let body = message.body;

        match message.options {
            ServerOptions::Put { key, value } => {
                if m.is_leader() {
                    // not doing any sort of consensus work here
                    // this should write to the log, and then send an append entries
                    // once we get a sufficient number of agreements from replicas
                    match m.commit(key, value) {
                        Some(_) => m.send_ok(&body.src, &body.mid).await?,
                        None => m.send_fail(&body.src, &body.mid).await?,
                    }
                } else {
                    // We don't redirect to the leader, we redirect to
                    // the client
                    m.redirect(&body.src, &body.mid).await?;
                }
            }
            ServerOptions::Get { key } => {
                // m.logger.log(format!("leader is {} ", m.leader));
                if m.is_leader() {
                    m.get(&key, &body.src, &body.mid).await?;
                } else {
                    m.redirect(&body.src, &body.mid).await?;
                }
            }
            ServerOptions::RequestVote {
                term,
                last_log_index,
                last_log_term,
            } => {
                // See if we should vote
                if !m.vote_history.contains(&term)
                    && m.as_least_as_long(last_log_index, last_log_term)
                {
                    m.logger
                        .log(format!("voting for {} in term {}", &body.src, term))
                        .unwrap();
                    m.vote(&body.src, term).await?
                }
            }
            ServerOptions::Vote { term } => {
                if matches!(m.state, ReplicaState::Candidate) && term == m.term {
                    // 1. tally the vote (this key should already exist in the map because I voted for myself)
                    let num_votes_in_term = m.vote_tally.entry(term).or_insert(1);

                    *num_votes_in_term += 1;

                    // 2. see if we're the leader yet
                    if *num_votes_in_term >= required_vote_threshold {
                        // 3. Change our status to leader
                        m.state = ReplicaState::Leader;
                        m.leader = m.id.to_string();

                        m.logger
                            .log(format!(
                                "{} is the leader, in term {}, with {} votes",
                                m.id, m.term, *num_votes_in_term
                            ))
                            .unwrap();

                        // 4. Send an append entry
                        m.reset_time_of_last_heartbeat();
                        m.send_heartbeat(term).await?;
                    }
                }
            }
            ServerOptions::AppendEntry {
                term,
                leader_id,
                prev_log_index,
                prev_log_term,
                entries,
                leader_commit_index,
            } => {
                if leader_id == m.leader {
                    m.reset_time_of_last_heartbeat();
                } else if m.should_accept_leader(term) {
                    m.logger
                        .log(format!(
                            "{} accepting leader {} in term {}",
                            m.id, leader_id, term
                        ))
                        .unwrap();
                    m.state = ReplicaState::Follower;
                    m.leader = leader_id;
                    m.term = term;
                }
            }
            ServerOptions::AppendEntryResult { term, success } => {}
        }
    }

    Ok(())
}
