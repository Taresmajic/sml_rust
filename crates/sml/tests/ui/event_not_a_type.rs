use sml::sml;
struct E;
fn g(_: &()) -> bool { true }

sml! {
    machine M;
    *A + 42 = B,
}

fn main() {}
