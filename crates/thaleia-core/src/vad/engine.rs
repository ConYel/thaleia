//! VAD (Voice Activity Detection) engine and types
//!
//! Provides voice activity detection with pluggable backends.
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
//! // Create VAD system
//! let mut vad = VadSystem::new()?;
//!
//! // Detect speech in audio
//! let result = vad.detect_speech(&audio_samples, 16000)?;
//! if result.is_speaking {
//!     println!("User is speaking!");
//! }
//! ```

use crate::Result;

// =============================================================================
// Domain Types
// =============================================================================

/// VAD configuration with sensible defaults
///
/// # Example
/// ```rust,ignore
/// use thaleia_core::vad::VadConfig;
///
/// let config = VadConfig::default();
/// // Or customize:
/// let config = VadConfig {
///     threshold: 0.7,
///     min_speech_duration_ms: 500,
///     ..Default::default()
/// };
/// ```
#[derive(Debug, Clone)]
pub struct VadConfig {
    /// Speech probability threshold (0.0 - 1.0)
    /// Higher = more strict (less false positives)
    /// Lower = more sensitive (more false positives)
    pub threshold: f32,
    /// Minimum speech duration in milliseconds
    pub min_speech_duration_ms: u32,
    /// Minimum silence duration in milliseconds to end speech
    pub min_silence_duration_ms: u32,
    /// Audio sample rate (typically 16000)
    pub sample_rate: u32,
}

impl Default for VadConfig {
    fn default() -> Self {
        Self {
            threshold: 0.5,
            min_speech_duration_ms: 250,
            min_silence_duration_ms: 300,
            sample_rate: 16000,
        }
    }
}

/// VAD detection result
#[derive(Debug, Clone)]
pub struct VadResult {
    /// Whether speech is detected
    pub is_speaking: bool,
    /// Speech probability (0.0 - 1.0)
    pub probability: f32,
    /// Current VAD state
    pub state: VadState,
}

/// VAD state machine states
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VadState {
    /// No speech detected
    #[default]
    Idle,
    /// Speech detected
    Speaking,
    /// Speech ending (in min_silence_duration)
    Ending,
    /// Silence (after speech ends)
    Silence,
}

// =============================================================================
// VadBackend Trait
// =============================================================================

/// Trait for VAD (Voice Activity Detection) backends.
///
/// Implement this trait to add new VAD backends.
/// Follows ISP: Provides essential methods without bloating the interface.
///
/// # Required Methods
///
/// - `detect_speech()` - Detect speech in audio
/// - `is_ready()` - Check if backend is initialized
///
/// # Optional Methods (with default implementations)
///
/// - `config()` - Get current configuration
///
/// # Example
///
/// ```rust,ignore
/// use thaleia_core::vad::{VadBackend, VadConfig, VadResult};
///
/// struct MyVadEngine {
///     model: MyModel,
/// }
///
/// impl VadBackend for MyVadEngine {
///     fn detect_speech(&mut self, audio: &[f32], sample_rate: u32) -> Result<VadResult> {
///         let prob = self.model.predict(audio, sample_rate)?;
///         Ok(VadResult {
///             is_speaking: prob > 0.5,
///             probability: prob,
///             state: VadState::Speaking,
///         })
///     }
///
///     fn is_ready(&self) -> bool {
///         self.model.is_loaded()
///     }
/// }
/// ```
pub trait VadBackend {
    /// Detect speech in audio samples.
    ///
    /// # Parameters
    /// * `audio` - Audio samples as f32 (mono)
    /// * `sample_rate` - Sample rate of the audio (e.g., 16000)
    ///
    /// # Returns
    /// * `Ok(VadResult)` - Detection result with probability and state
    /// * `Err(Error)` - If detection fails
    fn detect_speech(&mut self, audio: &[f32], sample_rate: u32) -> Result<VadResult>;

    /// Check if the backend is ready for detection.
    fn is_ready(&self) -> bool;

    /// Get current VAD configuration.
    fn config(&self) -> VadConfig {
        VadConfig::default()
    }
}

// =============================================================================
// VadSystem Factory
// =============================================================================

/// Unified VAD system with backend selection
///
/// # Auto-Detection
///
/// `VadSystem::new()` creates a default backend (ONNX-based if vad feature enabled).
///
/// # Manual Selection
///
/// Use `VadSystem::with_config()` to customize configuration.
#[derive(Debug)]
pub enum VadSystem {
    /// ONNX-based Silero VAD (real ML model)
    #[cfg(feature = "vad")]
    Onnx(super::onnx::OnnxVad),
    /// Energy-based fallback (simple but works without ONNX model)
    #[cfg(not(feature = "vad"))]
    Energy(super::silero::SileroVad),
}

impl VadSystem {
    /// Create a new VAD system with default configuration.
    pub fn new() -> crate::error::Result<Self> {
        let config = VadConfig::default();
        Self::with_config(config)
    }

    /// Create VAD system with custom configuration.
    #[cfg(feature = "vad")]
    pub fn with_config(config: VadConfig) -> crate::error::Result<Self> {
        let backend = super::onnx::OnnxVad::new(config)?;
        Ok(VadSystem::Onnx(backend))
    }

    /// Create VAD system with custom configuration (energy-based fallback).
    #[cfg(not(feature = "vad"))]
    pub fn with_config(config: VadConfig) -> crate::error::Result<Self> {
        let backend = super::silero::SileroVad::new(config)?;
        Ok(VadSystem::Energy(backend))
    }

    /// Check if VAD is ready.
    pub fn is_ready(&self) -> bool {
        match self {
            #[cfg(feature = "vad")]
            VadSystem::Onnx(b) => b.is_ready(),
            #[cfg(not(feature = "vad"))]
            VadSystem::Energy(b) => b.is_ready(),
        }
    }

    /// Detect speech in audio.
    pub fn detect_speech(
        &mut self,
        audio: &[f32],
        sample_rate: u32,
    ) -> crate::error::Result<VadResult> {
        match self {
            #[cfg(feature = "vad")]
            VadSystem::Onnx(b) => b.detect_speech(audio, sample_rate),
            #[cfg(not(feature = "vad"))]
            VadSystem::Energy(b) => b.detect_speech(audio, sample_rate),
        }
    }
}
