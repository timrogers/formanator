//! Process-wide verbose-logging toggle shared by the Forma HTTP client and the
//! LLM client. Both use the same flag because the CLI exposes a single
//! `--verbose` switch per subcommand.

use std::sync::atomic::{AtomicBool, Ordering};

static VERBOSE: AtomicBool = AtomicBool::new(false);

/// Enable or disable verbose request/response logging to stderr.
pub fn set(enabled: bool) {
    VERBOSE.store(enabled, Ordering::Relaxed);
}

/// Whether verbose logging is currently enabled.
pub fn is_enabled() -> bool {
    VERBOSE.load(Ordering::Relaxed)
}
