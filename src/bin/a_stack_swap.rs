use std::arch::asm;

pub const SSIZE: isize = 48;

#[derive(Debug, Default)]
#[repr(C)]
pub struct ThreadContext {
    rsp: u64,
}

fn main() {
    let mut ctx = ThreadContext::default();
    let mut stack = vec![0_u8; SSIZE as usize];

    unsafe {
        let stack_bottom = stack.as_mut_ptr().offset(SSIZE);
        let sb_aligned = (stack_bottom as usize & !15) as *mut u8;
        std::ptr::write(sb_aligned.offset(-16) as *mut u64, hello as u64);
        ctx.rsp = sb_aligned.offset(-16) as u64;
        gt_switch(&ctx);
    }
}

pub fn hello() {
    println!("I love waking up on a new stack!");
    loop {}
}

unsafe fn gt_switch(new: *const ThreadContext) {
    unsafe {
        asm!(
        "mov rsp, [{0} + 0x00]",
        "ret",
        in(reg) new,
        );
    }
}
