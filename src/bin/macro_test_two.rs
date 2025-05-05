// Shorthand for initializing a `String`.
macro_rules! S {
    ($e:expr) => {
        String::from($e)
    };
}

macro_rules! four {
    () => {
        1 + 3
    };
}

macro_rules! gibberish {
    (4 fn ['spang "whammo"] @_@) => {
        let _foo = "foo";
        println!("Gibberish!");
    };
}

macro_rules! vec_strs {
    (
        $(
            $element:expr
        ),*) => {
        {
            let mut v = Vec::new();
            $(
                v.push(format!("{}", $element));
            )*
            v
        }
    };
}

macro_rules! repeat_two {
    ($($i:ident)*, $($i2:ident)*) => {
        $( let $i: (); let $i2: (); )*
    }
}

fn main() {
    let world = S!("World");
    println!("Hello, {}!", world);
    let four = four!();
    println!("four: {}", four);
    gibberish!(4 fn ['spang "whammo"] @_@);

    let v = vec_strs! { 1, 2 };
    println!("v: {:?}", v);

    repeat_two! { _a _b, _c _d }
}
