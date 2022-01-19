use clap::Parser;
use polling::{Event, Poller};
use std::io::Result;
use tokio_seqpacket::UnixSeqpacket;
#[derive(Parser)]
struct Args {
    machine_id: String,
    replica_ids: Vec<String>,
}

async fn raft(args: Args) -> Result<()> {
    let sock = UnixSeqpacket::connect(args.machine_id).await?;

    let p = Poller::new()?;

    p.add(&sock, Event::readable(7))?;

    let mut events = Vec::new();
    loop {
        let mut buf = [0; 32768];
        events.clear();

        p.wait(&mut events, None)?;

        for e in &events {
            if e.readable {
                sock.recv(&mut buf);
                println!("{:?}", buf)
            }
            println!("{:?}", e)
        }
        break;
    }

    Ok(())
}
fn main() -> Result<()> {
    let mut last_time = 0;
    let args = Args::parse();

    println!("{}", args.machine_id);
    println!("{:?}", args.replica_ids);

    raft(args);
    Ok(())
}
