//! STT Backend trait and types
//!
//! Defines the interface for speech-to-text engines.
//! Follows ISP: Focused interface that clients depend on.
//!
//! # Design
//!
//! This module provides:
//! - `SttBackend` trait - Core STT operations
//! - Domain types - `BackendName`, `LanguageCode`, `SttInfo`
//! - `SttSystem` enum - Factory for backend selection

use crate::error::Result;

#[cfg(feature = "whisper")]
use crate::stt::whisper::ModelSize;

// =============================================================================
// Domain Types
// =============================================================================

/// Backend name - wrapped primitive for type safety
///
/// # Example
/// ```
/// let name = BackendName::Whisper;
/// assert_eq!(name.as_str(), "whisper");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum BackendName {
    /// Whisper backend (OpenAI)
    #[default]
    Whisper,
    /// Qwen3-ASR backend (Alibaba)
    Qwen3Asr,
    /// Custom backend name
    Custom(String),
}

impl BackendName {
    /// Get the backend name as a string slice.
    pub fn as_str(&self) -> &str {
        match self {
            BackendName::Whisper => "whisper",
            BackendName::Qwen3Asr => "qwen3-asr",
            BackendName::Custom(s) => s.as_str(),
        }
    }
}

impl std::fmt::Display for BackendName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Language code - wrapped primitive for type safety
///
/// Uses ISO 639-1 two-letter codes (e.g., "en", "zh", "es")
///
/// # Example
/// ```
/// let lang = LanguageCode::from("en");
/// assert_eq!(lang.as_str(), "en");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageCode(String);

impl LanguageCode {
    /// Create a new language code.
    ///
    /// Returns None if the code is invalid (not 2 letters).
    pub fn new(code: &str) -> Option<Self> {
        if code.len() == 2 && code.chars().all(|c| c.is_ascii_alphabetic()) {
            Some(LanguageCode(code.to_lowercase()))
        } else {
            None
        }
    }

    /// Get the language code as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Get common language codes for STT backends.
    pub fn common_codes() -> Vec<Self> {
        vec![
            Self::new("en").unwrap(),
            Self::new("zh").unwrap(),
            Self::new("es").unwrap(),
            Self::new("fr").unwrap(),
            Self::new("de").unwrap(),
            Self::new("ja").unwrap(),
            Self::new("ko").unwrap(),
            Self::new("ru").unwrap(),
            Self::new("pt").unwrap(),
            Self::new("it").unwrap(),
        ]
    }
}

impl std::fmt::Display for LanguageCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for LanguageCode {
    fn from(s: &str) -> Self {
        Self::new(s).unwrap_or_else(|| Self::new("en").unwrap())
    }
}

impl Default for LanguageCode {
    fn default() -> Self {
        Self::new("en").unwrap()
    }
}

/// STT backend information metadata
///
/// Provides metadata about an STT backend for introspection
/// and display purposes.
#[derive(Debug, Clone)]
pub struct SttInfo {
    /// Backend name
    pub name: BackendName,
    /// Supported language codes
    pub languages: Vec<LanguageCode>,
    /// Whether this backend supports streaming transcription
    pub supports_streaming: bool,
    /// Model size (if applicable)
    pub model_size: Option<String>,
}

impl SttInfo {
    /// Create new STT info.
    pub fn new(name: BackendName, languages: Vec<LanguageCode>, supports_streaming: bool) -> Self {
        Self {
            name,
            languages,
            supports_streaming,
            model_size: None,
        }
    }

    /// Create with model size.
    pub fn with_model(mut self, size: impl Into<String>) -> Self {
        self.model_size = Some(size.into());
        self
    }
}

/// STT configuration
///
/// Configuration options for speech-to-text processing.
#[derive(Debug, Clone, Default)]
pub struct SttConfig {
    /// Language code (e.g., "en", "zh")
    pub language: Option<String>,
    /// Model size to use (e.g., "tiny", "base", "small")
    pub model_size: Option<String>,
    /// Temperature for generation (0.0 - 1.0)
    pub temperature: Option<f32>,
}

/// Transcription result
///
/// Contains the transcribed text and metadata.
#[derive(Debug, Clone)]
pub struct Transcription {
    /// The transcribed text
    pub text: String,
    /// Language detected (if auto-detected)
    pub language: Option<LanguageCode>,
    /// Whether this is a partial (incomplete) result
    pub is_partial: bool,
}

impl Transcription {
    /// Create a new transcription result.
    pub fn new(text: String) -> Self {
        Self {
            text,
            language: None,
            is_partial: false,
        }
    }

    /// Create with language detection.
    pub fn with_language(mut self, lang: LanguageCode) -> Self {
        self.language = Some(lang);
        self
    }

    /// Mark as partial result.
    pub fn partial(mut self) -> Self {
        self.is_partial = true;
        self
    }
}

impl From<String> for Transcription {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

// =============================================================================
// SttBackend Trait
// =============================================================================

/// Trait for speech-to-text backends.
///
/// Implement this trait to add new STT backends.
/// Follows ISP: Provides essential methods without bloating the interface.
///
/// # Required Methods
///
/// - `transcribe()` - Convert audio to text
/// - `is_ready()` - Check if backend is initialized
///
/// # Optional Methods (with default implementations)
///
/// - `info()` - Get backend metadata
/// - `supports_streaming()` - Query streaming capability
/// - `languages()` - List supported languages
///
/// # Example
///
/// ```rust,ignore
/// use thaleia_core::stt::{SttBackend, SttInfo, BackendName, LanguageCode, Transcription};
///
/// struct MySttEngine {
///     model: MyModel,
/// }
///
/// impl SttBackend for MySttEngine {
///     fn transcribe(&mut self, audio: &[f32], sample_rate: u32) -> Result<Transcription> {
///         let text = self.model.predict(audio, sample_rate)?;
///         Ok(Transcription::new(text))
///     }
///
///     fn is_ready(&self) -> bool {
///         self.model.is_loaded()
///     }
///
///     fn info(&self) -> SttInfo {
///         SttInfo::new(
///             BackendName::Custom("my-stt".into()),
///             LanguageCode::common_codes(),
///             false,
///         )
///     }
/// }
/// ```
pub trait SttBackend {
    /// Transcribe audio samples to text.
    ///
    /// # Parameters
    /// * `audio` - Audio samples as f32 (mono)
    /// * `sample_rate` - Sample rate of the audio (e.g., 24000, 16000)
    ///
    /// # Returns
    /// * `Ok(Transcription)` - The transcribed text with metadata
    /// * `Err(Error)` - If transcription fails
    fn transcribe(&mut self, audio: &[f32], sample_rate: u32) -> Result<Transcription>;

    /// Check if the backend is ready for transcription.
    fn is_ready(&self) -> bool;

    /// Get backend information metadata.
    ///
    /// Default implementation returns basic info.
    fn info(&self) -> SttInfo {
        SttInfo::new(BackendName::default(), LanguageCode::common_codes(), false)
    }

    /// Check if this backend supports streaming transcription.
    ///
    /// Default returns false.
    fn supports_streaming(&self) -> bool {
        false
    }

    /// Get list of supported languages.
    ///
    /// Default implementation returns common codes.
    fn languages(&self) -> Vec<LanguageCode> {
        LanguageCode::common_codes()
    }
}

// =============================================================================
// SttSystem Factory
// =============================================================================

/// STT backend type enumeration
///
/// Used to select which STT backend to use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SttBackendType {
    /// Whisper backend (OpenAI)
    #[default]
    Whisper,
    // Qwen3-ASR backend (Alibaba) - coming soon
    // #[cfg(feature = "qwen-asr")]
    // Qwen3Asr,
}

impl SttBackendType {
    /// Get the backend type name as string.
    pub fn name(&self) -> &'static str {
        match self {
            // #[cfg(feature = "qwen-asr")]
            // SttBackendType::Qwen3Asr => "qwen3-asr",
            SttBackendType::Whisper => "whisper",
        }
    }
}

/// Unified STT system with backend selection
///
/// # Auto-Detection
///
/// `SttSystem::new()` creates a default backend (Whisper).
///
/// # Manual Selection
///
/// Use `SttSystem::with_backend()` to select a specific backend.
#[derive(Debug)]
#[non_exhaustive]
pub enum SttSystem {
    /// Whisper backend
    #[cfg(feature = "whisper")]
    Whisper(WhisperEngine),
    // /// Qwen3-ASR backend - coming soon
    // #[cfg(feature = "qwen-asr")]
    // Qwen3Asr(qwen_asr::Qwen3AsrBackend),
}

impl SttSystem {
    /// Create a new STT system with default backend (Whisper).
    #[cfg(feature = "whisper")]
    pub async fn new() -> crate::error::Result<Self> {
        Self::with_backend(SttBackendType::Whisper).await
    }

    /// Create STT system with a specific backend.
    pub async fn with_backend(backend_type: SttBackendType) -> crate::error::Result<Self> {
        match backend_type {
            #[cfg(feature = "whisper")]
            SttBackendType::Whisper => {
                let backend = WhisperEngine::new(ModelSize::default()).await?;
                Ok(SttSystem::Whisper(backend))
            }
            #[cfg(not(feature = "whisper"))]
            SttBackendType::Whisper => Err(crate::error::Error::Init(
                "Whisper feature not enabled".into(),
            )), // #[cfg(feature = "qwen-asr")]
                // SttBackendType::Qwen3Asr => {
                //     let backend = qwen_asr::Qwen3AsrBackend::new().await?;
                //     Ok(SttSystem::Qwen3Asr(backend))
                // }
                // #[cfg(not(feature = "qwen-asr"))]
                // SttBackendType::Qwen3Asr => {
                //     Err(crate::error::Error::Init("Qwen3-ASR feature not enabled".into()))
                // }
        }
    }

    /// Get the backend type name.
    pub fn backend_name(&self) -> &'static str {
        match self {
            #[cfg(feature = "whisper")]
            SttSystem::Whisper(_) => "whisper",
            // #[cfg(feature = "qwen-asr")]
            // SttSystem::Qwen3Asr(_) => "qwen3-asr",
        }
    }

    /// Check if the backend is ready.
    pub fn is_ready(&self) -> bool {
        match self {
            #[cfg(feature = "whisper")]
            SttSystem::Whisper(b) => b.is_ready(),
            // #[cfg(feature = "qwen-asr")]
            // SttSystem::Qwen3Asr(b) => b.is_ready(),
        }
    }

    /// Transcribe audio to text.
    pub fn transcribe(
        &mut self,
        audio: &[f32],
        sample_rate: u32,
    ) -> crate::error::Result<Transcription> {
        match self {
            #[cfg(feature = "whisper")]
            SttSystem::Whisper(b) => b.transcribe(audio, sample_rate),
            // #[cfg(feature = "qwen-asr")]
            // SttSystem::Qwen3Asr(b) => b.transcribe(audio, sample_rate),
        }
    }
}

// Re-export for convenience
#[cfg(feature = "whisper")]
use crate::stt::whisper::WhisperEngine;

// Placeholder for future Qwen3-ASR implementation
// #[cfg(feature = "qwen-asr")]
// mod qwen_asr;
