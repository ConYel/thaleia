//! Voice Activity Detection (VAD) module
//!
//! Provides speech detection with pluggable backends.
//!
//! # Architecture
//!
//! Follows SOLID principles with the [`VadBackend`] trait allowing
//! different VAD implementations to be swapped easily.
//!
//! # Usage
//!
//! ```rust,ignore
//! use thaleia_core::vad::{VadSystem, VadConfig};
//!
//! // Create VAD system with defaults
//! let mut vad = VadSystem::new()?;
//!
//! // Detect speech
//! let result = vad.detect_speech(&audio_samples, 16000)?;
//! if result.is_speaking {
//!     println!("User is speaking!");
//! }
//!
//! // Or with custom config
//! let config = VadConfig {
//!     threshold: 0.7,
//!     min_speech_duration_ms: 500,
//!     ..Default::default()
//! };
//! let mut vad = VadSystem::with_config(config)?;
//! ```

pub mod engine;
pub mod silero;

#[cfg(feature = "vad")]
pub mod onnx;

pub use engine::{VadBackend, VadConfig, VadResult, VadState, VadSystem};
