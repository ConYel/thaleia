//! Audio module for Thaleia
//!
//! Provides unified audio playback and capture with automatic backend selection.
//!
//! ## Architecture
//!
//! ```text
//! AudioSystem
//! ├── RodioBackend (primary - ALSA)
//! ├── SDL2Backend  (fallback - PulseAudio)
//! └── NullBackend   (file-only mode)
//! ```
//!
//! ## Features
//!
//! - Non-blocking playback with automatic fallback
//! - Cross-platform audio capture
//! - Backend detection and selection
//! - Graceful degradation

pub mod backends;
pub mod diagnostics;
pub mod engine;

pub use backends::{AudioBackend, AudioError, AudioSystem, BackendType};
pub use engine::{AudioChunk, AudioEngine, Sample};

/// Result type for audio operations
pub type Result<T> = std::result::Result<T, AudioError>;
