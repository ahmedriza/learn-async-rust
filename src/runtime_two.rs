use crate::{executor::Executor, reactor};

pub fn init() -> Executor {
    reactor::start();
    Executor::new()
}
