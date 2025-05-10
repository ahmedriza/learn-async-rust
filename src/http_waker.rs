use chrono::Local;
use mio::Interest;

use crate::{
    executor::Waker,
    future_with_waker::{Future, PollState},
    reactor::reactor,
};
use std::io::{ErrorKind, Read, Write};

pub struct Http;

impl Http {
    pub fn get(path: String) -> impl Future<Output = String> {
        HttpGetFuture::new(&path)
    }
}

// This is our leaf future that will perform the HTTP GET request.
pub struct HttpGetFuture {
    pub stream: Option<mio::net::TcpStream>,
    // We'll read the data from the TcpStream and put it all in this buffer
    // until we've read all the data returned from the server.
    pub buffer: Vec<u8>,
    pub path: String,
    id: usize,
}

impl HttpGetFuture {
    pub fn new(path: &str) -> Self {
        let id = reactor().next_id();
        Self {
            stream: None,
            buffer: vec![],
            path: path.to_string(),
            id,
        }
    }

    /// Write a request to the server and initialize the stream.
    pub fn write_request(&mut self) {
        let stream = std::net::TcpStream::connect("127.0.0.1:7070").unwrap();
        stream.set_nonblocking(true).unwrap();
        let mut stream = mio::net::TcpStream::from_std(stream);
        stream.write_all(get_req(&self.path).as_bytes()).unwrap();
        self.stream = Some(stream);
    }
}

/// Implement the Future trait for our HttpGetFuture
impl Future for HttpGetFuture {
    type Output = String;

    fn poll(&mut self, waker: &Waker) -> PollState<Self::Output> {
        if self.stream.is_none() {
            let now = Local::now();
            println!(
                "{:?} [{}]: First poll, start operation",
                now,
                std::thread::current().name().unwrap()
            );
            self.write_request();

            let stream = self.stream.as_mut().unwrap();
            reactor().register(stream, Interest::READABLE, self.id);
            reactor().set_waker(waker, self.id);
        }

        let mut buf = vec![0; 4096];
        loop {
            match self.stream.as_mut().unwrap().read(&mut buf) {
                Ok(0) => {
                    // No more data to read
                    let s = String::from_utf8_lossy(&self.buffer);
                    // De-register the stream from our `Poll` instance when
                    // we're done.
                    reactor()
                        .deregister(self.stream.as_mut().unwrap(), self.id);
                    break PollState::Ready(s.to_string());
                }
                Ok(n) => {
                    // Read n bytes from the stream
                    self.buffer.extend_from_slice(&buf[..n]);
                    // Try to read more data from the stream
                    continue;
                }
                Err(e) if e.kind() == ErrorKind::WouldBlock => {
                    // As per the Rust `Future::poll` documentation, it's
                    // expected that the Waker from the *most recent call*
                    // should be scheduled to wake up. That means that everytime
                    // we get a `WouldBlock` error, we need to make sure that
                    // we store the most recent Waker.  The reason is that the
                    // future could have moved to a different executor in between
                    // calls, and we need to wake up the correct one (it won't
                    // be possible to move futures like those in our example,
                    // but we are playing by the same rules).
                    reactor().set_waker(waker, self.id);
                    // Since we put the stream in non-blocking mode,
                    // the data is not ready yet, or there is more data, but
                    // we haven't received it yet.
                    return PollState::NotReady;
                }
                Err(e) if e.kind() == ErrorKind::Interrupted => {
                    // reads can be interrupted by signals.
                    // The operation was interrupted, try again
                    continue;
                }
                Err(e) => {
                    // An other error occurred; simply panic
                    panic!("Error reading from stream: {}", e);
                }
            }
        }
    }
}

// -----------------------------------------------------------------------------

pub fn get_req(path: &str) -> String {
    format!(
        "GET {path} HTTP/1.1\r\n\
             Host: localhost\r\n\
             Connection: close\r\n\
             \r\n"
    )
}
