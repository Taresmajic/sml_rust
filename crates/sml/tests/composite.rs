//! Composite states (`state<Sub>`) and history `(H)`.

use sml::{Sm, StateMachine, sml};

#[derive(Default)]
struct Ctx {
    log: Vec<&'static str>,
}

struct Next;
struct Leave;
struct Back;
struct Finish;

fn sub_done(ctx: &Ctx) -> bool {
    ctx.log.last() == Some(&"finished")
}

sml! {
    machine Inner<Ctx>;
    *A + Next = B,
     A + on_entry<_> / (|c: &mut Ctx| c.log.push("enter A")),
     A + on_exit<_> / (|c: &mut Ctx| c.log.push("exit A")),
     B + on_entry / (|c: &mut Ctx| c.log.push("enter B")),
     B + on_exit / (|c: &mut Ctx| c.log.push("exit B")),
     B + Finish / (|c: &mut Ctx| c.log.push("finished")) = X,
}

sml! {
    machine Remembering<Ctx>;
    *A(H) + Next = B,
     B + Next = A,
}

sml! {
    machine Outer<Ctx>;
    *Idle + Next = state<Inner>,
     state<Inner> + on_entry / (|c: &mut Ctx| c.log.push("enter Inner")),
     state<Inner> + on_exit / (|c: &mut Ctx| c.log.push("exit Inner")),
     Inner + Next = Idle,
     Inner + Leave = Idle,
     Inner [sub_done] = Done,
     Idle + Back = state<Remembering>,
     Remembering + Leave = Idle,
}

fn outer() -> Sm<Outer> {
    Sm::new(Ctx::default())
}

#[test]
fn entering_composite_enters_outer_then_inner_initial() {
    let mut sm = outer();
    assert!(sm.process_event(Next));
    assert!(sm.is(OuterState::Inner));
    assert_eq!(sm.sub::<Inner>().state(), InnerState::A);
    assert_eq!(sm.context().log, ["enter Inner", "enter A"]);
}

#[test]
fn sub_machine_handles_event_before_parent() {
    let mut sm = outer();
    sm.process_event(Next);
    assert!(sm.process_event(Next));
    assert!(sm.is(OuterState::Inner));
    assert_eq!(sm.sub::<Inner>().state(), InnerState::B);
}

#[test]
fn parent_handles_event_sub_machine_ignores_and_exits_inner_first() {
    let mut sm = outer();
    sm.process_event(Next);
    sm.context_mut().log.clear();
    assert!(sm.process_event(Leave));
    assert!(sm.is(OuterState::Idle));
    assert_eq!(sm.context().log, ["exit A", "exit Inner"]);
}

#[test]
fn reentry_without_history_resets_sub_machine() {
    let mut sm = outer();
    sm.process_event(Next);
    sm.process_event(Next);
    sm.process_event(Leave);
    sm.process_event(Next);
    assert_eq!(sm.sub::<Inner>().state(), InnerState::A);
}

#[test]
fn reentry_with_history_resumes_last_state() {
    let mut sm = outer();
    sm.process_event(Back);
    sm.process_event(Next);
    assert_eq!(sm.sub::<Remembering>().state(), RememberingState::B);
    sm.process_event(Leave);
    sm.process_event(Back);
    assert!(sm.is(OuterState::Remembering));
    assert_eq!(sm.sub::<Remembering>().state(), RememberingState::B);
}

#[test]
fn parent_anonymous_transition_fires_after_sub_machine_terminates() {
    let mut sm = outer();
    sm.process_event(Next);
    sm.process_event(Next);
    assert!(sm.process_event(Finish));
    assert!(sm.sub::<Inner>().is_terminated());
    assert!(sm.is(OuterState::Done));
    assert_eq!(
        sm.context().log[sm.context().log.len() - 2..],
        ["finished", "exit Inner"]
    );
}

sml! {
    machine Plain<Ctx>;
    *Idle + Next = state<Inner>,
     Inner + Leave = Idle,
}

#[test]
fn composite_without_parent_entry_exit_actions_still_enters_and_exits_sub() {
    let mut sm = Sm::<Plain>::new(Ctx::default());
    sm.process_event(Next);
    sm.process_event(Next);
    assert_eq!(sm.sub::<Inner>().state(), InnerState::B);
    sm.process_event(Leave);
    sm.process_event(Next);
    assert_eq!(sm.sub::<Inner>().state(), InnerState::A);
    assert_eq!(
        sm.context().log,
        ["enter A", "exit A", "enter B", "exit B", "enter A"]
    );
}
