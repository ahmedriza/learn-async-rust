pub trait Future {
    type Output;
    fn poll(&mut self) -> PollState<Self::Output>;
}

pub enum PollState<T> {
    Ready(T),
    NotReady,
}

pub struct JoinAll<F: Future> {
    pub futures: Vec<(bool, F)>,
    pub finished_count: usize,
}

pub fn join_all<F: Future>(futures: Vec<F>) -> JoinAll<F> {
    let futures = futures.into_iter().map(|f| (false, f)).collect();
    JoinAll {
        futures,
        finished_count: 0,
    }
}

impl<F: Future> Future for JoinAll<F> {
    type Output = String;

    fn poll(&mut self) -> PollState<Self::Output> {
        for (finished, f) in self.futures.iter_mut() {
            if *finished {
                continue;
            }
            match f.poll() {
                PollState::Ready(_) => {
                    *finished = true;
                    self.finished_count += 1;
                }
                PollState::NotReady => continue,
            }
        }

        if self.finished_count == self.futures.len() {
            PollState::Ready(String::new())
        } else {
            PollState::NotReady
        }
    }
}
