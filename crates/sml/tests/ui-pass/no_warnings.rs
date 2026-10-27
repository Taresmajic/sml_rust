//! Expansions must be warning-free, including when optional helpers are omitted.
#![deny(warnings)]

use sml::sml;

struct Ctx;
struct E;

fn act(_: &mut Ctx) {}

// No entry/exit/anonymous/composite helpers; `Done` has no rows; `E` binding unused.
sml! {
    machine Minimal<Ctx>;
    *A + E = Done,
}

// No events at all.
sml! {
    machine Eventless<Ctx>;
    *A = B,
}

// Exit action only.
sml! {
    machine ExitOnly<Ctx>;
    *A + E = B,
     A + on_exit / act,
}

// Entry action only.
sml! {
    machine EntryOnly<Ctx>;
    *A + E = B,
     B + on_entry / act,
}

fn main() {
    let _ = (
        sml::Sm::<Minimal>::new(Ctx),
        sml::Sm::<Eventless>::new(Ctx),
        sml::Sm::<ExitOnly>::new(Ctx),
        sml::Sm::<EntryOnly>::new(Ctx),
    );
}
