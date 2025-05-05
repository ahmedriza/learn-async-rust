use std::arch::asm;

fn main() {
    let message = "Hello, world!";
    let message = String::from(message);
    syscall(message);
}

fn syscall(message: String) {
    let mut buffer = message.as_bytes().to_vec();
    let len = buffer.len();
    let ptr = buffer.as_mut_ptr();
    unsafe {
        asm!(
            "mov x16, 4",
            "mov x0, 1",
            "svc 0",
            in("x1") ptr,
            in("x2") len,
            out("x16") _,
            out("x0") _,
            lateout("x1") _,
            lateout("x2") _,
        );
    }
}
