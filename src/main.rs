use clap::Parser;
use core::panic;
use polling::{Event, Poller};
use std::io::{Bytes, Result};
use std::os::unix::net::UnixStream;
#[derive(Parser)]
struct Args {
    machine_id: String,
    replica_ids: Vec<String>,
}

fn main() -> Result<()> {
    let mut last_time = 0;
    let args = Args::parse();

    println!("{}", args.machine_id);
    println!("{:?}", args.replica_ids);

    let stream_result = UnixStream::connect(args.machine_id);

    let mut sock = match stream_result {
        Ok(t) => t,
        Err(x) => panic!("{}", x),
    };

    let p = Poller::new()?;

    p.add(&sock, Event::readable(7))?;

    let mut events = Vec::new();
    loop {
        events.clear();

        p.wait(&mut events, None)?;

        for e in &events {
            println!("{:?}", e)
        }
        break;
    }
    Ok(())
}
