#[cfg(target_arch = "x86_64")]
mod poll_impl {
    use std::{io, net::TcpStream, os::fd::AsRawFd};

    use crate::ffi;

    type Events = Vec<ffi::Event>;

    // Represents the event queue.
    pub struct Poll {
        registry: Registry,
    }

    impl Poll {
        /// Create a new event queue.
        pub fn new() -> anyhow::Result<Self> {
            let res = unsafe { ffi::epoll_create(1) };
            if res < 0 {
                return Err(io::Error::last_os_error().into());
            }
            Ok(Self {
                registry: Registry { raw_fd: res },
            })
        }

        /// Return a reference to the Registry that we can use to register interest
        /// to be notified about new events.
        pub fn registry(&self) -> &Registry {
            &self.registry
        }

        /// Blocks the thread it's called on until an event is ready or times out,
        /// whichever occurs first.
        pub fn poll(
            &mut self,
            events: &mut Events,
            timeout: Option<i32>,
        ) -> anyhow::Result<()> {
            let fd = self.registry.raw_fd;
            let timeout = timeout.unwrap_or(-1);
            let max_events = events.capacity() as i32;

            // This call will block the current thread until an event is ready
            // or the timeout occurs.
            // The call will return 0 or more, telling us how many events have
            // occurred. We would get a value of 0 if the timeout occurs.
            let res = unsafe {
                ffi::epoll_wait(fd, events.as_mut_ptr(), max_events, timeout)
            };
            if res < 0 {
                return Err(io::Error::last_os_error().into());
            }

            // We know from the guarantee the operating system gives us that the
            // number of events it returns is pointing to valid data in our Vec
            // so this is safe.
            unsafe {
                events.set_len(res as usize);
            }

            Ok(())
        }
    }

    // -----------------------------------------------------------------------------

    /// A handle that allows us to register interest in new events.
    pub struct Registry {
        pub raw_fd: i32,
    }

    impl Registry {
        /// Register interest.
        ///
        /// The `interests` argument indicates what kind of events we want our
        /// event queue to keep track of.
        pub fn register(
            &self,
            source: &TcpStream,
            token: usize,
            interests: i32,
        ) -> anyhow::Result<()> {
            let mut event = ffi::Event {
                events: interests as u32,
                epoll_data: token as usize,
            };

            let op = ffi::EPOLL_CTL_ADD;
            let res = unsafe {
                ffi::epoll_ctl(self.raw_fd, op, source.as_raw_fd(), &mut event)
            };
            if res < 0 {
                return Err(io::Error::last_os_error().into());
            }

            Ok(())
        }
    }

    impl Drop for Registry {
        fn drop(&mut self) {
            let res = unsafe { ffi::close(self.raw_fd) };
            if res < 0 {
                let err = io::Error::last_os_error();
                eprintln!("Error closing epoll fd: {}", err);
            }
        }
    }
}
