use clap::Parser;
use std::io::Result;
mod messages;
mod raft;

#[derive(Parser)]
struct Args {
    machine_id: String,
    replica_ids: Vec<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut last_time = 0;
    let args = Args::parse();

    let m = raft::new(&args.machine_id).await?;

    loop {
        let recv_msg: messages::Body = m.read().await?;
        println!("{:?}", recv_msg);

        match recv_msg.mtype() {
            messages::Recv::Put { key, value } => println!("put"),
            messages::Recv::Get { key } => println!("get"),
        }
        break;
    }

    Ok(())
}
