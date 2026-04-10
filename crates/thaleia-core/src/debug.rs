//! Debug utilities for Thaleia
//!
//! Provides unified debug/trace system following Rust 2026 best practices:
//! - `tracing` for production logging (info, warn, error)
//! - `debug!` macro for development-only verbose output
//! - Controlled via RUST_LOG or THALEIA_DEBUG env vars

use std::sync::atomic::{AtomicBool, Ordering};

/// Global debug flag - set via environment or programmatically
static DEBUG_ENABLED: AtomicBool = AtomicBool::new(false);

/// Initialize debug mode from environment
///
/// Called automatically when the crate is loaded.
/// Checks: THALEIA_DEBUG=1 or RUST_LOG is set to a debug level.
pub fn init_from_env() {
    let from_env = std::env::var("THALEIA_DEBUG").is_ok();
    let from_rust_log = std::env::var("RUST_LOG")
        .map(|v| {
            let v_lower = v.to_lowercase();
            v_lower.contains("debug") || v_lower.contains("trace")
        })
        .unwrap_or(false);

    DEBUG_ENABLED.store(from_env || from_rust_log, Ordering::SeqCst);
}

/// Check if debug mode is enabled
///
/// Debug mode enables verbose `debug!` macro output.
/// Controlled by:
/// - `THALEIA_DEBUG=1` environment variable
/// - `RUST_LOG=debug` or `RUST_LOG=trace`
/// - Programmatic call to `set_debug(true)`
pub fn is_debug() -> bool {
    DEBUG_ENABLED.load(Ordering::SeqCst)
}

/// Set debug mode programmatically
///
/// # Safety
/// This is safe in single-threaded contexts or before any threads spawn.
/// In multi-threaded contexts, prefer using the environment variables.
pub fn set_debug(enabled: bool) {
    DEBUG_ENABLED.store(enabled, Ordering::SeqCst);
}

/// Initialize tracing subscriber for the application
///
/// Should be called once at application startup in main.rs.
/// Uses RUST_LOG environment variable for log level filtering.
/// Falls back to "info" if not set.
pub fn init_logging() {
    use tracing_subscriber::{EnvFilter, fmt, prelude::*};

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(fmt::layer().with_target(true).with_thread_ids(true))
        .with(filter)
        .init();
}

/// Initialize logging with custom default level
///
/// Use this when you want a different default level than "info".
pub fn init_logging_with_default(default_level: &str) {
    use tracing_subscriber::{EnvFilter, fmt, prelude::*};

    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(default_level));

    tracing_subscriber::registry()
        .with(fmt::layer().with_target(true).with_thread_ids(true))
        .with(filter)
        .init();
}

#[macro_export]
/// Debug print macro - only outputs if debug mode is enabled
///
/// Sends output to stderr, suitable for verbose development debugging.
/// Use `tracing::info!`, `tracing::warn!`, `tracing::error!` for production.
///
/// # Examples
/// ```ignore
/// thaleia_debug!("Captured {} samples", count);
/// thaleia_debug!("Audio backend: {}", backend_name);
/// ```
macro_rules! thaleia_debug {
    ($($arg:tt)*) => {
        if $crate::debug::is_debug() {
            eprintln!($($arg)*)
        }
    };
}

// Initialize on module load
#[doc(hidden)]
pub mod init {
    use super::*;

    #[ctor::ctor]
    fn init() {
        init_from_env();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_debug_flag_default() {
        // By default, should not be debug (unless env var set)
        // This test just verifies the function works
        let _ = is_debug();
    }

    #[test]
    fn test_set_debug() {
        let original = is_debug();
        set_debug(true);
        assert!(is_debug());
        set_debug(original);
    }
}
