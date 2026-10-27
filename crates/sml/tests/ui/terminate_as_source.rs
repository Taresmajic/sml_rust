use sml::sml;
struct E;
fn g(_: &()) -> bool { true }

sml! {
    machine M;
    *A + E = X,
     X + E = A,
}

fn main() {}
