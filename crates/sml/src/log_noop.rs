//! No-op logging hooks (`tracing` feature disabled).

use std::fmt::Debug;

/// No-op.
#[inline]
pub fn log_process_event<M, E>(_handled: bool) {}

/// Returns `result`.
#[inline]
#[must_use]
pub fn log_guard(_guard: &'static str, result: bool) -> bool {
    result
}

/// No-op.
#[inline]
pub fn log_action(_action: &'static str) {}

/// No-op.
#[inline]
pub fn log_state_change<M>(_from: &dyn Debug, _to: &dyn Debug) {}
