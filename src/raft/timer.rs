use crossbeam::channel::{bounded, Receiver};
use std::{thread, time::Duration};

pub struct Timer {
    pub rx: Receiver<()>,
    timeout: Duration,
}
// timing strategy borrowed from little-raft
/*
 basically the way this works is it puts the timer
 on a separate thread and then turns the main event loop into a select loop
 between the timer channel and the message channel
*/
impl Timer {
    pub fn new(timeout: Duration) -> Self {
        Timer {
            rx: Timer::get_timeout_channel(timeout),
            timeout,
        }
    }

    // I need to call reset on this every time a replica gets a heartbeat
    pub fn reset(&mut self) {
        self.rx = Timer::get_timeout_channel(self.timeout);
    }

    pub fn get_rx(&self) -> &Receiver<()> {
        &self.rx
    }

    fn get_timeout_channel(timeout: Duration) -> Receiver<()> {
        let (tx, rx) = bounded(1);

        thread::spawn(move || {
            thread::sleep(timeout);
            let _ = tx.send(());
        });

        rx
    }
}
