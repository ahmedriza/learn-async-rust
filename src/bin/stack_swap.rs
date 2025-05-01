use std::arch::naked_asm;

/// Based on the example from
/// https://github.com/PacktPublishing/Asynchronous-Programming-in-Rust/blob/main/ch05/a-stack-swap/src/main.rs
///
/// The actual stack swap code is only for arm64 macOS as I have used
/// arch64 assembly code to swap the stack.
///
pub const SSIZE: isize = 48;

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

fn main() {
    let mut ctx = ThreadContext::default();
    let mut stack = vec![0_u8; SSIZE as usize];
    //
    // The stack grows downwards, so the bottom of the stack is at a higher
    // address than the top of the stack.
    //
    // 0x30 [47] <-- bottom of the stack
    // ...
    // 0x02 [2]
    // 0x01 [1]
    // 0x00 [0] <-- top of the stack
    //
    unsafe {
        let s_ptr = stack.as_mut_ptr().offset(SSIZE);
        // align the stack pointer to 16 bytes
        let s_ptr = (s_ptr as usize & !0x0f) as *mut u8;
        std::ptr::write(s_ptr.offset(-16) as *mut u64, t_return as u64);

        let sp = s_ptr.offset(-16) as u64;
        ctx.sp = sp;
        ctx.lr = hello as u64;

        print_stack(s_ptr);

        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        {
            use std::arch::asm;
            let ctx_ptr: *mut ThreadContext = &mut ctx;
            asm!(
                "bl _context_switch",
                in("x0") ctx_ptr,
                clobber_abi("C"),
            );
        }

        println!("Returned to main()");
    }
}

// The function that will be called when the context switch
// is done.
fn t_return() {
    println!("Returned to t_return()");
    std::process::exit(0);
}

// #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
#[unsafe(naked)]
#[unsafe(no_mangle)]
unsafe extern "C" fn context_switch() {
    naked_asm! {
        // Save the current context
        "str x19, [sp, #0x00]",
        "str x20, [sp, #0x08]",
        "str x21, [sp, #0x10]",
        "str x22, [sp, #0x18]",
        "str x23, [sp, #0x20]",
        "str x24, [sp, #0x28]",
        "str x25, [sp, #0x30]",
        "str x26, [sp, #0x38]",
        "str x27, [sp, #0x40]",
        "str x28, [sp, #0x48]",
        "str x29, [sp, #0x50]",
        "str x30, [sp, #0x58]",
        //
        // Load the new context
        "ldr x19, [x0, #0x00]",
        "ldr x20, [x0, #0x08]",
        "ldr x21, [x0, #0x10]",
        "ldr x22, [x0, #0x18]",
        "ldr x23, [x0, #0x20]",
        "ldr x24, [x0, #0x28]",
        "ldr x25, [x0, #0x30]",
        "ldr x26, [x0, #0x38]",
        "ldr x27, [x0, #0x40]",
        "ldr x28, [x0, #0x48]",
        //
        "ldr x10, [x0, #0x50]",
        "mov sp, x10",
        //
        "ldr fp, [x0, #0x58]",
        "ldr lr, [x0, #0x60]",
        //
        // Save the current lr to x11.
        "mov x11, lr",
        //
        // Load the return address of the fuction we want to call from the stack
        "ldr x10, [sp]",
        "mov lr , x10",
        //
        "ret x11",
    }
}

pub fn hello() {
    println!("I love waking up on a new stack!");
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
