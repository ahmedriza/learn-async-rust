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
