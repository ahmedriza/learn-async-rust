const EPOLLIN: i32 = 0x1;
const EPOLLET: i32 = 1 << 31;
const EPOLLONESHOT: i32 = 0x40000000;

fn main() {
    bit_flags();
}

fn bit_flags() {
    println!("EPOLLIN:      {EPOLLIN:032b}");
    println!("EPOLLET:      {EPOLLET:032b}");
    println!("EPOLLONESHOT: {EPOLLONESHOT:032b}");

    let bitmask = EPOLLIN | EPOLLET | EPOLLONESHOT;
    println!("");
    println!("bitmask:      {bitmask:032b}");

    check(bitmask);
}

fn check(bitmask: i32) {
    let read = bitmask & EPOLLIN != 0;
    let et = bitmask & EPOLLET != 0;
    let oneshot = bitmask & EPOLLONESHOT != 0;

    println!("read_event? {read}, edge_triggered? {et}, oneshot? {oneshot}");
}
