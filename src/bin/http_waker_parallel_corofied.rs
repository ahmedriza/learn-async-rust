//
// This is the template file that needs to be run through `corofy_waker` 
// in order to generate the state machine transformation for the async code.
//

use std::time::Instant;
use chrono::Local;
use std::thread::Builder;

use learn_async_rust::{
    executor::{self, Executor, Waker},
    future_with_waker::{Future, PollState},
    http_waker::Http, runtime_two,
};

//
// To get the number of cores on a Mac:
// `sysctl -n hw.ncpu`
//
// This will send 60 HTTP GET requests in total.
//
fn main() {
    let start = Instant::now();
    let mut executor = runtime_two::init();
    let mut handles = vec![];

    // Create 11 parallel executors (each in their own thread) each running 
    // 5 tasks.
    for i in 1..12 {
        let name = format!("exec-{i}");
        let h = Builder::new().name(name).spawn(move || {
            let mut executor = Executor::new();
            executor.block_on(async_main());
        }).unwrap();
        handles.push(h);
    }
  
    // Submit another 5 tasks on the `main` thread.
    executor.block_on(async_main());
    handles.into_iter().for_each(|h| h.join().unwrap());

    println!("\nELAPSED TIME: {}", start.elapsed().as_secs_f32());
}






// =================================
// We rewrite this:
// =================================
    
// coroutine fn request(i: usize) {
//     let path = format!("/{}/HelloWorld-{i}", i * 1000);
//     let txt = Http::get(path).wait;
//     let now = Local::now();
//     println!("{now} [{}] Response:\n{txt}", std::thread::current().name().unwrap());
//     println!();

// }

// =================================
// Into this:
// =================================

fn request(i: usize) -> impl Future<Output=String> {
    Coroutine0::new(i)
}
        
enum State0 {
    Start(usize),
    Wait1(Box<dyn Future<Output = String>>),
    Resolved,
}

struct Coroutine0 {
    state: State0,
}

impl Coroutine0 {
    fn new(i: usize) -> Self {
        Self { state: State0::Start(i) }
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
                State0::Start(i) => {
                    // ---- Code you actually wrote ----
                    let path = format!("/{}/HelloWorld-{i}", i * 1000);

                    // ---------------------------------
                    let fut1 = Box::new( Http::get(path));
                    self.state = State0::Wait1(fut1);
                }

                State0::Wait1(ref mut f1) => {
                    match f1.poll(waker) {
                        PollState::Ready(txt) => {
                            // ---- Code you actually wrote ----
                            let now = Local::now();
    println!("{now} [{}] Response:\n{txt}", std::thread::current().name().unwrap());
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


// =================================
// We rewrite this:
// =================================
    
// coroutine fn async_main() {
//     println!("Program starting");
// 
//     for i in 0..5 {
//         let future = request(i);
//         executor::spawn(future);
//     }

// }

// =================================
// Into this:
// =================================

fn async_main() -> impl Future<Output=String> {
    Coroutine1::new()
}
        
enum State1 {
    Start,
    Resolved,
}

struct Coroutine1 {
    state: State1,
}

impl Coroutine1 {
    fn new() -> Self {
        Self { state: State1::Start }
    }
}


impl Future for Coroutine1 {
    type Output = String;

    // Supress warnings about unused variables since `waker` may not always
    // be used directly.
    #[allow(unused)]
    fn poll(&mut self, waker: &Waker) -> PollState<Self::Output> {
        loop {
        match self.state {
                State1::Start => {
                    // ---- Code you actually wrote ----
                    println!("Program starting");

    for i in 0..5 {
        let future = request(i);
        executor::spawn(future);
    }

                    // ---------------------------------
                    self.state = State1::Resolved;
                    break PollState::Ready(String::new());
                }

                State1::Resolved => panic!("Polled a resolved future")
            }
        }
    }
}
