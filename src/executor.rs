use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    sync::{Arc, Mutex},
    thread::Thread,
};

use crate::future_with_waker::{Future, PollState};

pub type Task = Box<dyn Future<Output = String>>;

thread_local! {
    // Executor that's currently running on this thread.
    static CURRENT_EXECUTOR: ExecutorCore = ExecutorCore::default();
}

/// Holds the state of the `Executor`
#[derive(Default)]
pub struct ExecutorCore {
    // Holds all the top level futures associated with the executor on this
    // thread and allows us to give each an ID property to identify them.
    // Since this will be called from a single thread, a `RefCell` will do
    // as there is no need for synchronization.
    pub tasks: RefCell<HashMap<usize, Task>>,

    // Stores IDs of tasks that should be polled by the executor.
    // A reference to this is passed to each Waker that this executor creates.
    // Since the Waker can (and will) be sent to a different thread and signal
    // that a specific task is ready by adding the task's ID to the queue,
    // we need to wrap this in an `Arc<Mutex<...>>`.
    pub ready_queue: Arc<Mutex<Vec<usize>>>,

    // This is a counter that gives out the next available ID, which means
    // it should never hand out the same ID twice for this executor instance.
    // We'll use this to give each top level future a unique ID. Since the
    // executor instance will only be accessible on the same thread it was
    // created on, a simple `Cell` will suffice in giving us the internal
    // mutability we need.
    pub next_id: Cell<usize>,
}

/// Allows us to register new top-level futures with our executor from anywhere
/// in our program.
pub fn spawn<F>(future: F)
where
    F: Future<Output = String> + 'static,
{
    CURRENT_EXECUTOR.with(|e| {
        // Get the next available ID.
        let id = e.next_id.get();
        println!("Spawning task with ID: {id}");
        // Assigns the ID to the future and store it in the HashMap.
        e.tasks.borrow_mut().insert(id, Box::new(future));
        // Adds the ID that represents this task to `ready_queue`, so that
        // it's polled at least once (recall that Future traits in Rust don't
        // do anything unless they're polled at least once).
        e.ready_queue.lock().map(|mut q| q.push(id)).unwrap();
        // Increment the ID by one.
        e.next_id.set(id + 1);
    });
}

// -----------------------------------------------------------------------------

pub struct Executor {}

impl Executor {
    pub fn new() -> Self {
        Self {}
    }

    /// This is the entry point for our executor. Often, you will pass in one
    /// top level future first, and when the top level future progresses, it
    /// will spawn new top-level futures onto our executor. Each new future can,
    /// of course, spawn new futures onto the Executor too, and that's how an
    /// asynchronous program basically works.
    ///
    /// `spawn` is similar to `std::thread::spawn`, with the exception that the
    /// tasks stay on the same OS thread. This means that the tasks won't be
    /// able to run in parallel, which in turn allows us to avoid any need for
    /// synchronization between tasks to avoid data races.
    pub fn block_on<F>(&mut self, future: F)
    where
        F: Future<Output = String> + 'static,
    {
        spawn(future);

        loop {
            while let Some(id) = self.pop_ready() {
                println!("Polling task with ID: {id}");

                // Remove future from the `tasks` collection.
                let mut future = match self.get_future(id) {
                    Some(f) => f,
                    // guard against false wakeups. `mio` doesn't guarantee
                    // that false wakeups won't happen.
                    None => continue,
                };
                // Create a new Waker instance. Remember that this Waker
                // instance now holds the ID property that identifies the
                // specific `Future` trait and a handle to the thread we are
                // currently running on.
                let waker = self.make_waker(id);
                match future.poll(&waker) {
                    // If `NotReady` we insert the task back into the `tasks`
                    // collection.  When a `Future` trait returns `NotReady`,
                    // we know that it will arrange it so that `Waker::wake`
                    // is called at a later point in time.  It's not the
                    // executor's responsibility to track the readiness of this
                    // future.
                    PollState::NotReady => self.insert_task(id, future),
                    // If the `Future` trait returns `Ready`, we simply continue
                    // to the next item in the ready queue. Since we took
                    // ownership of the future, this will drop the object before
                    // we enter the next iteration of the `while let` loop.
                    PollState::Ready(_) => continue,
                }
            }

            // Now that we've polled all the tasks in our ready queue, the first
            // thing we do is get a task count to see how many tasks we have
            // left.
            let task_count = self.task_count();
            let name = self.get_thread_name();
            if task_count > 0 {
                // If the task count is greater than 0, we park the thread.
                // Parking the thread will yield control to the OS scheduler,
                // and our `Executor` does nothing until it's woken up again.
                println!(
                    "{name}: {task_count} pending tasks, Sleep until notified."
                );
                std::thread::park();
                println!("{name}: Woken up");
            } else {
                // If the task count is 0, we're done with our asynchronous
                // program and exit the main loop.
                println!("{name}: All tasks are finished");
                break;
            }
        }
    }

    /// Pops off an ID that's ready from the ready queue.
    /// Since Waker pushes the ID to the back of the queue, and we pop off
    /// from the back as well, we essentially get a Last In First Out (LIFO)
    /// queue.
    fn pop_ready(&self) -> Option<usize> {
        CURRENT_EXECUTOR
            .with(|q| q.ready_queue.lock().map(|mut q| q.pop()).unwrap())
    }

    /// Takes the ID of a top level future, removes the future from the `tasks`
    /// collection and returns it (it the task is found). This means that if
    /// the task returns `NotReady` (signalling we're not done with it), we
    /// need to remember to add it back to the collection again.
    fn get_future(&self, id: usize) -> Option<Task> {
        CURRENT_EXECUTOR.with(|q| q.tasks.borrow_mut().remove(&id))
    }

    /// Create a new Waker instance.
    fn make_waker(&self, id: usize) -> Waker {
        let thread = std::thread::current();
        let ready_queue = CURRENT_EXECUTOR.with(|q| q.ready_queue.clone());
        Waker {
            thread,
            id,
            ready_queue,
        }
    }

    /// Taks an ID property and a Task property and inserts them into our
    /// `tasks` collection.
    fn insert_task(&self, id: usize, task: Task) {
        CURRENT_EXECUTOR.with(|q| q.tasks.borrow_mut().insert(id, task));
    }

    /// Returns the number of tasks that are currently in the queue.
    fn task_count(&self) -> usize {
        CURRENT_EXECUTOR.with(|q| q.tasks.borrow().len())
    }

    /// Returns the name of the thread that this executor is running on.
    fn get_thread_name(&self) -> String {
        std::thread::current()
            .name()
            .unwrap_or_default()
            .to_string()
    }
}

// -----------------------------------------------------------------------------

#[derive(Clone)]
pub struct Waker {
    // Handle to the thread.
    pub thread: Thread,

    // identifies the task associated with this waker
    pub id: usize,

    // This is a reference that can be shared between threads to a
    // Vec<usize>, where usize represents the ID of a task that's in the ready
    // queue. We share this object with the executor, so that we can push
    // the task ID associated with the Waker onto that queue when it's ready.
    pub ready_queue: Arc<Mutex<Vec<usize>>>,
}

impl Waker {
    /// When `wake` is called, we first take a lock on the Mutex that protects
    /// the ready queue we share with the executor. We then push the ID of
    /// the task that this Waker is associated with onto the ready queue.
    ///
    /// After that, we call `unpark` on the executor thread and wake it up.
    /// It will now find the task associated with this Waker in the ready
    /// queue and can call `poll` on it.
    pub fn wake(&self) {
        println!("Waker::wake, id: {}", self.id);
        self.ready_queue
            .lock()
            .map(|mut q| q.push(self.id))
            .unwrap();
        println!("Waking up thread: {:?}", self.thread.name());
        self.thread.unpark();
    }
}
