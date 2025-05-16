//
// This program demonstrates how to use data across wait points.
//
// Run this template through `corofy_waker` to generate the Rust code.
// The generated code, `http_wait_points_corofied.rs` need to be modified
// in order to handle the `counter` state across wait points.
//
use learn_async_rust::{
    executor::Waker,
    future_with_waker::{Future, PollState},
    http_waker::Http,
    runtime_two,
};
use std::fmt::Write;

fn main() {
    let future = async_main();
    let mut executor = runtime_two::init();
    executor.block_on(future);
}

// =================================
// We rewrite this:
// =================================

// coroutine fn async_main() {
//     let mut buffer = String::from("\nBUFFER:\n----\n");
//     let writer = &mut buffer;
//     println!("Program starting");
//
//     let txt = Http::get("/600/HelloAsyncAwait".to_string()).wait;
//     writeln!(writer, "{txt}").unwrap();
//
//     let txt = Http::get("/400/HelloAsyncAwait".to_string()).wait;
//     writeln!(writer, "{txt}").unwrap();
//
//     println!("{}", buffer);

// }

// =================================
// Into this:
// =================================

fn async_main() -> impl Future<Output = String> {
    Coroutine0::new()
}

enum State0 {
    Start,
    Wait1(Box<dyn Future<Output = String>>),
    Wait2(Box<dyn Future<Output = String>>),
    Resolved,
}

#[derive(Default)]
struct Stack0 {
    buffer: Option<String>,
    writer: Option<*mut String>,
}

struct Coroutine0 {
    state: State0,
    stack: Stack0,
}

impl Coroutine0 {
    fn new() -> Self {
        Self {
            state: State0::Start,
            stack: Stack0::default(),
        }
    }
}

impl Future for Coroutine0 {
    type Output = String;

    // Supress warnings about unused variables since `waker` may not always
    // be used directly.
    #[allow(unused)]
    fn poll(&mut self, waker: &Waker) -> PollState<Self::Output> {
        loop {
            match self.state {
                State0::Start => {
                    // initialise stack (hoist variables)
                    self.stack.buffer = Some(String::from("\nBUFFER:\n----\n"));
                    self.stack.writer =
                        Some(self.stack.buffer.as_mut().unwrap());

                    // ---- Code you actually wrote ----
                    let mut buffer = String::from("\nBUFFER:\n----\n");
                    let writer = &mut buffer;
                    println!("Program starting");

                    // ---------------------------------
                    let fut1 =
                        Box::new(Http::get("/600/HelloAsyncAwait".to_string()));
                    self.state = State0::Wait1(fut1);
                }

                State0::Wait1(ref mut f1) => {
                    match f1.poll(waker) {
                        PollState::Ready(txt) => {
                            // restore stack
                            let writer = unsafe {
                                &mut *self.stack.writer.take().unwrap()
                            };

                            // ---- Code you actually wrote ----
                            writeln!(writer, "{txt}").unwrap();

                            // ---------------------------------
                            let fut2 = Box::new(Http::get(
                                "/400/HelloAsyncAwait".to_string(),
                            ));
                            self.state = State0::Wait2(fut2);

                            // save stack
                            self.stack.writer = Some(writer);
                        }
                        PollState::NotReady => break PollState::NotReady,
                    }
                }

                State0::Wait2(ref mut f2) => {
                    match f2.poll(waker) {
                        PollState::Ready(txt) => {
                            // restore stack
                            let buffer = self.stack.buffer.as_ref().unwrap();
                            let writer = unsafe {
                                &mut *self.stack.writer.take().unwrap()
                            };

                            // ---- Code you actually wrote ----
                            writeln!(writer, "{txt}").unwrap();

                            println!("{}", buffer);

                            // ---------------------------------
                            self.state = State0::Resolved;

                            // save stack / free resources
                            let _ = self.stack.buffer.take();

                            break PollState::Ready(String::new());
                        }
                        PollState::NotReady => break PollState::NotReady,
                    }
                }

                State0::Resolved => panic!("Polled a resolved future"),
            }
        }
    }
}
