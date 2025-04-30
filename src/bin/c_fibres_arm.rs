use std::arch::{asm, naked_asm};

pub const DEFAULT_STACK_SIZE: usize = 1024 * 1024 * 2;
pub const MAX_THREADS: usize = 4;

pub static mut RUNTIME: usize = 0;

#[derive(Debug, Default)]
#[repr(C)]
pub struct ThreadContext {
    // callee saved registers, x19 to x28
    x19: u64,
    x20: u64,
    x21: u64,
    x22: u64,
    x23: u64,
    x24: u64,
    x25: u64,
    x26: u64,
    x27: u64,
    x28: u64,
    // x31 or stack pointer
    sp: u64,
    // x29 or frame pointer
    fp: u64,
    // x30 or link register
    lr: u64, // x30 or link register contains the return address
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

    pub fn run(&mut self) {
        while self.t_yield() {}
        println!("\t\trun():\t\tThread {} exiting", self.current());
        std::process::exit(0);
    }

    pub fn t_return(&mut self) {
        let pos = self.current();
        if pos != 0 {
            self.threads[pos].state = State::Available;
            self.t_yield();
        }
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
                println!(
                    "\t\tt_yield():\tCurrent thread: {}, no thread is ready \
                    to run, exiting",
                    self.current()
                );
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
            std::ptr::write(s_ptr.offset(-32) as *mut u64, guard as u64);
            available.ctx.sp = s_ptr.offset(-32) as u64;
            available.ctx.lr = f as u64;

            println!(
                "Thread stack, size: {}, s_ptr: {:#018x}, rsp: {:#018x}",
                size, s_ptr as u64, available.ctx.sp
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

#[allow(unused)]
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
        // the x0 register. Use x10 as a temporary register.
        "str x19, [x0, #0x00]",
        "str x20, [x0, #0x08]",
        "str x21, [x0, #0x10]",
        "str x22, [x0, #0x18]",
        "str x23, [x0, #0x20]",
        "str x24, [x0, #0x28]",
        "str x25, [x0, #0x30]",
        "str x26, [x0, #0x38]",
        "str x27, [x0, #0x40]",
        "str x28, [x0, #0x48]",
        "mov x10, sp",
        "str x10, [x0, #0x50]",
        "str fp,  [x0, #0x58]",
        "str lr,  [x0, #0x60]",
        //
        //
        // Set up the new thread context.  Load the values of the registers
        // from the location pointed to by x1. Use x10 and x11 as temporary
        // registers.
        "ldr x19, [x1, #0x00]",
        "ldr x20, [x1, #0x08]",
        "ldr x21, [x1, #0x10]",
        "ldr x22, [x1, #0x18]",
        "ldr x23, [x1, #0x20]",
        "ldr x24, [x1, #0x28]",
        "ldr x25, [x1, #0x30]",
        "ldr x26, [x1, #0x38]",
        "ldr x27, [x1, #0x40]",
        "ldr x28, [x1, #0x48]",
        // Load the stack pointer from the new thread context.
        // We can't directly load the stack pointer from the address in x1.
        // First load the value of the address in x1 to x10, then set the stack
        // pointer to the value in x10.
        "ldr x10, [x1, #0x50]",
        "mov sp, x10",
        "ldr fp, [x1, #0x58]",
        "ldr lr, [x1, #0x60]",
        // Save the current link register to x11.  This is the address at which
        // the thread will start or resume execution from.
        "mov x11, lr",
        //
        // Load the return address from the stack. This is the address that
        // the function will jump to when it returns.
        // This is only going to be used when the newly spawned thread starts
        // executing, since, in the `spawn()` function, we put the
        // address of the `guard()` function in sp.
        // In all other cases, what we set to `lr` here doesn't really matter,
        // since the functions will take care of storing the actual `lr` and
        // restoring them during normal function prologue and epilogue.
        "ldr x10, [sp]",
        "mov lr, x10", // set the link register to address in sp
        //
        // jump to the location in x11 which is the actual `lr`.
        "ret x11",
    );
}

#[allow(unused)]
fn f() {
    println!("\t\tf():\t\tThread: 1 Starting");
    let id = 1;
    for i in 0..2 {
        println!("\t\tf():\t\tThread: {} counter: {}", id, i);
        yield_thread();
    }
    println!("\t\tf():\t\tThread 1 Finished");
}

#[allow(unused)]
fn g() {
    println!("\t\tg():\t\tThread: 2 Starting");
    let id = 2;
    for i in 0..15 {
        println!("\t\tg():\t\tThread: {} counter: {}", id, i);
        yield_thread();
    }
    println!("\t\tg():\t\tThread 2 Finished");
}

pub fn main() {
    let mut runtime = Runtime::new();

    runtime.init();

    // runtime.spawn(f);
    // runtime.spawn(g);

    runtime.spawn(|| {
        println!("\t\tThread: 1 Starting");
        let id = 1;
        for i in 0..=10 {
            println!("\t\tThread: {} counter: {}", id, i);
            yield_thread();
        }
        println!("\t\tThread: 1 Finished");
    });

    runtime.spawn(|| {
        println!("\t\tThread: 2 Starting");
        let id = 2;
        for i in 0..=15 {
            println!("\t\tThread: {} counter: {}", id, i);
            yield_thread();
        }
        println!("\t\tThread: 2 Finished");
    });

    runtime.run();
}
