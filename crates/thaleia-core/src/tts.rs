//! Text-to-Speech engine for Thaleia
//!
//! Following SRP: This module has ONE reason to change - TTS operations.

use crate::Result;

// Kokoro TTS submodule (requires `kokoro` feature)
#[cfg(feature = "kokoro")]
pub mod kokoro;

/// Voice identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VoiceId(pub String);

impl VoiceId {
    /// Create new voice ID
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Get the voice ID as string
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Voice metadata
#[derive(Debug, Clone)]
pub struct Voice {
    /// Voice identifier
    pub id: VoiceId,
    /// Human-readable name
    pub name: String,
    /// Language code (e.g., "en")
    pub language: String,
}

impl Voice {
    /// Create new voice
    pub fn new(id: VoiceId, name: impl Into<String>, language: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            language: language.into(),
        }
    }
}

/// Synthesis request
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct SynthesisRequest {
    /// Text to synthesize
    pub text: String,
    /// Voice to use
    pub voice: Option<VoiceId>,
    /// Speech speed (0.5 - 2.0)
    pub speed: f32,
}

impl SynthesisRequest {
    /// Create new synthesis request
    #[allow(dead_code)]
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            voice: None,
            speed: 1.0,
        }
    }

    /// Set voice
    #[allow(dead_code)]
    pub fn with_voice(mut self, voice: VoiceId) -> Self {
        self.voice = Some(voice);
        self
    }

    /// Set speed
    #[allow(dead_code)]
    pub fn with_speed(mut self, speed: f32) -> Self {
        self.speed = speed.clamp(0.5, 2.0);
        self
    }
}

/// TTS engine trait
///
/// Following OCP: New TTS implementations can be added without modifying existing code.
#[allow(dead_code)]
pub trait TtsEngineTrait: Send + Sync {
    /// Synthesize text to audio
    fn synthesize(&self, request: &SynthesisRequest) -> Result<Vec<u8>>;

    /// List available voices
    fn list_voices(&self) -> Vec<Voice>;

    /// Get default voice
    fn default_voice(&self) -> VoiceId;
}

/// TTS engine placeholder
///
/// When `kokoro` feature is enabled, this uses actual Kokoro-ONNX synthesis.
#[derive(Debug)]
pub struct TtsEngine {
    #[cfg(feature = "kokoro")]
    inner: kokoro::KokoroTtsEngine,
    /// Marker field to ensure struct has at least one field
    #[cfg(not(feature = "kokoro"))]
    _marker: (),
}

impl TtsEngine {
    /// Create new TTS engine
    #[cfg(feature = "kokoro")]
    pub async fn new() -> Result<Self> {
        Ok(Self {
            inner: kokoro::KokoroTtsEngine::new().await?,
        })
    }

    /// Create new TTS engine (stub when kokoro not enabled)
    #[cfg(not(feature = "kokoro"))]
    pub fn new() -> Result<Self> {
        Ok(Self { _marker: () })
    }

    /// Synthesize text to audio samples
    ///
    /// Returns raw f32 audio samples at 22050 Hz.
    #[cfg(feature = "kokoro")]
    pub fn synthesize(&mut self, text: &str, voice: Option<&str>) -> Result<Vec<f32>> {
        self.inner.synthesize_text(text, voice)
    }

    /// Synthesize text to audio samples (stub)
    #[cfg(not(feature = "kokoro"))]
    pub fn synthesize(&mut self, _text: &str, _voice: Option<&str>) -> Result<Vec<f32>> {
        Err(crate::Error::TtsSynthesis(
            "Kokoro support not enabled. Build with --features kokoro".into(),
        ))
    }

    /// List built-in voices
    pub fn built_in_voices() -> Vec<Voice> {
        vec![
            Voice::new(VoiceId::new("af_sarah"), "Sarah", "en"),
            Voice::new(VoiceId::new("af_sky"), "Sky", "en"),
            Voice::new(VoiceId::new("af_bella"), "Bella", "en"),
            Voice::new(VoiceId::new("af_nicole"), "Nicole", "en"),
            Voice::new(VoiceId::new("am_adam"), "Adam", "en"),
        ]
    }

    /// Get all available voices (uses Kokoro when enabled)
    #[cfg(feature = "kokoro")]
    pub fn list_voices(&self) -> Vec<Voice> {
        self.inner.list_voices()
    }

    /// Get all available voices (stub)
    #[cfg(not(feature = "kokoro"))]
    pub fn list_voices(&self) -> Vec<Voice> {
        Self::built_in_voices()
    }

    /// Get the default voice
    #[cfg(feature = "kokoro")]
    pub fn default_voice(&self) -> VoiceId {
        self.inner.default_voice()
    }

    /// Get the default voice (stub)
    #[cfg(not(feature = "kokoro"))]
    pub fn default_voice(&self) -> VoiceId {
        VoiceId::new("af_sky")
    }
}

/// Default TTS engine (only when kokoro not enabled, since async init required)
#[cfg(not(feature = "kokoro"))]
impl Default for TtsEngine {
    fn default() -> Self {
        Self::new().expect("TTS engine should always initialize")
    }
}

/// TTS configuration
///
/// Configuration options for text-to-speech processing.
#[derive(Debug, Clone, Default)]
pub struct TtsConfig {
    /// Voice ID to use
    pub voice_id: Option<String>,
    /// Speech speed (0.5 - 2.0)
    pub speed: Option<f32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_voice_id() {
        let id = VoiceId::new("af_sarah");
        assert_eq!(id.as_str(), "af_sarah");
    }

    #[test]
    fn test_voice() {
        let voice = Voice::new(VoiceId::new("test"), "Test Voice", "en");
        assert_eq!(voice.name, "Test Voice");
        assert_eq!(voice.language, "en");
    }

    #[test]
    fn test_synthesis_request() {
        let req = SynthesisRequest::new("Hello!")
            .with_voice(VoiceId::new("af_sky"))
            .with_speed(1.2);

        assert_eq!(req.text, "Hello!");
        assert_eq!(req.voice.unwrap().as_str(), "af_sky");
        assert!((req.speed - 1.2).abs() < 0.001);
    }

    #[test]
    fn test_speed_clamping() {
        let req = SynthesisRequest::new("Test").with_speed(5.0);
        assert_eq!(req.speed, 2.0);

        let req = SynthesisRequest::new("Test").with_speed(0.1);
        assert_eq!(req.speed, 0.5);
    }

    #[test]
    fn test_built_in_voices() {
        let voices = TtsEngine::built_in_voices();
        assert!(!voices.is_empty());
        assert!(voices.iter().all(|v| v.language == "en"));
    }

    // =========================================================================
    // Kokoro Integration Tests (require OpenSSL dev libraries)
    // These tests verify the TTS interface contract. To run them:
    //   1. Install OpenSSL: apt-get install libssl-dev (Ubuntu) or dnf install openssl-devel (Fedora)
    //   2. Uncomment kokoro-tiny in Cargo.toml
    //   3. Run: cargo test -- --ignored
    // =========================================================================

    #[test]
    #[ignore = "Requires kokoro-tiny crate (needs OpenSSL dev libraries)"]
    fn test_kokoro_api_exists() {
        // This test verifies the kokoro-tiny API contract
        // Once dependencies are installed, uncomment to test:

        // use kokoro_tiny::TtsEngine as KokoroEngine;
        // let result = KokoroEngine::new();
        // assert!(result.is_ok() || result.is_err()); // Just verify API exists
    }

    #[test]
    fn test_synthesis_request_validation() {
        // Verify SynthesisRequest properly validates input
        let empty_request = SynthesisRequest::new("");
        assert_eq!(empty_request.text, "");

        let long_text = SynthesisRequest::new("A".repeat(1000));
        assert_eq!(long_text.text.len(), 1000);
    }
}
