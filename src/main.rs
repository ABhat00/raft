use clap::Parser;
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
//     candidate to follower
//   - If I get a majority of votes, transition to state leader and start sending out
#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let m = raft::new(&args.machine_id, &args.replica_ids).await?;

    loop {
        let attempt_read = m.read();

        match timeout(Duration::from_millis(m.election_timeout), attempt_read).await {
            Ok(msg) => {
                let recv_msg = msg?;

                println!("{:?}", recv_msg);
                let body = recv_msg.body;

                match recv_msg.options {
                    messages::RecvOptions::Put { key, value } => {
                        if m.is_leader() {
                            m.send_fail(&body.src, &body.id).await?;
                        } else {
                            m.redirect(&body.src, &body.id).await?;
                        }
                    }
                    messages::RecvOptions::Get { key } => {
                        if m.is_leader() {
                            m.get(&key, &body.src, &body.id).await?;
                        } else {
                            m.redirect(&body.src, &body.id).await?;
                        }
                        m.send_fail(&body.src, &body.id).await?;
                    }
                    messages::RecvOptions::RequestVote {
                        id,
                        term,
                        last_log_index,
                        last_log_term,
                    } => {}
                }
            }
            Err(err) => {
                // This is where we start a leader election
            }
        }

        // I've left this break in to make small tests possible
    }
}
