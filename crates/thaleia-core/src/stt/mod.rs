//! Speech-to-Text (STT) module
//!
//! Provides speech recognition with pluggable backends.
//!
//! # Architecture
//!
//! Follows SOLID principles with the [`SttBackend`] trait allowing
//! different STT implementations to be swapped easily.
//!
//! # Usage
//!
//! ```rust,ignore
//! use thaleia_core::stt::{SttSystem, SttBackendType};
//!
//! // Auto-detect backend (default: Whisper)
//! let mut stt = SttSystem::new().await?;
//! let result = stt.transcribe(&audio_samples, 24000)?;
//! println!("You said: {}", result.text);
//!
//! // Or use specific backend
//! let mut stt = SttSystem::with_backend(SttBackendType::Whisper).await?;
//! ```

pub mod engine;
pub mod whisper;

pub use engine::{
    BackendName, LanguageCode, SttBackend, SttBackendType, SttConfig, SttInfo, SttSystem,
    Transcription,
};

#[cfg(feature = "whisper")]
pub use whisper::{ModelSize, WhisperEngine};

// Qwen3-ASR support coming soon
// #[cfg(feature = "qwen-asr")]
// pub use engine::qwen_asr::Qwen3AsrBackend;
