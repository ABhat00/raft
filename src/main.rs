use clap::Parser;
use std::io::Result;
//use tokio_seqpacket::UnixSeqpacket;
mod messages;

#[derive(Parser)]
struct Args {
    machine_id: String,
    replica_ids: Vec<String>,
}

struct RaftMachine {}

#[tokio::main]
async fn main() -> Result<()> {
    let mut last_time = 0;
    let args = Args::parse();

    println!("{}", args.machine_id);
    println!("{:?}", args.replica_ids);
    let sock = UnixSeqpacket::connect(args.machine_id).await?;

    loop {
        let mut buf = [0u8; 32768];

        let len = sock.recv(&mut buf).await?;

        let res: messages::Body = serde_json::from_slice(&buf[0..len]).unwrap();

        println!("{:?}", res);

        break;
    }

    Ok(())
}
