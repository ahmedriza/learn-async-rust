use std::{
    collections::HashSet,
    env,
    io::{self, Read, Write},
    net::TcpStream,
};

use learn_async_rust::{
    ffi::{self, Event},
    poll::Poll,
};

/// Send a set of requests to a delayserver with varying delays and then use
/// epoll to wait for the responses. Therefore, we'll only use epoll to
/// track read events in this example.
fn main() -> anyhow::Result<()> {
    let mut poll = Poll::new()?;
    let n_events = 5;

    let mut streams = vec![];
    // Allow the base URL override by passing it as a command line argument
    let base_url = env::args()
        .nth(1)
        .unwrap_or_else(|| String::from("localhost"));
    let addr = format!("{}:7070", base_url);

    for i in 0..n_events {
        let delay = (n_events - i) * 1000;
        let url_path = format!("/{delay}/request-{i}");
        let request = get_req(&url_path);

        let mut stream = TcpStream::connect(&addr)?;
        stream.set_nonblocking(true)?;
        // Disable Nagle's algorithm to send the request immediately
        stream.set_nodelay(true)?;

        stream.write_all(request.as_bytes())?;

        // Token is equal to index in Vec.
        poll.registry()
            .register(&stream, i, ffi::EPOLLIN | ffi::EPOLLET)?;

        streams.push(stream);
    }

    // Store the handled IDs
    let mut handled_ids = HashSet::new();

    let mut handled_events = 0;
    while handled_events < n_events {
        let mut events = Vec::with_capacity(10);
        poll.poll(&mut events, None)?;

        if events.is_empty() {
            println!("Timeout (or spurious event notification)");
            continue;
        }

        handled_events +=
            handle_events(&events, &mut streams, &mut handled_ids)?;
    }

    println!("All events handled");

    Ok(())
}

fn handle_events(
    events: &[Event],
    streams: &mut [TcpStream],
    handled: &mut HashSet<usize>,
) -> anyhow::Result<usize> {
    let mut handled_events = 0;
    for event in events {
        let index = event.token();
        let mut data = vec![0u8; 4096];

        // loop until we read all the data from the stream
        // Remember how important it is to fully drain the buffer when using
        // epoll in edge-triggered mode.
        loop {
            match streams[index].read(&mut data) {
                Ok(n) if n == 0 => {
                    // If n = 0, we've drained the buffer; we consider the 
                    // event as handled and break out of the loop.
                    //
                    // `insert` returns false if the value already existed in
                    // the set.
                    if !handled.insert(index) {
                        break;
                    }
                    handled_events += 1;
                    break;
                }
                Ok(n) => {
                    let txt = String::from_utf8_lossy(&data[..n]);
                    println!("Received: {:?}", event);
                    println!("{txt}\n------\n");
                    // We do not break out of the loop since we have to read
                    // until 0 is returned (or an error) to be sure that 
                    // we've drained the buffer fully.
                }
                // Not ready to read in a non-blocking manner. This could
                // happen even if the event was reported as ready.
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
                // Reading from a stream could be interrupted by a signal from
                // the operating system. This should be expected and not 
                // probably not considered a failure.
                Err(e) if e.kind() == io::ErrorKind::Interrupted => break,
                Err(e) => return Err(e.into()),
            }
        }
    }
    Ok(handled_events)
}

fn get_req(path: &str) -> String {
    format!(
        "GET {path} HTTP/1.1\r\n\
             Host: localhost\r\n\
             Connection: close\r\n\
             \r\n"
    )
}
