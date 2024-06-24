use crossbeam::channel::{bounded, Receiver};
use std::{thread, time::Duration};

pub struct Timer {
    rx: Receiver<()>,
    timeout: Duration,
}
// borrowed from little-raft
/*
basically the way this works is it puts the timer
 on a separate thread and then turns the main event loop into a select loop
*/
impl Timer {
    pub fn new(timeout: Duration) -> Self {
        Timer {
            rx: Timer::get_timeout_channel(timeout),
            timeout,
        }
    }

    pub fn renew(&mut self) {
        self.rx = Timer::get_timeout_channel(self.timeout);
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
