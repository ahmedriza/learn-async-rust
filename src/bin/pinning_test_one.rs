//! 
//! See https://doc.rust-lang.org/std/pin/
//!
#[derive(Default)]
struct AddrTracker(Option<usize>);

impl AddrTracker {
    fn check_for_move(&mut self) {
        let _tmp = self as *mut Self;
        let current_addr = _tmp as usize;
        match self.0 {
            None => self.0 = Some(current_addr),
            Some(prev_addr) => assert_eq!(prev_addr, current_addr),
        }
    }
}

fn main() {
    // Create a tracker and store the initial address.
    let mut tracker = AddrTracker::default();
    tracker.check_for_move();

    // Here we shadow the variable. This carries a semantic move, and may
    // therefore also come with a mechanical memory *move*.
    let mut tracker = tracker;

    // will panic here!
    tracker.check_for_move();
}
