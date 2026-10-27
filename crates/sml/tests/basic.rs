//! Transitions, guards, actions, internal/self transitions, terminate.

use sml::{Sm, sml};

#[derive(Default)]
struct Ctx {
    locked: bool,
    alarm: bool,
    log: Vec<&'static str>,
    last_code: u32,
}

struct Open;
struct Close;
struct Code(u32);
struct Ping;
struct Kick;
struct Unrelated;

fn is_unlocked(ctx: &Ctx) -> bool {
    !ctx.locked
}
fn alarm(ctx: &Ctx) -> bool {
    ctx.alarm
}
fn code_is_42(_: &Ctx, code: &Code) -> bool {
    code.0 == 42
}
fn store_code(ctx: &mut Ctx, code: &Code) {
    ctx.last_code = code.0;
}
fn log_open(ctx: &mut Ctx) {
    ctx.log.push("open");
}
fn log_close(ctx: &mut Ctx, _: &Close) {
    ctx.log.push("close");
}

sml! {
    machine Door<Ctx>;
    *Closed + Open [is_unlocked && !alarm] / log_open = Opened,
     Closed + Open [alarm || (|c: &Ctx| c.locked && c.alarm)] = Alarmed,
     Closed + Code [code_is_42] / (store_code, |c: &mut Ctx| c.log.push("unlocked")) = Closed,
     Closed + Code / store_code,
     Closed + on_entry / (|c: &mut Ctx| c.log.push("enter closed")),
     Closed + on_exit / (|c: &mut Ctx| c.log.push("exit closed")),
     Opened + Close / log_close = Closed,
     Opened + Ping / (|c: &mut Ctx| c.log.push("ping")),
     Opened + Kick = X,
     Alarmed + Kick = X,
}

fn door() -> Sm<Door> {
    Sm::new(Ctx::default())
}

#[test]
fn initial_state_entered_on_construction() {
    let sm = door();
    assert!(sm.is(DoorState::Closed));
    assert!(!sm.is(DoorState::Opened));
    assert_eq!(sm.context().log, ["enter closed"]);
}

#[test]
fn guarded_external_transition_runs_exit_action_entry() {
    let mut sm = door();
    assert!(sm.process_event(Open));
    assert_eq!(sm.state(), DoorState::Opened);
    assert_eq!(sm.context().log, ["enter closed", "exit closed", "open"]);
    assert!(sm.process_event(Close));
    assert_eq!(sm.context().log[3..], ["close", "enter closed"]);
}

#[test]
fn failing_guards_leave_state_and_return_false() {
    let mut sm = door();
    sm.context_mut().locked = true;
    assert!(!sm.process_event(Open));
    assert!(sm.is(DoorState::Closed));
    assert_eq!(sm.context().log, ["enter closed"]);
}

#[test]
fn first_matching_row_wins_in_declaration_order() {
    let mut sm = door();
    sm.context_mut().alarm = true;
    assert!(sm.process_event(Open));
    assert!(sm.is(DoorState::Alarmed));
}

#[test]
fn closure_guard_inside_composition() {
    let mut sm = door();
    sm.context_mut().locked = true;
    sm.context_mut().alarm = true;
    assert!(sm.process_event(Open));
    assert!(sm.is(DoorState::Alarmed));
}

#[test]
fn self_transition_exits_and_reenters() {
    let mut sm = door();
    assert!(sm.process_event(Code(42)));
    assert_eq!(sm.context().last_code, 42);
    assert_eq!(
        sm.context().log,
        ["enter closed", "exit closed", "unlocked", "enter closed"]
    );
}

#[test]
fn internal_transition_skips_exit_and_entry() {
    let mut sm = door();
    assert!(sm.process_event(Code(7)));
    assert_eq!(sm.context().last_code, 7);
    assert_eq!(sm.context().log, ["enter closed"]);

    sm.process_event(Open);
    assert!(sm.process_event(Ping));
    assert!(sm.is(DoorState::Opened));
    assert_eq!(sm.context().log.last(), Some(&"ping"));
}

#[test]
fn unknown_event_is_ignored() {
    let mut sm = door();
    assert!(!sm.process_event(Unrelated));
    assert!(!sm.process_event(Close));
    assert!(sm.is(DoorState::Closed));
}

#[test]
fn terminate_state_ignores_all_events() {
    let mut sm = door();
    assert!(!sm.is_terminated());
    sm.process_event(Open);
    assert!(sm.process_event(Kick));
    assert!(sm.is_terminated());
    assert!(sm.is(DoorState::X));
    assert!(!sm.process_event(Close));
    assert!(sm.is(DoorState::X));
}

#[test]
fn into_context_returns_owned_context() {
    let mut sm = door();
    sm.process_event(Code(9));
    assert_eq!(sm.into_context().last_code, 9);
}

sml! {
    machine Trivial;
    *Only + Ping = Only,
}

#[test]
fn machine_without_context_uses_unit() {
    let mut sm = Sm::<Trivial>::new(());
    assert!(sm.process_event(Ping));
    assert!(sm.is(TrivialState::Only));
    assert!(!sm.is_terminated());
}
