//
// This is the template file that needs to be run through `corofy_waker` 
// in order to generate the state machine transformation for the async code.
//

use learn_async_rust::{
    future_with_waker::{Future, PollState},
    http_waker::Http, runtime_two,
};

fn main() {
    let future = async_main();
    let mut executor = runtime_two::init();
    executor.block_on(future);
}




// =================================
// We rewrite this:
// =================================
    
// coroutine fn async_main() {
//     println!("Program starting");
// 
//     let txt = Http::get("/600/HelloAsyncAwait".to_string()).wait;
//     println!("{txt}");
//     println!();
//     
//     let txt = Http::get("/400/HelloAsyncAwait".to_string()).wait;
//     println!("{txt}");
//     println!();

// }

// =================================
// Into this:
// =================================

fn async_main() -> impl Future<Output=String> {
    Coroutine0::new()
}
        
enum State0 {
    Start,
    Wait1(Box<dyn Future<Output = String>>),
    Wait2(Box<dyn Future<Output = String>>),
    Resolved,
}

struct Coroutine0 {
    state: State0,
}

impl Coroutine0 {
    fn new() -> Self {
        Self { state: State0::Start }
    }
}


use learn_async_rust::executor::Waker;
impl Future for Coroutine0 {
    type Output = String;

    fn poll(&mut self, waker: &Waker) -> PollState<Self::Output> {
        loop {
        match self.state {
                State0::Start => {
                    // ---- Code you actually wrote ----
                    println!("Program starting");


                    // ---------------------------------
                    let fut1 = Box::new( Http::get("/600/HelloAsyncAwait".to_string()));
                    self.state = State0::Wait1(fut1);
                }

                State0::Wait1(ref mut f1) => {
                    match f1.poll(waker) {
                        PollState::Ready(txt) => {
                            // ---- Code you actually wrote ----
                            println!("{txt}");
    println!();
    

                            // ---------------------------------
                            let fut2 = Box::new( Http::get("/400/HelloAsyncAwait".to_string()));
                            self.state = State0::Wait2(fut2);
                        }
                        PollState::NotReady => break PollState::NotReady,
                    }
                }

                State0::Wait2(ref mut f2) => {
                    match f2.poll(waker) {
                        PollState::Ready(txt) => {
                            // ---- Code you actually wrote ----
                            println!("{txt}");
    println!();

                            // ---------------------------------
                            self.state = State0::Resolved;
                            break PollState::Ready(String::new());
                        }
                        PollState::NotReady => break PollState::NotReady,
                    }
                }

                State0::Resolved => panic!("Polled a resolved future")
            }
        }
    }
}
