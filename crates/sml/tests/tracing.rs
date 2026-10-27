//! `tracing` feature: events, guards, actions and state changes are logged.
#![cfg(feature = "tracing")]

use std::io::Write;
use std::sync::{Arc, Mutex, PoisonError};

use sml::{Sm, sml};
use tracing_subscriber::util::SubscriberInitExt;

#[derive(Clone, Default)]
struct Capture(Arc<Mutex<Vec<u8>>>);

impl Write for Capture {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut out = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        out.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl Capture {
    fn text(&self) -> String {
        let out = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        String::from_utf8_lossy(&out).into_owned()
    }
}

struct Ctx;
struct Go;
struct Nope;

fn allowed(_: &Ctx) -> bool {
    true
}
fn mark(_: &mut Ctx) {}

sml! {
    machine Logged<Ctx>;
    *Idle + Go [allowed] / mark = Busy,
}

#[test]
fn logs_event_guard_action_and_state_change() {
    let capture = Capture::default();
    let writer = capture.clone();
    let _guard = tracing_subscriber::fmt()
        .with_max_level(tracing_subscriber::filter::LevelFilter::TRACE)
        .with_ansi(false)
        .with_writer(move || writer.clone())
        .finish()
        .set_default();

    let mut sm = Sm::<Logged>::new(Ctx);
    assert!(sm.process_event(Go));
    assert!(!sm.process_event(Nope));

    let text = capture.text();
    assert!(
        text.contains("guard guard=\"allowed\" result=true"),
        "{text}"
    );
    assert!(text.contains("action action=\"mark\""), "{text}");
    assert!(
        text.contains("state_change") && text.contains("from=Idle to=Busy"),
        "{text}"
    );
    assert!(
        text.contains("event=\"tracing::Go\" handled=true"),
        "{text}"
    );
    assert!(
        text.contains("event=\"tracing::Nope\" handled=false"),
        "{text}"
    );
}
