//! Null backend for audio - file-only mode
//!
//! This backend does nothing for audio playback and capture.
//! It's used when no audio backend is available.
//!
//! ## Use Case
//!
//! - Systems without audio hardware
//! - Systems where audio is not configured
//! - Debugging / testing without audio
//!
//! ## Capabilities
//!
//! | Feature | Status |
//! |---------|--------|
//! | Playback | ❌ (logs warning) |
//! | Capture | ❌ (returns error) |
//! | File output | ✅ (use `speak-output` instead) |

use crate::audio::backends::{AudioBackend, AudioError};
use std::time::Duration;

/// Null backend - no audio operations
///
/// This backend logs all operations but does nothing.
/// Use file-based output (`speak-output`) for actual audio.
#[derive(Debug)]
pub struct NullBackend {
    /// Human-readable reason why no audio backend is available
    reason: String,
}

impl NullBackend {
    /// Create a new NullBackend
    ///
    /// # Arguments
    /// * `reason` - Why no other backend was available
    pub fn new(reason: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
        }
    }

    /// Create with default reason
    pub fn with_default_reason() -> Self {
        Self::new("No audio backend available on this system")
    }
}

impl Default for NullBackend {
    fn default() -> Self {
        Self::with_default_reason()
    }
}

impl AudioBackend for NullBackend {
    fn backend_name(&self) -> &'static str {
        "null"
    }

    fn is_available(&self) -> bool {
        false
    }

    fn play(&mut self, samples: &[f32], sample_rate: u32) -> Result<(), AudioError> {
        tracing::warn!(
            "NullBackend: play() called - {} ({}/{} Hz, {} samples)",
            self.reason,
            samples.len(),
            sample_rate,
            samples.len()
        );
        // Don't error - just log and return Ok
        // This allows Thaleia to continue without audio
        Ok(())
    }

    fn capture(&mut self, _duration: Duration) -> Result<Vec<f32>, AudioError> {
        tracing::warn!("NullBackend: capture() called - {}", self.reason);
        Err(AudioError::NoBackend)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_null_backend() {
        let backend = NullBackend::new("test");
        assert_eq!(backend.backend_name(), "null");
        assert!(!backend.is_available());
    }

    #[test]
    fn test_play_returns_ok() {
        let mut backend = NullBackend::default();
        let result = backend.play(&[0.0f32; 100], 44100);
        assert!(result.is_ok());
    }

    #[test]
    fn test_capture_returns_error() {
        let mut backend = NullBackend::default();
        let result = backend.capture(Duration::from_secs(1));
        assert!(result.is_err());
    }
}
