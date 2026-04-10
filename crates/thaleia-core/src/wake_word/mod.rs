//! Wake Word Detection module
//!
//! Provides wake word detection with pluggable backends.
//! Uses Silero ONNX model for detection (reuses VAD's ONNX runtime).
//!
//! # Architecture
//!
//! Follows the same pattern as VAD and STT modules:
//! - [`WakeWordBackend`] trait - Interface for wake word implementations
//! - [`WakeWordConfig`] - Configuration options
//! - [`WakeWordResult`] - Detection result with metadata
//! - [`WakeWordSystem`] - Factory for backend selection
//!
//! # Usage
//!
//! ```rust,ignore
//! use thaleia_core::wake_word::{WakeWordSystem, WakeWordConfig};
//!
//! // Create wake word system with defaults
//! let mut ww = WakeWordSystem::new()?;
//!
//! // Detect wake word in audio
//! let result = ww.detect(&audio_samples, 16000)?;
//! if result.detected {
//!     println!("Wake word '{}' detected!", result.keyword);
//! }
//! ```

pub mod config;
pub mod engine;

pub use config::{WakeWordConfig, WakeWordResult};
pub use engine::{WakeWordBackend, WakeWordSystem};
