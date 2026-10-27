//! Logging hooks called by `Sm` and `sml!` expansions (`tracing` feature).

use std::any::type_name;
use std::fmt::Debug;

/// Logs a dispatched top-level event.
pub fn log_process_event<M, E>(handled: bool) {
    tracing::debug!(
        target: "sml",
        machine = type_name::<M>(),
        event = type_name::<E>(),
        handled,
        "process_event"
    );
}

/// Logs a guard evaluation, returning its `result`.
pub fn log_guard(guard: &'static str, result: bool) -> bool {
    tracing::trace!(target: "sml", guard, result, "guard");
    result
}

/// Logs an action about to run.
pub fn log_action(action: &'static str) {
    tracing::trace!(target: "sml", action, "action");
}

/// Logs a state change of machine `M`.
pub fn log_state_change<M>(from: &dyn Debug, to: &dyn Debug) {
    tracing::debug!(
        target: "sml",
        machine = type_name::<M>(),
        from = ?from,
        to = ?to,
        "state_change"
    );
}
