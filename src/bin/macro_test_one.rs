fn main() {}

macro_rules! test_battery {
    ($($t:ty as $name:ident),*) => {
        $(
            mod $name {
                use super::*;
                #[allow(unused)]
                pub fn frobnified() {
                    test_inner::<$t>(1, true);
                }
                #[allow(unused)]
                pub fn unfrobnified() {
                    test_inner::<$t>(1, false);
                }
            }
            )*
    }
}

test_battery! {
    u8 as u8_tests,
    i128 as i128_tests
}

pub fn test_inner<T: std::fmt::Debug>(init: T, frobnify: bool) {
    println!("init: {init:?}, frobnify: {frobnify}");
}
