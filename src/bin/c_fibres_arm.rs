use std::arch::{asm, naked_asm};

pub const DEFAULT_STACK_SIZE: usize = 1024 * 1024 * 2;
pub const MAX_THREADS: usize = 4;

pub static mut RUNTIME: usize = 0;

#[derive(Debug, Default)]
#[repr(C)]
pub struct ThreadContext {
    sp: u64,
    x28: u64,
    x27: u64,
    x26: u64,
    x25: u64,
    x24: u64,
    x23: u64,
    x22: u64,
    x21: u64,
    x20: u64,
    x19: u64,
    fp: u64, // fp
    lr: u64, // lr, contains the return address
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

    // After the call to `init()`, we have to make sure we don't do anything
    // that can invalidate the pointer we take to `self` once it's initialized.
    pub fn init(&self) {
        unsafe {
            let r_ptr: *const Runtime = self;
            RUNTIME = r_ptr as usize;
        }
    }

    pub fn run(&mut self) {
        while self.t_yield() {}
        std::process::exit(0);
    }

    pub fn t_return(&mut self) {
        let pos = self.current();
        if pos != 0 {
            // println!("\t\tThread {} finished", pos);
            self.threads[pos].state = State::Available;
            self.t_yield();
        }
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
            available.ctx.sp = s_ptr.offset(-32) as u64;

            available.ctx.fp = s_ptr.offset(-32) as u64;
            available.ctx.lr = f as u64;

            println!(
                "Thread stack, size: {}, s_ptr: {:#018x}, rsp: {:#018x}",
                size, s_ptr as u64, available.ctx.sp
            );
        }
        available.state = State::Ready;
    }

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

        println!("\tSwitching from thread {} to thread {}", old_pos, pos);

        // let current_thread = &self.threads[pos];
        // println!("\t\tCurrent thread sp: {:#018x}", current_thread.ctx.sp);
        // println!("\t\tCurrent thread fp: {:#018x}", current_thread.ctx.fp);
        // println!("\t\tCurrent thread lr: {:#018x}", current_thread.ctx.lr);

        // The `clobber_abi("C")` tells the compiler that it may not assume
        // that any general-purpose registers are preserved across the asm!
        // block. The compiler will emit instructions to push the registers
        // it uses to the stack, and restore them when resuming after the
        // asm! block.
        unsafe {
            let __old: *mut ThreadContext = &mut self.threads[old_pos].ctx;
            let __new: *const ThreadContext = &self.threads[pos].ctx;
            asm!(
            "bl _switch",
            in("x0") __old,
            in("x1") __new,
            clobber_abi("C")
            );
        }

        // This is just a way for us to prevent the compiler from optimizing
        // our code away.  The code never reaches this point, anyway.
        self.threads.len() > 0
    }

    fn current(&self) -> usize {
        self.__current
    }

    fn set_current(&mut self, current: usize) {
        self.__current = current;
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
unsafe extern "C" fn switch() {
    naked_asm!(
        // Save the context of the current thread
        // Save the current value of registers to the location pointed to by
        // the x0 register.
        "mov x2, sp",
        "str x2,  [x0]",
        "str x28, [x0, #0x08]",
        "str x27, [x0, #0x10]",
        "str x26, [x0, #0x18]",
        "str x25, [x0, #0x20]",
        "str x24, [x0, #0x28]",
        "str x23, [x0, #0x30]",
        "str x22, [x0, #0x38]",
        "str x21, [x0, #0x40]",
        "str x20, [x0, #0x48]",
        "str x19, [x0, #0x50]",
        "str x29, [x0, #0x58]",
        "str x30, [x0, #0x60]",

        // "stp x29, x30, [sp, -16]!",
        // "stp x19, x20, [sp, -16]!",
        // "stp x21, x22, [sp, -16]!",
        // "stp x23, x24, [sp, -16]!",
        // "stp x25, x26, [sp, -16]!",
        // "stp x27, x28, [sp, -16]!",

        // Switch to the next thread
        // "ldr r0, =RUNTIME",
        // "ldr r0, [r0]",
        // "ldr r1, [r0]",
        // "str r1, [r0]",

        // Restore the context of the next thread
        // "ldp x27, x28, [sp], 16",
        // "ldp x25, x26, [sp], 16",
        // "ldp x23, x24, [sp], 16",
        // "ldp x21, x22, [sp], 16",
        // "ldp x19, x20, [sp], 16",
        // "ldp x29, x30, [sp], 16",
        
        "ldr x29, [x1]", // set the frame pointer
        "ldr x28, [x1, #0x08]",
        "ldr x27, [x1, #0x10]",
        "ldr x26, [x1, #0x18]",
        "ldr x25, [x1, #0x20]",
        "ldr x24, [x1, #0x28]",
        "ldr x23, [x1, #0x30]",
        "ldr x22, [x1, #0x38]",
        "ldr x21, [x1, #0x40]",
        "ldr x20, [x1, #0x48]",
        "ldr x19, [x1, #0x50]",
        // "ldr x29, [x1, #0x58]", // set the frame pointer
        "ldr x30, [x1, #0x60]", // set the link register
        // Return to the next thread
        "ret"
    );
}

pub fn main() {
    let mut runtime = Runtime::new();

    runtime.init();

    println!("Runtime initialized");

    runtime.spawn(|| {
        println!("Thread: 1 Starting");
        let id = 1;
        for i in 0..10 {
            println!("Thread: {} counter: {}", id, i);
            yield_thread();
        }
        println!("Thread: 1 Finished");
    });

    runtime.spawn(|| {
        println!("Thread: 2 Starting");
        let id = 2;
        for i in 0..15 {
            println!("Thread: {} counter: {}", id, i);
            yield_thread();
        }
        println!("Thread: 2 Finished");
    });

    runtime.run();
}
