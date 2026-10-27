use sml::sml;
struct E;
fn g(_: &()) -> bool { true }

sml! {
    machine M;
    *A [g],
}

fn main() {}
