use crate::future::{Future, PollState};
use std::sync::OnceLock;

use mio::{Events, Poll, Registry};

static REGISTRY: OnceLock<Registry> = OnceLock::new();

pub fn registry() -> &'static Registry {
    REGISTRY.get().expect("Called outside a runtime context")
}

/// Single threaded runtime for executing futures
pub struct Runtime {
    poll: Poll,
}

impl Runtime {
    pub fn new() -> Self {
        let poll = Poll::new().unwrap();
        let registry = poll.registry().try_clone().unwrap();
        REGISTRY.set(registry).unwrap();
        Self { poll }
    }

    pub fn block_on<F>(&mut self, future: F)
    where
        F: Future<Output = String>,
    {
        let mut future = future;
        // loop until the future returns PollState::Ready
        loop {
            match future.poll() {
                PollState::NotReady => {
                    println!("Schedule other tasks\n");
                    let mut events = Events::with_capacity(100);
                    // Wait for events on the poller.
                    // This yields to the OS scheduler.
                    self.poll.poll(&mut events, None).unwrap();
                }
                PollState::Ready(_) => {
                    break;
                }
            }
        }
    }
}
