//!
//! https://veykril.github.io/tlborm/decl-macros/macros-practical.html
//!
macro_rules! count_exprs {
    () => (0);
    ($head:expr) => (1);
    ($head:expr, $($tail:expr),*) => (1 + count_exprs!($($tail),*));
}

macro_rules! recurrence {
    ( $seq:ident [ $ind:ident ]: $sty:ty = $($inits:expr),+ ; ... ; $recur:expr ) => {{
        use std::ops::Index;

        const MEM_SIZE : usize = count_exprs!($($inits),+);

        struct Recurrence {
            pub mem: [$sty; MEM_SIZE],
            pub pos: usize,
        }

        struct IndexOffset<'a> {
            pub slice: &'a [$sty; MEM_SIZE],
            pub offset: usize,
        }

        impl<'a> Index<usize> for IndexOffset<'a> {
            type Output = $sty;

            #[inline(always)]
            fn index<'b>(&'b self, index: usize) -> &'b $sty {
                use std::num::Wrapping;

                let index = Wrapping(index);
                let offset = Wrapping(self.offset);
                let window = Wrapping(MEM_SIZE);

                let real_index = index - offset + window;
                &self.slice[real_index.0]
            }
        }

        impl Iterator for Recurrence {
            type Item = $sty;

            fn next(&mut self) -> Option<Self::Item> {
                if self.pos < MEM_SIZE {
                    let next_val = self.mem[self.pos];
                    self.pos += 1;
                    Some(next_val)
                } else {
                    let next_val = {
                        let $ind = self.pos;
                        let $seq = IndexOffset {
                            slice: &self.mem,
                            offset: $ind,
                        };
                        $recur
                    };
                    // shuffle down and append
                    {
                        use std::mem::swap;

                        let mut swap_tmp = next_val;
                        for i in (0..MEM_SIZE).rev() {
                            swap(&mut swap_tmp, &mut self.mem[i]);
                        }
                    }

                    self.pos += 1;
                    Some(next_val)
                }
            }
        }

        Recurrence {
            mem: [$($inits),+],
            pos: 0,
        }
    }};
}

fn main() {
    let fib = recurrence![a[n]: u64 = 0, 1; ... ; a[n-1] + a[n-2]];

    for e in fib.take(10) {
        println!("{e}");
    }

    println!();

    for e in recurrence!(f[i]: f64 = 1.0; ...; f[i-1] * i as f64).take(10) {
        println!("{e}");
    }
}

// -----------------------------------------------------------------------------
