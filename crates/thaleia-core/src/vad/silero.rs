//! Silero VAD implementation
//!
//! Energy-based placeholder for VAD.
//!
//! TODO: Proper integration with ONNX-based Silero VAD requires resolving
//! ndarray version conflicts between vad-rs (0.16) and kokoro-tiny (0.17).

use crate::Result;
use crate::vad::engine::{VadBackend, VadConfig, VadResult, VadState};

/// Silero VAD - energy-based placeholder implementation
///
/// Note: This is a placeholder using simple energy detection.
/// For proper Silero VAD, we need to resolve the ndarray version conflict:
///
///   - vad-rs uses ndarray 0.16
///   - kokoro-tiny uses ndarray 0.17
///
/// A patch has been created at patches/vad_rs but needs more work to integrate.
#[derive(Debug)]
pub struct SileroVad {
    /// Configuration
    config: VadConfig,
    /// Current state
    state: VadState,
    /// Speech duration counter (ms)
    speech_duration_ms: u32,
    /// Silence duration counter (ms)
    silence_duration_ms: u32,
}

impl SileroVad {
    /// Create a new Silero VAD instance.
    pub fn new(config: VadConfig) -> Result<Self> {
        Ok(Self {
            config,
            state: VadState::Idle,
            speech_duration_ms: 0,
            silence_duration_ms: 0,
        })
    }
}

impl VadBackend for SileroVad {
    fn detect_speech(&mut self, audio: &[f32], sample_rate: u32) -> Result<VadResult> {
        // Simple energy-based detection as placeholder
        // Calculate RMS energy correctly: sqrt(sum of squares / count)
        let sum_squares: f32 = audio.iter().map(|&s| s * s).sum();
        let len = audio.len().max(1) as f32;
        let energy = (sum_squares / len).sqrt();

        // Simple threshold-based detection
        let probability = if energy > 0.01 { 0.8 } else { 0.1 };
        let is_speech = probability > self.config.threshold;

        // Update state machine
        if is_speech {
            self.speech_duration_ms += (audio.len() as u32 * 1000) / sample_rate;
            self.silence_duration_ms = 0;

            if (self.state == VadState::Idle || self.state == VadState::Silence)
                && self.speech_duration_ms >= self.config.min_speech_duration_ms
            {
                self.state = VadState::Speaking;
            }
        } else {
            self.silence_duration_ms += (audio.len() as u32 * 1000) / sample_rate;
            self.speech_duration_ms = 0;

            if self.state == VadState::Speaking
                && self.silence_duration_ms >= self.config.min_silence_duration_ms
            {
                self.state = VadState::Ending;
            } else if self.state == VadState::Ending
                && self.silence_duration_ms >= self.config.min_silence_duration_ms
            {
                self.state = VadState::Silence;
            }
        }

        Ok(VadResult {
            is_speaking: self.state == VadState::Speaking || self.state == VadState::Ending,
            probability,
            state: self.state,
        })
    }

    fn is_ready(&self) -> bool {
        true
    }

    fn config(&self) -> VadConfig {
        self.config.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vad_config_default() {
        let config = VadConfig::default();
        assert_eq!(config.threshold, 0.5);
        assert_eq!(config.min_speech_duration_ms, 250);
        assert_eq!(config.min_silence_duration_ms, 300);
        assert_eq!(config.sample_rate, 16000);
    }

    #[test]
    fn test_vad_config_custom() {
        let config = VadConfig {
            threshold: 0.7,
            min_speech_duration_ms: 500,
            min_silence_duration_ms: 400,
            sample_rate: 16000,
        };
        assert_eq!(config.threshold, 0.7);
        assert_eq!(config.min_speech_duration_ms, 500);
    }

    #[test]
    fn test_vad_state_default() {
        let state = VadState::default();
        assert_eq!(state, VadState::Idle);
    }

    #[test]
    fn test_vad_result_default() {
        let result = VadResult {
            is_speaking: false,
            probability: 0.0,
            state: VadState::Idle,
        };
        assert!(!result.is_speaking);
        assert_eq!(result.probability, 0.0);
    }

    #[test]
    fn test_silero_vad_initialization() {
        let vad = SileroVad::new(VadConfig::default());
        assert!(vad.is_ok());
    }

    #[test]
    fn test_silero_vad_silent_audio() {
        let mut vad = SileroVad::new(VadConfig::default()).unwrap();

        // Silent audio (zeros)
        let silent_audio: Vec<f32> = vec![0.0; 16000];
        let result = vad.detect_speech(&silent_audio, 16000).unwrap();

        // Silent audio should have low probability
        assert!(
            result.probability < 0.5,
            "Silent audio should have low probability"
        );
    }

    #[test]
    fn test_silero_vad_with_audio() {
        let mut vad = SileroVad::new(VadConfig::default()).unwrap();

        // Generate audio with some energy (simulating speech)
        let speech_audio: Vec<f32> = (0..16000).map(|i| (i as f32 * 0.01).sin() * 0.1).collect();

        let result = vad.detect_speech(&speech_audio, 16000).unwrap();

        // Audio with energy should have higher probability
        assert!(
            result.probability > 0.5,
            "Speech audio should have higher probability"
        );
    }
}
