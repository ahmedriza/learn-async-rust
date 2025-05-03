use std::{thread, time::Duration};

use learn_async_rust::{
    future::{Future, PollState},
    http::Http,
};

pub fn main() {
    let mut future = async_main();
    loop {
        match future.poll() {
            PollState::Ready(()) => {
                println!("Program finished");
                break;
            }
            PollState::NotReady => {
                println!("Schedule other tasks");
            }
        }
        thread::sleep(Duration::from_millis(100));
    }
}

pub fn async_main() -> impl Future<Output = ()> {
    Coroutine::new()
}

pub enum State {
    Start,
    Wait1(Box<dyn Future<Output = String>>),
    Wait2(Box<dyn Future<Output = String>>),
    Resolved,
}

pub struct Coroutine {
    pub state: State,
}

impl Coroutine {
    pub fn new() -> Self {
        Self {
            state: State::Start,
        }
    }
}

impl Future for Coroutine {
    type Output = ();

    fn poll(&mut self) -> PollState<Self::Output> {
        loop {
            match self.state {
                State::Start => {
                    println!("Program starting");
                    let fut =
                        Box::new(Http::get("/600/HelloWorld1".to_string()));
                    self.state = State::Wait1(fut);
                }
                State::Wait1(ref mut fut) => match fut.poll() {
                    PollState::Ready(txt) => {
                        println!("Response: {txt}");
                        let fut2 =
                            Box::new(Http::get("/400/HelloWorld2".to_string()));
                        self.state = State::Wait2(fut2);
                    }
                    PollState::NotReady => break PollState::NotReady,
                },
                State::Wait2(ref mut fut2) => match fut2.poll() {
                    PollState::Ready(txt2) => {
                        println!("Response: {txt2}");
                        self.state = State::Resolved;
                        break PollState::Ready(());
                    }
                    PollState::NotReady => break PollState::NotReady,
                },
                State::Resolved => {
                    panic!("Polled a resoled future");
                }
            }
        }
    }
}
