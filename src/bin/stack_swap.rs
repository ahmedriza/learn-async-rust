/// Modified from the example from
/// https://github.com/PacktPublishing/Asynchronous-Programming-in-Rust/blob/main/ch05/a-stack-swap/src/main.rs
///
/// This example demonstrates how to swap the stack using assembly code. It
/// changes to a new stack and then calls a function. Then it returns to
/// another specified function before terminating.
///
/// The actual stack swap code is only for arm64 macOS as I have used
/// arch64 assembly code to swap the stack.
///
pub const SSIZE: isize = 48;

#[derive(Debug, Default)]
#[repr(C)]
pub struct ThreadContext {
    sp: u64,
    lr: u64,
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

        ctx.sp = s_ptr.offset(-16) as u64;
        ctx.lr = hello as u64;

        // When we print the stack, we should see the address of `t_return`
        // at the top of the stack.
        print_stack(s_ptr);

        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        {
            use std::arch::asm;
            let ctx_ptr: *mut ThreadContext = &mut ctx;
            println!();
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

#[unsafe(naked)]
#[unsafe(no_mangle)]
unsafe extern "C" fn context_switch() {
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        use std::arch::naked_asm;
        naked_asm! {
            // Get the address of the `t_return` function and copy it to the
            // link register, so that the link register is correctly setup when
            // `hello` is called.
            "ldr x10, [x0, #0x00]",
            "ldr x11, [x10]",
            "mov lr, x11",
            // Get the starting address of the `hello` function
            "ldr x12, [x0, #0x08]",
            "ret x12",
        }
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
