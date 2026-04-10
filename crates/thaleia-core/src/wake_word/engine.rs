//! Wake Word engine and types
//!
//! Provides wake word detection with pluggable backends.
//!
//! # Architecture
//!
//! Follows SOLID principles with the [`WakeWordBackend`] trait allowing
//! different wake word implementations to be swapped easily.

use crate::Result;

use super::config::{WakeWordConfig, WakeWordResult};

// =============================================================================
// WakeWordBackend Trait
// =============================================================================

/// Trait for wake word detection backends.
///
/// Implement this trait to add new wake word detection implementations.
/// Follows ISP: Provides essential methods without bloating the interface.
///
/// # Required Methods
///
/// - `detect()` - Detect wake word in audio
/// - `is_ready()` - Check if backend is initialized
///
/// # Optional Methods (with default implementations)
///
/// - `config()` - Get current configuration
/// - `set_keywords()` - Update wake word keywords
///
/// # Example
///
/// ```rust,ignore
/// use thaleia_core::wake_word::{WakeWordBackend, WakeWordConfig, WakeWordResult};
///
/// struct MyWakeWordEngine {
///     model: MyModel,
/// }
///
/// impl WakeWordBackend for MyWakeWordEngine {
///     fn detect(&mut self, audio: &[f32], sample_rate: u32) -> Result<WakeWordResult> {
///         let (detected, confidence) = self.model.predict(audio, sample_rate)?;
///         Ok(WakeWordResult {
///             detected,
///             keyword: Some("hey_thaleia".to_string()),
///             confidence,
///         })
///     }
///
///     fn is_ready(&self) -> bool {
///         self.model.is_loaded()
///     }
/// }
/// ```
pub trait WakeWordBackend {
    /// Detect wake word in audio samples.
    ///
    /// # Parameters
    /// * `audio` - Audio samples as f32 (mono)
    /// * `sample_rate` - Sample rate of the audio (e.g., 16000)
    ///
    /// # Returns
    /// * `Ok(WakeWordResult)` - Detection result with keyword and confidence
    /// * `Err(Error)` - If detection fails
    fn detect(&mut self, audio: &[f32], sample_rate: u32) -> Result<WakeWordResult>;

    /// Check if the backend is ready for detection.
    fn is_ready(&self) -> bool;

    /// Get current wake word configuration.
    fn config(&self) -> WakeWordConfig {
        WakeWordConfig::default()
    }

    /// Set wake word keywords to detect.
    ///
    /// Default implementation does nothing.
    fn set_keywords(&mut self, _keywords: Vec<String>) {}
}

// =============================================================================
// WakeWordSystem Factory
// =============================================================================

/// Wake word backend type enumeration
///
/// Used to select which wake word backend to use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WakeWordBackendType {
    /// Silero ONNX-based wake word
    #[default]
    Silero,
    /// Placeholder/keyword spotting (simple energy-based)
    Placeholder,
}

impl WakeWordBackendType {
    /// Get the backend type name as string.
    pub fn name(&self) -> &'static str {
        match self {
            WakeWordBackendType::Silero => "silero",
            WakeWordBackendType::Placeholder => "placeholder",
        }
    }
}

/// Unified wake word system with backend selection
///
/// # Auto-Detection
///
/// `WakeWordSystem::new()` creates a default backend.
///
/// # Manual Selection
///
/// Use `WakeWordSystem::with_config()` to customize configuration.
#[derive(Debug)]
pub enum WakeWordSystem {
    /// Placeholder wake word (simple threshold-based)
    Placeholder(PlaceholderWakeWord),
    // NOTE: Silero ONNX wake word will be added when model is ready
    // Silero(OnnxWakeWord),
}

impl WakeWordSystem {
    /// Create a new wake word system with default configuration.
    pub fn new() -> crate::error::Result<Self> {
        let config = WakeWordConfig::default();
        Self::with_config(config)
    }

    /// Create wake word system with custom configuration.
    pub fn with_config(config: WakeWordConfig) -> crate::error::Result<Self> {
        // Use placeholder for now - Silero ONNX will be added later
        let backend = PlaceholderWakeWord::new(config);
        Ok(WakeWordSystem::Placeholder(backend))
    }

    /// Check if wake word is ready.
    pub fn is_ready(&self) -> bool {
        match self {
            WakeWordSystem::Placeholder(b) => b.is_ready(),
        }
    }

    /// Detect wake word in audio.
    pub fn detect(
        &mut self,
        audio: &[f32],
        sample_rate: u32,
    ) -> crate::error::Result<WakeWordResult> {
        match self {
            WakeWordSystem::Placeholder(b) => b.detect(audio, sample_rate),
        }
    }

    /// Set wake word keywords.
    pub fn set_keywords(&mut self, keywords: Vec<String>) {
        match self {
            WakeWordSystem::Placeholder(b) => b.set_keywords(keywords),
        }
    }
}

// =============================================================================
// Placeholder Wake Word Backend
// =============================================================================

/// Simple energy-based wake word placeholder
///
/// This is a placeholder implementation that can be replaced with
/// Silero ONNX when the model is ready. Currently uses simple
/// energy threshold detection.
#[derive(Debug)]
pub struct PlaceholderWakeWord {
    config: WakeWordConfig,
    /// Running average energy for adaptive threshold
    avg_energy: f32,
    /// Number of samples processed
    sample_count: u64,
}

/// Constants for energy-based detection
///
/// These values are tuned for real-time wake word detection with
/// a microphone. After resampling to 16kHz, audio values are normalized
/// to the range [-1.0, 1.0].
const ADAPTATION_ALPHA: f32 = 0.0005;
const ENERGY_MULTIPLIER: f32 = 1.5;
const MIN_THRESHOLD: f32 = 0.005;
const MIN_ENERGY: f32 = 0.02;

impl PlaceholderWakeWord {
    /// Create new placeholder wake word engine.
    #[must_use]
    pub fn new(config: WakeWordConfig) -> Self {
        Self {
            config,
            avg_energy: 0.0,
            sample_count: 0,
        }
    }

    /// Calculate RMS energy of audio chunk
    fn calculate_energy(audio: &[f32]) -> f32 {
        if audio.is_empty() {
            return 0.0;
        }
        let sum: f32 = audio.iter().map(|&s| s * s).sum();
        (sum / audio.len() as f32).sqrt()
    }
}

impl WakeWordBackend for PlaceholderWakeWord {
    fn detect(&mut self, audio: &[f32], _sample_rate: u32) -> Result<WakeWordResult> {
        // Calculate current energy
        let energy = Self::calculate_energy(audio);

        // Update running average (exponential moving average)
        self.sample_count += 1;
        self.avg_energy = ADAPTATION_ALPHA * energy + (1.0 - ADAPTATION_ALPHA) * self.avg_energy;

        // Simple detection: if energy is significantly above average
        let threshold = self.avg_energy * ENERGY_MULTIPLIER + MIN_THRESHOLD;

        if energy > threshold && energy > MIN_ENERGY {
            // Simulate keyword detection (placeholder)
            let keyword = self.config.keywords.first().cloned().unwrap_or_default();
            Ok(WakeWordResult::detected(
                keyword,
                0.5 + (energy / (energy + threshold)) * 0.5,
            ))
        } else {
            Ok(WakeWordResult::not_detected())
        }
    }

    fn is_ready(&self) -> bool {
        true // Placeholder is always ready
    }

    fn config(&self) -> WakeWordConfig {
        self.config.clone()
    }

    fn set_keywords(&mut self, keywords: Vec<String>) {
        self.config.keywords = keywords;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_placeholder_creation() {
        let ww = PlaceholderWakeWord::new(WakeWordConfig::default());
        assert!(ww.is_ready());
    }

    #[test]
    fn test_silent_audio_not_detected() {
        let mut ww = PlaceholderWakeWord::new(WakeWordConfig::default());
        let silent = vec![0.0f32; 16000];
        let result = ww.detect(&silent, 16000).unwrap();
        assert!(!result.detected);
    }

    #[test]
    fn test_loud_audio_may_detect() {
        let mut ww = PlaceholderWakeWord::new(WakeWordConfig::default());
        // Very loud audio
        let loud: Vec<f32> = (0..16000).map(|i| (i as f32 * 0.1).sin()).collect();
        let result = ww.detect(&loud, 16000).unwrap();
        // May or may not detect depending on adaptive threshold
        assert!(result.confidence >= 0.0 && result.confidence <= 1.0);
    }

    #[test]
    fn test_loud_audio_detects_after_silence_baseline() {
        let mut ww = PlaceholderWakeWord::new(WakeWordConfig::default());

        // First, process quiet audio to establish baseline
        let quiet: Vec<f32> = vec![0.001f32; 16000]; // Very quiet
        for _ in 0..10 {
            let _ = ww.detect(&quiet, 16000).unwrap();
        }

        // Now process loud audio - should detect
        let loud: Vec<f32> = (0..16000).map(|i| (i as f32 * 0.1).sin()).collect();
        let result = ww.detect(&loud, 16000).unwrap();

        // After establishing quiet baseline, loud audio should trigger detection
        assert!(
            result.detected,
            "Loud audio should be detected after quiet baseline"
        );
    }

    #[test]
    fn test_returns_configured_keyword() {
        let config = WakeWordConfig {
            keywords: vec!["hey_thaleia".to_string()],
            sample_rate: 16000,
            threshold: 0.5,
        };
        let mut ww = PlaceholderWakeWord::new(config);

        // First process quiet to establish baseline
        let quiet = vec![0.001f32; 16000];
        let _ = ww.detect(&quiet, 16000).unwrap();

        // Then loud audio
        let loud: Vec<f32> = vec![0.5f32; 16000];
        let result = ww.detect(&loud, 16000).unwrap();

        if result.detected {
            assert_eq!(result.keyword, Some("hey_thaleia".to_string()));
        }
    }

    #[test]
    fn test_system_new() {
        let system = WakeWordSystem::new();
        assert!(system.is_ok());
    }

    #[test]
    fn test_system_is_ready() {
        let system = WakeWordSystem::new().unwrap();
        assert!(system.is_ready());
    }

    #[test]
    fn test_system_detect() {
        let mut system = WakeWordSystem::new().unwrap();
        let silent = vec![0.0f32; 16000];
        let result = system.detect(&silent, 16000).unwrap();
        assert!(!result.detected);
    }

    #[test]
    fn test_set_keywords() {
        let mut system = WakeWordSystem::new().unwrap();
        system.set_keywords(vec!["hey_computer".to_string()]);
        // Placeholder doesn't store keywords in result, but should not error
        let silent = vec![0.0f32; 16000];
        let result = system.detect(&silent, 16000).unwrap();
        assert!(!result.detected);
    }
}
