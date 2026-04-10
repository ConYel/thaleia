//! Thaleia Core - Voice AI Engine
//!
//! The joyful voice AI companion built with Rust.
//!
//! Thaleia provides voice input (STT) and output (TTS) via MCP server.
//! Any MCP-compatible LLM can use Thaleia as ears and mouth.

pub mod audio;
pub mod debug;
mod error;

#[cfg(feature = "playback-capture")]
pub mod capture;

pub mod tts;

#[cfg(feature = "kokoro")]
pub mod download;

#[cfg(feature = "whisper")]
pub mod stt;

/// VAD (Voice Activity Detection) module
///
/// Provides voice activity detection for natural conversations.
/// Uses energy-based detection as placeholder until ONNX-based VAD is implemented.
pub mod vad;

/// Pipeline module for voice AI dialogue management
///
/// Provides the DialogueManager for orchestrating voice interactions,
/// including state machine management and interruption handling.
pub mod pipeline;

/// Wake Word Detection module
///
/// Provides wake word detection for triggering voice interactions.
/// Uses energy-based placeholder until ONNX-based wake word is implemented.
pub mod wake_word;

pub use audio::{AudioBackend, AudioEngine, AudioSystem, BackendType, diagnostics};
pub use debug::{init_logging, init_logging_with_default, is_debug, set_debug};
pub use error::{Error, Result};

#[cfg(feature = "playback-capture")]
pub use capture::{AudioCapture, CaptureConfig, CapturedAudio};

pub use tts::TtsEngine;

#[cfg(feature = "whisper")]
pub use stt::{
    BackendName, LanguageCode, ModelSize, SttBackend, SttBackendType, SttConfig, SttInfo,
    SttSystem, Transcription, WhisperEngine,
};

/// VAD exports - always available
pub use vad::{VadBackend, VadConfig, VadResult, VadState, VadSystem};

/// Pipeline exports - dialogue management
pub use pipeline::{DialogueEvent, DialogueManager, DialogueState};

/// Wake Word exports
pub use wake_word::{WakeWordBackend, WakeWordConfig, WakeWordResult, WakeWordSystem};

/// Voice Pipeline integration
///
/// Integrates WakeWord, VAD, STT, and TTS into a unified pipeline.
pub mod integration;

pub use integration::{PipelineConfig, PipelineEvent, VoicePipeline};

/// Thaleia version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_exists() {
        assert!(!VERSION.is_empty());
    }

    #[test]
    fn test_error_trait_impls() {
        fn assert_error<T: std::error::Error>() {}
        assert_error::<Error>();
    }
}
