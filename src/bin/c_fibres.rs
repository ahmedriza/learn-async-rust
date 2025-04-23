// `cargo run --target x86_64-apple-darwin --bin c_fibres`

use std::arch::{asm, naked_asm};

pub const DEFAULT_STACK_SIZE: usize = 1024 * 1024 * 2;
pub const MAX_THREADS: usize = 4;

pub static mut RUNTIME: usize = 0;


// -----------------------------------------------------------------------------

#[derive(Debug, Default)]
#[repr(C)]
pub struct ThreadContext {
    rsp: u64,
    r15: u64,
    r14: u64,
    r13: u64,
    r12: u64,
    rbx: u64,
    rbp: u64,
}

// -----------------------------------------------------------------------------

pub struct Thread {
    stack: Vec<u8>,
    ctx: ThreadContext,
    state: State,
}

impl Thread {
    pub fn new() -> Self {
        let stack = vec![0; DEFAULT_STACK_SIZE];
        Thread {
            // Once a stack is allocated, it must not move. No `push()` on
            // the vector or any other methods that might trigger a relocation.
            // If the stack is reallocated, any pointers we hold to it are
            // invalidated.
            stack,
            ctx: ThreadContext::default(),
            state: State::Available,
        }
    }
}

// -----------------------------------------------------------------------------

#[derive(PartialEq, Eq, Debug)]
pub enum State {
    // Thread is available and ready to be assigned a task if needed
    Available,

    // Thread is running
    Running,

    // Thread is ready to move forward and resume execution
    Ready,
}

// -----------------------------------------------------------------------------

pub struct Runtime {
    threads: Vec<Thread>,
    __current: usize,
}

impl Runtime {
    pub fn new() -> Self {
        // The base thread ensures that we keep the runtime running util all
        // tasks are finished.
        let base_thread = Thread {
            stack: vec![0; DEFAULT_STACK_SIZE],
            ctx: ThreadContext::default(),
            state: State::Running,
        };

        let mut threads = vec![base_thread];
        let mut available_threads: Vec<Thread> =
            (1..MAX_THREADS).map(|_| Thread::new()).collect();
        threads.append(&mut available_threads);
        Runtime {
            threads,
            __current: 0,
        }
    }

    fn current(&self) -> usize {
        self.__current
    }

    fn set_current(&mut self, current: usize) {
        self.__current = current;
    }

    // After the call to `init()`, we have to make sure we don't do anything
    // that can invalidate the pointer we take to `self` once it's initialized.
    pub fn init(&self) {
        unsafe {
            let r_ptr: *const Runtime = self;
            RUNTIME = r_ptr as usize;
        }
    }

    // Continually call `t_yield` until it returns false, which means that
    // there is no more work to do and we can exit the process.
    pub fn run(&mut self) {
        while self.t_yield() {}
        std::process::exit(0);
    }

    // The user of our threads does not call this; we setup our stack so that
    // this is called when the task is done.
    pub fn t_return(&mut self) {
        // If the calling thread is the `base_thread`, we won't do anything.
        // Our runtime wil call `t_yield` for us on the base thread.  If it's
        // called on a spawned thread, we know it's finished since all threads
        // will have a `guard` function on top of their stack, and the only
        // place where this function is called is on our `guard` function.
        //
        // We'll set its state to `Available`, letting the runtime know that
        // it's ready to be assigned a new task, and then immediately call
        // `t_yield` to switch to the next thread which will schedule a new
        // thread to be run.
        let pos = self.current();
        if pos != 0 {
            self.threads[pos].state = State::Available;
            self.t_yield();
        }
    }

    // The first part of this function is our scheduler. We simply go through
    // all the threads and see if any are in the `Ready` state, which
    // indicates that it has a task ready to make progress on. This could be
    // a database call that has returned in a real-world application.
    //
    // If no thread is `Ready`, we're all done. This is an extremely simple
    // scheduler, using only a rounb-robin algorithm. A real scheduler might
    // have a much more sophisticated way of deciding what task to run next.
    //
    // If we find a thread that is ready to be run, we change the state of the
    // current thread from `Running` to `Ready`.
    //
    // The next thing we do is to call the function `switch`, which will save
    // the current context (the old context), and load the new context into
    // the CPU. The new context is either a new task or all the information
    // the CPU needs to resume work on an existing task.
    //
    // Our `switch` function takes two arguments and is marked as `#[naked]`.
    // Naked functions are not like normal functions. They don't accept formal
    // arguments, for example, so we can't simply call it in Rust as a normal
    // function like `switch(old, new)`.
    //
    // You see, usually, when we call a function with two arguments, the
    // compiler will place each argument in a register described by the calling
    // convention for the platform. However, when we call a naked function,
    // we need to take care of this ourselves. Therefore, we pass in the
    // address to our `old` and `new` `ThreadContext` using assembly.
    // `rdi` is the register for the first argument in the System V ABI calling
    // convention, and `rsi` is the register used for the second argument.
    //
    // The `#[inline(never)]` attribute prevents the compiler from simply
    // substituting a call to our function with a copy of the function content
    // wherever it's called (this is what inlining means).  This is almost
    // never a problem on debug builds, but in this case, our program will fail
    // if the compiler inlines this function in a release build. The issue
    // manifests itself by the runtime exiting before all the tasks are
    // finished. Since we store Runtime as a static usize that we then cast
    // as a `*mut pointer`, (which is almost guaranteed to cause UB), it's
    // most likely caused by the compiler making the wrong assumptions when
    // this function is inlined and called by casting and derferencing
    // `RUNTIME` in one of the helper methods that will be outlined. Just make
    // a note that this is probably avoidable if we change our design.
    #[inline(never)]
    fn t_yield(&mut self) -> bool {
        let mut pos = self.current();
        while self.threads[pos].state != State::Ready {
            pos += 1;
            if pos == self.threads.len() {
                pos = 0;
            }
            // Could not find a thread that is ready to run.  We are done.
            if pos == self.current() {
                return false;
            }
        }

        // We found a thread that is ready to run.  We need to switch to it.
        // Set the current thread to `Ready` and the new thread to `Running`.
        let _current = self.current();
        if self.threads[_current].state != State::Available {
            self.threads[_current].state = State::Ready;
        }
        self.threads[pos].state = State::Running;
        let old_pos = _current;
        self.set_current(pos);

        println!("\t\tSwitching from thread {} to thread {}", old_pos, pos);

        // The `clobber_abi("C")` tells the compiler that it may not assume
        // that any general-purpose registers are preserved across the asm!
        // block. The compiler will emit instructions to push the registers
        // it uses to the stack, and restore them when resuming after the
        // asm! block.
        unsafe {
            let __old: *mut ThreadContext = &mut self.threads[old_pos].ctx;
            let __new: *const ThreadContext = &self.threads[pos].ctx;
            asm!(
            "call _switch",
            in("rdi") __old,
            in("rsi") __new,
            clobber_abi("C")
            );
        }

        // This is just a way for us to prevent the compiler from optimizing
        // our code away.  The code never reaches this point, anyway.
        self.threads.len() > 0
    }

    pub fn spawn(&mut self, f: fn()) {
        let available = self
            .threads
            .iter_mut()
            .find(|t| t.state == State::Available)
            .expect("No available thread");
        let size = available.stack.len();
        unsafe {
            let s_ptr = available.stack.as_mut_ptr().offset(size as isize);
            let s_ptr = (s_ptr as usize & !0x0f) as *mut u8;
            std::ptr::write(s_ptr.offset(-16) as *mut u64, guard as u64);
            std::ptr::write(s_ptr.offset(-24) as *mut u64, skip as u64);
            std::ptr::write(s_ptr.offset(-32) as *mut u64, f as u64);
            available.ctx.rsp = s_ptr.offset(-32) as u64;

            println!(
                "Thread stack, size: {}, s_ptr: {:#018x}, rsp: {:#018x}",
                size, s_ptr as u64, available.ctx.rsp
            );
        }
        available.state = State::Ready;
    }
}

// -----------------------------------------------------------------------------

fn guard() {
    unsafe {
        let rt_ptr = RUNTIME as *mut Runtime;
        (*rt_ptr).t_return();
    }
}

#[unsafe(naked)]
unsafe extern "C" fn skip() {
    naked_asm!("ret")
}

pub fn yield_thread() {
    unsafe {
        let rt_ptr = RUNTIME as *mut Runtime;
        (*rt_ptr).t_yield();
    }
}

#[unsafe(naked)]
#[unsafe(no_mangle)]
// #[cfg_attr(target_os = "macos", unsafe(export_name = "\x01switch"))]
unsafe extern "C" fn switch() {
    // Save the current value of registers to the location pointed to by
    // the first argument (rdi).
    //
    // Then copy the values of the of the location pointed to by the second
    // argument (rsi) to the registers.  This is our new stack.
    naked_asm!(
        "mov [rdi + 0x00], rsp",
        "mov [rdi + 0x08], r15",
        "mov [rdi + 0x10], r14",
        "mov [rdi + 0x18], r13",
        "mov [rdi + 0x20], r12",
        "mov [rdi + 0x28], rbx",
        "mov [rdi + 0x30], rbp",
        "mov rsp, [rsi + 0x00]",
        "mov r15, [rsi + 0x08]",
        "mov r14, [rsi + 0x10]",
        "mov r13, [rsi + 0x18]",
        "mov r12, [rsi + 0x20]",
        "mov rbx, [rsi + 0x28]",
        "mov rbp, [rsi + 0x30]",
        "ret"
    );
}

fn main() {
    let mut runtime = Runtime::new();

    runtime.init();

    runtime.spawn(|| {
        println!("Thread 1 Starting");
        let id = 1;
        for i in 0..10 {
            println!("Thread: {} counter: {}", id, i);
            yield_thread();
        }
        println!("Thread 1 Finished");
    });

    runtime.spawn(|| {
        println!("Thread 2 Starting");
        let id = 2;
        for i in 0..15 {
            println!("Thread: {} counter: {}", id, i);
            yield_thread();
        }
        println!("Thread 2 Finished");
    });

    runtime.run();
}
