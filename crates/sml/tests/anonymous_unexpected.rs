//! Anonymous (eventless) transitions and `unexpected_event`.

use std::any::Any;

use sml::{Sm, sml};

#[derive(Default)]
struct Ctx {
    ready: bool,
    log: Vec<&'static str>,
}

struct Go;
struct Known;
struct Stray(u8);
struct Other;

fn ready(ctx: &Ctx) -> bool {
    ctx.ready
}
fn never(_: &Ctx, _: &Known) -> bool {
    false
}

sml! {
    machine Boot<Ctx>;
    *Init = Loading,
     Loading [ready] / (|c: &mut Ctx| c.log.push("loaded")) = Running,
     Loading + Go / (|c: &mut Ctx| c.ready = true),
     Running + Go = Init,
}

#[test]
fn anonymous_transitions_run_on_construction() {
    let sm = Sm::<Boot>::new(Ctx::default());
    assert!(sm.is(BootState::Loading));
}

#[test]
fn guarded_anonymous_transition_fires_after_internal_transition() {
    let mut sm = Sm::<Boot>::new(Ctx::default());
    assert!(sm.process_event(Go));
    assert!(sm.is(BootState::Running));
    assert_eq!(sm.context().log, ["loaded"]);
}

#[test]
fn anonymous_chain_runs_after_external_transition() {
    let mut sm = Sm::<Boot>::new(Ctx {
        ready: true,
        log: Vec::new(),
    });
    assert!(sm.is(BootState::Running));
    assert!(sm.process_event(Go));
    assert!(sm.is(BootState::Running));
    assert_eq!(sm.context().log, ["loaded", "loaded"]);
}

fn log_any(ctx: &mut Ctx, event: &dyn Any) {
    ctx.log
        .push(if event.is::<Other>() { "other" } else { "any" });
}
fn log_stray(ctx: &mut Ctx, stray: &Stray) {
    ctx.log.push(if stray.0 == 0 { "stray0" } else { "stray" });
}

sml! {
    machine Strict<Ctx>;
    *Idle + Known [never] = Idle,
     Idle + unexpected_event<Stray> / log_stray,
     Idle + unexpected_event<_> / log_any = Error,
     Error + unexpected_event = X,
}

#[test]
fn typed_unexpected_event_gets_the_event() {
    let mut sm = Sm::<Strict>::new(Ctx::default());
    assert!(sm.process_event(Stray(0)));
    assert!(sm.is(StrictState::Idle));
    assert_eq!(sm.context().log, ["stray0"]);
}

#[test]
fn wildcard_unexpected_event_gets_dyn_any() {
    let mut sm = Sm::<Strict>::new(Ctx::default());
    assert!(sm.process_event(Other));
    assert!(sm.is(StrictState::Error));
    assert_eq!(sm.context().log, ["other"]);
    assert!(sm.process_event(Go));
    assert!(sm.is_terminated());
}

#[test]
fn known_event_with_failing_guard_is_not_unexpected() {
    let mut sm = Sm::<Strict>::new(Ctx::default());
    assert!(!sm.process_event(Known));
    assert!(sm.is(StrictState::Idle));
    assert!(sm.context().log.is_empty());
}

fn always(_: &Ctx) -> bool {
    true
}

sml! {
    machine Fork<Ctx>;
    *Start [always] = Left,
     Start [always] = Right,
}

#[test]
fn only_first_eligible_anonymous_transition_fires() {
    let sm = Sm::<Fork>::new(Ctx::default());
    assert!(sm.is(ForkState::Left));
}
