use std::{
    collections::HashMap,
    sync::{Arc, Mutex, OnceLock, atomic::AtomicUsize},
};

use mio::{Events, Interest, Poll, Token, net::TcpStream};

use crate::executor::Waker;

type Wakers = Arc<Mutex<HashMap<usize, Waker>>>;

// OnceLock allows us to define a static variable that we can write to once
// so that we can initialise it when we start our reactor. By doing so, we also
// make sure that there can only be a single instance of this specific reactor
// running in our program.
static REACTOR: OnceLock<Reactor> = OnceLock::new();

/// Returns a reference to the reactor instance.
pub fn reactor() -> &'static Reactor {
    REACTOR.get().expect("Called outside a runtime context")
}

pub struct Reactor {
    // A HashMap of Waker objects, each identified by an integer ID.
    wakers: Wakers,

    // Holds a `Registry` instance so that we can interact with the event queue
    // in `mio`.
    registry: mio::Registry,

    // Stores the next available ID so that we can track which event occurred
    // and which `Waker` should be woken up.
    next_id: AtomicUsize,
}

impl Reactor {
    /// Register with the `Registry`. We pass in an ID property so that we can
    /// identify which event has occurred when we receive a notification later
    /// on.
    pub fn register(
        &self,
        stream: &mut TcpStream,
        interest: Interest,
        id: usize,
    ) {
        self.registry
            .register(stream, Token(id), interest)
            .expect("Failed to register stream with reactor");
    }

    /// Adds a waker to our HashMap using the ID property as the key. If there
    /// is Waker already there, we replace it and drop the old one.
    ///
    /// An important point to remember is that **we should always store the
    /// most recent Waker** so that this function can be called multiple times,
    /// even though there is already a Waker associated with the `TcpStream`.
    pub fn set_waker(&self, waker: &Waker, id: usize) {
        let _ = self
            .wakers
            .lock()
            .map(|mut w| w.insert(id, waker.clone()).is_none())
            .unwrap();
    }

    /// Removes the Waker from the HashMap and deregisters the `TcpStream` from
    /// our `Registry`.
    pub fn deregister(&self, stream: &mut TcpStream, id: usize) {
        self.wakers.lock().map(|mut w| w.remove(&id)).unwrap();
        self.registry.deregister(stream).unwrap();
    }

    /// Gets the current `next_id` value and incremements the counter atomically.
    /// We don't care about any happens before/after relationships here; we only
    /// care about not handing out the same value twice, so `Ordering::Relaxed`
    /// will suffice here.
    pub fn next_id(&self) -> usize {
        self.next_id
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    }
}

/// Initialises and starts the reactor.
pub fn start() {
    use std::thread::spawn;

    let wakers = Arc::new(Mutex::new(HashMap::new()));

    let poll = Poll::new().unwrap();
    let registry = poll.registry().try_clone().unwrap();
    let next_id = AtomicUsize::new(1);
    let reactor = Reactor {
        wakers: wakers.clone(),
        registry,
        next_id,
    };

    REACTOR
        .set(reactor)
        .ok()
        .expect("Reactor is already running");

    // We spawn a new OS thread and start our event loop function on that one.
    // This also means that pass on our `Poll` instance to the event loop
    // thread for good.
    //
    // The best practice would be to store the `JoinHandle` returned from
    // `spawn` so that can join the thread later on, but our thread has no way
    // to shut down the event loop anyway, so joining it later on makes little
    // sense, and we simply discard the handle.
    spawn(move || {
        event_loop(poll, wakers);
    });
}

// Loop forever. This makes the example short and simple, but it has the
// downside that we have no way of shutting our event loop down once it's
// started.  Fixing is not that difficult, but since it won't be necessary
// for our example, we won't do it here.
fn event_loop(mut poll: Poll, wakers: Wakers) {
    let mut events = Events::with_capacity(100);
    loop {
        // Call `poll` with a timeout of `None`, which means that it will block
        // until an event occurs.
        poll.poll(&mut events, None).unwrap();
        // If we receive an event, it means that something we registered
        // interest in has happened. We get the `id` we passed in when we
        // first registered an interest in events on this `TcpStream`.
        for e in events.iter() {
            let Token(id) = e.token();
            println!("Event occurred for id: {}, event: {:?}", id, e);
            let wakers = wakers.lock().unwrap();
            // We try to get the associated Waker and call `wake` on it.
            // We guard ourselves from the fact that the Waker may have been
            // removed from our collection already, in which case we do nothing.
            //
            // It's worth noting that we can filter events if we want to here.
            // For our example, we don't need to filter events.
            if let Some(waker) = wakers.get(&id) {
                waker.wake();
            }
        }
    }
}
