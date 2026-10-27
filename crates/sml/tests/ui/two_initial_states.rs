use sml::sml;
struct E;
fn g(_: &()) -> bool { true }

sml! {
    machine M;
    *A + E = B,
    *B + E = A,
}

fn main() {}
