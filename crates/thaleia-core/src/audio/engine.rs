//! Audio engine - unified audio interface
//!
//! Wrapper around AudioSystem for backwards compatibility.
//! New code should use AudioSystem directly.

use crate::audio::backends::{AudioError, AudioSystem, BackendType};

/// Audio sample type (16-bit signed PCM)
pub type Sample = i16;

/// Audio chunk containing samples
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct AudioChunk {
    /// Audio samples
    pub samples: Vec<Sample>,
    /// Sample rate (typically 16000 or 24000)
    pub sample_rate: u32,
    /// Number of channels (typically 1 = mono)
    pub channels: u16,
}

impl AudioChunk {
    /// Create new audio chunk
    #[allow(dead_code)]
    pub fn new(samples: Vec<Sample>, sample_rate: u32, channels: u16) -> Self {
        Self {
            samples,
            sample_rate,
            channels,
        }
    }

    /// Duration in seconds
    #[allow(dead_code)]
    pub fn duration_secs(&self) -> f64 {
        self.samples.len() as f64 / self.sample_rate as f64
    }
}

/// Audio engine for capture and playback
///
/// This is a wrapper around AudioSystem for backwards compatibility.
/// New code should use AudioSystem directly.
#[derive(Debug)]
pub struct AudioEngine {
    audio: AudioSystem,
}

impl AudioEngine {
    /// Create new audio engine with automatic backend detection
    pub fn new() -> Self {
        Self {
            audio: AudioSystem::new(),
        }
    }

    /// Create audio engine with specific backend
    pub fn with_backend(backend: BackendType) -> Result<Self, AudioError> {
        Ok(Self {
            audio: AudioSystem::with_backend(backend)?,
        })
    }

    /// Get the audio system
    pub fn audio_system(&self) -> &AudioSystem {
        &self.audio
    }

    /// Get mutable reference to audio system
    pub fn audio_system_mut(&mut self) -> &mut AudioSystem {
        &mut self.audio
    }

    /// Get the backend name
    pub fn backend_name(&self) -> &'static str {
        self.audio.backend_name()
    }

    /// Check if audio is available
    pub fn is_available(&self) -> bool {
        self.audio.is_available()
    }
}

impl Default for AudioEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_chunk_duration() {
        let chunk = AudioChunk::new(vec![0; 16_000], 16_000, 1);
        assert!((chunk.duration_secs() - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_audio_engine_creation() {
        let engine = AudioEngine::new();
        // Should always create (may be no-backend mode)
        assert!(!engine.backend_name().is_empty());
    }

    #[test]
    fn test_audio_engine_with_backend() {
        // Test with null backend (always available)
        let result = AudioEngine::with_backend(BackendType::None);
        assert!(result.is_ok());
    }
}
