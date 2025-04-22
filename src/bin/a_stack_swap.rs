use std::arch::asm;

/// To run this example on MacOS:
///
/// * `$env /usr/bin/arch -x86_64 /bin/zsh --login` in the terminal to force
/// the current session to emulate a x86-64 architecture.
///
/// * `rustup target add x86_64-apple-darwin` if not done already
///
/// * `cargo run --target x86_64-apple-darwin --bin a_stack_swap`

pub const SSIZE: isize = 48;

pub static mut THREAD_CONTEXT_PTR: *mut u8 = std::ptr::null_mut();

#[derive(Debug, Default)]
#[repr(C)]
pub struct ThreadContext {
    rsp: u64,
    saved_rbp: u64,
    return_addr: u64,
}

fn main() {
    let mut ctx = ThreadContext::default();
    let mut stack = vec![0_u8; SSIZE as usize];

    // [0] <-- top of the stack
    // [1]
    // [2]
    // ...
    // [47] <-- bottom of the stack

    unsafe {
        let stack_bottom = stack.as_mut_ptr().offset(SSIZE);
        let sb_aligned = (stack_bottom as usize & !0xf) as *mut u8;

        let sp = sb_aligned.offset(-16);

        println!("Stack bottom:         {:#018x}", stack_bottom as usize);
        println!("Stack bottom aligned: {:#018x}", sb_aligned as usize);
        println!("Stack pointer:        {:#018x}", sp as usize);
        println!("Address of hello:     {:#018x}", hello as usize);

        std::ptr::write(sp as *mut u64, hello as u64);
        ctx.rsp = sp as u64;

        // print_stack(sb_aligned);

        let rbp = get_rbp();

        ctx.saved_rbp = rbp;
        println!("saved rbp:            {:#018x}", rbp);

        THREAD_CONTEXT_PTR = &mut ctx as *mut _ as *mut u8;

        gt_switch(&mut ctx);

        println!("Returned to main()");
    }
}

unsafe fn gt_switch(ctx: *mut ThreadContext) {
    unsafe {
        // The compiler would have placed the return address on the stack.
        // However, it'd also have pushed the base pointer (rbp) on the stack.
        // This means that the return address is at [rbp + 0x08].
        let return_addr: u64;
        asm!(
            "mov {0}, [rbp + 0x08]",
            out(reg) return_addr,
        );
        (*ctx).return_addr = return_addr;

        // Now we can switch to the new stack.  The top of the stack
        // points to the address of the function we want to call.
        asm!(
        "mov rsp, [{0} + 0x00]",
        "ret",
        in(reg) ctx,
        );
    }
}

pub fn hello() {
    println!("I love waking up on a new stack!");

    // Before returning, we need to restore the stack pointer
    // and the base pointer
    // and then arrange to return to the caller
    unsafe {
        let ctx = THREAD_CONTEXT_PTR.offset(0x00) as *mut ThreadContext;
        let rbp = (*ctx).saved_rbp;
        let return_addr = (*ctx).return_addr;

        println!("rbp:                  {:#018x}", rbp);
        println!("return_addr:          {:#018x}", return_addr);

        asm!(
            "mov rsp, [{0}]",
            in(reg) rbp,
        );

        println!("About to return from hello()");

        asm!(
            "push {0}",
            "push {1}",
            in(reg) return_addr,
            in(reg) rbp,
        );
    }

    // loop {}
}

pub fn get_rbp() -> u64 {
    let rbp: u64;
    unsafe {
        asm!(
            "mov {0}, rbp", out(reg) rbp,
        );
    }
    println!("rbp:                  {:#018x}", rbp);
    return rbp;
}

#[allow(unused)]
unsafe fn print_stack(sb_aligned: *mut u8) {
    unsafe {
        for i in 0..SSIZE {
            println!(
                "i: {:3}, mem: {:#018x}, val: {:02x}",
                SSIZE - i,
                sb_aligned.offset(-i as isize) as usize,
                *sb_aligned.offset(-i as isize)
            );
        }
        println!();
    }
}
