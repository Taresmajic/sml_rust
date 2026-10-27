use sml::sml;
struct E;
fn bad(_: &u32) {}

sml! {
    machine M;
    *A + E / bad = B,
}

fn main() {}
