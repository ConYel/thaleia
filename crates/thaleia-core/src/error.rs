//! Error types for Thaleia
//!
//! Following SRP: This module has ONE reason to change - error definitions.

use thiserror::Error;

/// Thaleia result type
pub type Result<T> = std::result::Result<T, Error>;

/// Thaleia error types
#[derive(Debug, Error)]
pub enum Error {
    /// Audio device not found or unavailable
    #[error("Audio device error: {0}")]
    AudioDevice(String),

    /// TTS synthesis failed
    #[error("TTS synthesis error: {0}")]
    TtsSynthesis(String),

    /// STT transcription failed
    #[error("STT transcription error: {0}")]
    SttTranscription(String),

    /// Voice not found
    #[error("Voice not found: {0}")]
    VoiceNotFound(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),

    /// Initialization failed
    #[error("Initialization failed: {0}")]
    Init(String),

    /// Playback error
    #[error("Playback error: {0}")]
    Playback(String),

    /// VAD error
    #[error("VAD error: {0}")]
    VadError(String),

    /// Network error
    #[error("Network error: {0}")]
    Network(String),

    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::sync::Arc<std::io::Error>),

    /// Generic internal error
    #[error("Internal error: {0}")]
    Internal(String),
}

impl Error {
    /// Check if error is recoverable
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            Error::AudioDevice(_) | Error::VoiceNotFound(_) | Error::Config(_)
        )
    }

    /// User-friendly message
    pub fn user_message(&self) -> &'static str {
        match self {
            Error::AudioDevice(_) => "I had trouble with audio. Is my microphone working?",
            Error::TtsSynthesis(_) => "I had trouble finding the right words. Let me try again!",
            Error::SttTranscription(_) => "I didn't quite catch that. Could you repeat?",
            Error::VoiceNotFound(_) => {
                "I don't know that voice. Let me show you the ones I do know!"
            }
            Error::Config(_) => "Something's not set up right. Let me check...",
            Error::Init(_) => "I'm having trouble starting up. Give me a moment!",
            Error::Playback(_) => "I can't play audio right now. Let me try again!",
            Error::VadError(_) => "I'm having trouble detecting speech. Let me try again!",
            Error::Network(_) => "I can't connect to the network. Let me try again!",
            Error::Io(_) => "I had trouble reading or writing a file. Let me try again!",
            Error::Internal(_) => "Oops! Something went wrong, but I'm still here!",
        }
    }
}

// From implementations for cpal errors (only available with cpal feature)
#[cfg(feature = "cpal")]
impl From<cpal::BuildStreamError> for Error {
    fn from(e: cpal::BuildStreamError) -> Self {
        Error::AudioDevice(format!("Failed to build audio stream: {}", e))
    }
}

#[cfg(feature = "cpal")]
impl From<cpal::PlayStreamError> for Error {
    fn from(e: cpal::PlayStreamError) -> Self {
        Error::AudioDevice(format!("Failed to play audio stream: {}", e))
    }
}

#[cfg(feature = "cpal")]
impl From<cpal::DevicesError> for Error {
    fn from(e: cpal::DevicesError) -> Self {
        Error::AudioDevice(format!("Failed to enumerate audio devices: {}", e))
    }
}
