//!
//! See https://doc.rust-lang.org/std/pin/
//!
//! The guarantee of a stable address is necessary to make `AddrTracker` work.
//! When `check_for_move` sees a `Pin<&mut AddrTracker>`, it can safely assume
//! that value will exist at the samd address until said value goes out of
//! scope, and thus multiple calls to it *cannot* panic.
//!
//! Note that this invariant is enforced by simply making it impossible to call
//! code that would perform a move on the pinned value. This is the case since
//! the only way to access the pinned vlaue is through the pinning
//! `Pin<&mut T>`, which in turn restricts our access.
//!
use std::marker::PhantomPinned;
use std::pin::Pin;
use std::pin::pin;

#[derive(Default)]
struct AddrTracker {
    prev_addr: Option<usize>,
    // remove auto-implemented `Unpin` bound to mark this type as having some
    // address sensitive state. This is essential for our expected pinning
    // guarantees to work.
    //
    // If a type contains `PhantomPinned`, it wll not implement `Unpin` by default.
    _pin: PhantomPinned,
}

impl AddrTracker {
    fn check_for_move(self: Pin<&mut Self>) {
        let _tmp = &*self as *const Self;
        let current_addr = _tmp as usize;

        match self.prev_addr {
            None => {
                // SAFETY: we do not move out of self
                let self_data_mut = unsafe { self.get_unchecked_mut() };
                self_data_mut.prev_addr = Some(current_addr);
            }
            Some(prev_addre) => assert_eq!(prev_addre, current_addr),
        }
    }
}

fn main() {
    // 1. Create the value, not yet in an address-sensitive state.
    let tracker = AddrTracker::default();

    // 2. Pin the value by putting it behind a pinning pointer, thus putting
    // it into an address sensitive state.
    let mut ptr_to_pinned_tracker: Pin<&mut AddrTracker> = pin!(tracker);

    ptr_to_pinned_tracker.as_mut().check_for_move();

    // Trying to access `tracker` or pass `ptr_to_pinned_tracker` to anything
    // that requires mutable access to a non-pinned version of it will no
    // longer compile

    // 3. We can now assume that the tracker value will never be moved, thus
    // this will never panic!
    ptr_to_pinned_tracker.as_mut().check_for_move();
}
