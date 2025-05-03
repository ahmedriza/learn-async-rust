use crate::future::{Future, PollState};
use std::io::{ErrorKind, Read, Write};

pub struct Http;

impl Http {
    pub fn get(path: &str) -> impl Future<Output = String> {
        HttpGetFuture::new(path)
    }
}

// This is our leaf future that will perform the HTTP GET request.
pub struct HttpGetFuture {
    pub stream: Option<mio::net::TcpStream>,
    // We'll read the data from the TcpStream and put it all in this buffer
    // until we've read all the data returned from the server.
    pub buffer: Vec<u8>,
    pub path: String,
}

impl HttpGetFuture {
    pub fn new(path: &str) -> Self {
        Self {
            stream: None,
            buffer: vec![],
            path: path.to_string(),
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

    fn poll(&mut self) -> PollState<Self::Output> {
        if self.stream.is_none() {
            println!("First poll, start operation");
            self.write_request();
            return PollState::NotReady;
        }

        let mut buf = vec![0; 4096];
        loop {
            match self.stream.as_mut().unwrap().read(&mut buf) {
                Ok(0) => {
                    // No more data to read
                    let s = String::from_utf8_lossy(&self.buffer);
                    break PollState::Ready(s.to_string());
                }
                Ok(n) => {
                    // Read n bytes from the stream
                    self.buffer.extend_from_slice(&buf[..n]);
                    // Try to read more data from the stream
                    continue;
                }
                Err(e) if e.kind() == ErrorKind::WouldBlock => {
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
