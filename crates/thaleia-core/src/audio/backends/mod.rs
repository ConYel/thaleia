//! Audio backends module
//!
//! Provides trait definition and implementations for different audio systems.

pub mod null_backend;

#[cfg(feature = "rodio")]
pub mod rodio_backend;

#[cfg(feature = "sdl2-audio")]
pub mod sdl2_backend;

use std::time::Duration;
use thiserror::Error;

/// Audio backend trait - minimal interface for audio operations
///
/// All implementations MUST provide:
/// - `is_available()` - Check if backend can produce sound
/// - `play()` - Play audio samples
/// - `capture()` - Capture audio from microphone
///
/// Note: Due to audio library constraints, implementations may not be `Send + Sync`.
/// Use interior mutability (Mutex) for thread-safe access.
pub trait AudioBackend {
    /// Get the backend type name
    fn backend_name(&self) -> &'static str {
        "unknown"
    }

    /// Check if this backend is available on this system
    fn is_available(&self) -> bool;

    /// Play raw audio samples
    ///
    /// # Arguments
    /// * `samples` - f32 samples in range [-1.0, 1.0]
    /// * `sample_rate` - Sample rate in Hz (e.g., 24000, 44100)
    ///
    /// # Errors
    /// Returns `AudioError::PlaybackFailed` if playback fails
    fn play(&mut self, samples: &[f32], sample_rate: u32) -> Result<(), AudioError>;

    /// Capture audio from microphone
    ///
    /// # Arguments
    /// * `duration` - How long to capture
    ///
    /// # Returns
    /// * `Vec<f32>` - Captured samples in range [-1.0, 1.0], mono
    ///
    /// # Errors
    /// Returns `AudioError::CaptureFailed` if capture fails
    fn capture(&mut self, duration: Duration) -> Result<Vec<f32>, AudioError>;
}

/// Audio errors
#[derive(Error, Debug)]
pub enum AudioError {
    #[error("Backend not available: {0}")]
    NotAvailable(String),

    #[error("Playback failed: {0}")]
    PlaybackFailed(String),

    #[error("Capture failed: {0}")]
    CaptureFailed(String),

    #[error("Backend error: {0}")]
    Backend(String),

    #[error("No audio backend available")]
    NoBackend,
}

/// Backend type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendType {
    /// No audio backend - file-only mode
    None,
    /// Rodio/cpal backend (ALSA - standard Linux)
    #[cfg(feature = "rodio")]
    Rodio,
    /// SDL2 backend (PulseAudio - Qubes/custom systems)
    #[cfg(feature = "sdl2-audio")]
    SDL2,
}

/// Unified audio system with automatic backend selection
///
/// # Auto-Detection
///
/// `AudioSystem::new()` tries backends in order:
/// 1. Rodio (standard Linux with ALSA)
/// 2. SDL2 (Qubes/PulseAudio)
/// 3. Null (file-only fallback)
///
/// # Manual Selection
///
/// Use `AudioSystem::with_backend()` to force a specific backend.
#[derive(Debug)]
#[non_exhaustive]
pub enum AudioSystem {
    /// No audio - file-only mode (always available)
    None,
    /// Rodio/cpal backend (ALSA)
    #[cfg(feature = "rodio")]
    Rodio(rodio_backend::RodioBackend),
    /// SDL2 backend (PulseAudio)
    #[cfg(feature = "sdl2-audio")]
    SDL2(sdl2_backend::SDL2Backend),
}

impl AudioSystem {
    /// Create a new AudioSystem with automatic backend detection
    ///
    /// Tries backends in order until one is available.
    /// Returns `AudioSystem::None` if no backend works.
    ///
    /// For Qubes/PulseAudio systems, SDL2 is tried first when the sdl2-audio
    /// feature is enabled, since Rodio uses ALSA which is blocked on Qubes.
    pub fn new() -> Self {
        // Try SDL2 first for Qubes/PulseAudio systems
        #[cfg(feature = "sdl2-audio")]
        {
            if let Ok(backend) = sdl2_backend::SDL2Backend::new() {
                // Check availability using trait method - trait is defined in this module
                if AudioBackend::is_available(&backend) {
                    tracing::debug!("Audio: Selected SDL2 backend");
                    return AudioSystem::SDL2(backend);
                }
            } else {
                tracing::debug!("SDL2 backend failed to initialize, trying other backends");
            }
        }

        // Try Rodio (best for standard Linux with ALSA)
        #[cfg(feature = "rodio")]
        {
            if let Ok(backend) = rodio_backend::RodioBackend::new() {
                // Check availability using trait method - trait is defined in this module
                if AudioBackend::is_available(&backend) {
                    tracing::debug!("Audio: Selected Rodio backend");
                    return AudioSystem::Rodio(backend);
                }
            } else {
                tracing::debug!("Rodio backend failed to initialize, trying other backends");
            }
        }

        tracing::warn!("Audio: No audio backend available");
        AudioSystem::None
    }

    /// Create AudioSystem with a specific backend
    ///
    /// # Errors
    /// Returns error if the specified backend is not available
    pub fn with_backend(backend_type: BackendType) -> std::result::Result<Self, AudioError> {
        match backend_type {
            #[cfg(feature = "rodio")]
            BackendType::Rodio => {
                let backend = rodio_backend::RodioBackend::new()
                    .map_err(|e| AudioError::NotAvailable(e.to_string()))?;
                Ok(AudioSystem::Rodio(backend))
            }
            #[cfg(feature = "sdl2-audio")]
            BackendType::SDL2 => {
                let backend = sdl2_backend::SDL2Backend::new()
                    .map_err(|e| AudioError::NotAvailable(e.to_string()))?;
                Ok(AudioSystem::SDL2(backend))
            }
            BackendType::None => Ok(AudioSystem::None),
        }
    }

    /// Get the backend type name
    pub fn backend_name(&self) -> &'static str {
        match self {
            #[cfg(feature = "rodio")]
            AudioSystem::Rodio(_) => "rodio",
            #[cfg(feature = "sdl2-audio")]
            AudioSystem::SDL2(_) => "sdl2",
            AudioSystem::None => "none",
        }
    }

    /// Check if audio playback is available
    pub fn is_available(&self) -> bool {
        match self {
            #[cfg(feature = "rodio")]
            AudioSystem::Rodio(b) => b.is_available(),
            #[cfg(feature = "sdl2-audio")]
            AudioSystem::SDL2(b) => b.is_available(),
            AudioSystem::None => false,
        }
    }

    /// Play audio samples
    ///
    /// If no backend is available, logs a warning and returns Ok.
    pub fn play(&mut self, samples: &[f32], sample_rate: u32) -> Result<(), AudioError> {
        match self {
            #[cfg(feature = "rodio")]
            AudioSystem::Rodio(b) => b.play(samples, sample_rate),
            #[cfg(feature = "sdl2-audio")]
            AudioSystem::SDL2(b) => b.play(samples, sample_rate),
            AudioSystem::None => {
                let _ = (samples, sample_rate); // Silence unused warnings
                tracing::warn!("Audio: play() called but no backend available");
                Ok(())
            }
        }
    }

    /// Capture audio from microphone
    pub fn capture(&mut self, duration: Duration) -> Result<Vec<f32>, AudioError> {
        match self {
            #[cfg(feature = "rodio")]
            AudioSystem::Rodio(b) => b.capture(duration),
            #[cfg(feature = "sdl2-audio")]
            AudioSystem::SDL2(b) => b.capture(duration),
            AudioSystem::None => {
                let _ = duration; // Silence unused warning
                tracing::warn!("Audio: capture() called but no backend available");
                Err(AudioError::NoBackend)
            }
        }
    }
}

impl Default for AudioSystem {
    fn default() -> Self {
        Self::new()
    }
}
