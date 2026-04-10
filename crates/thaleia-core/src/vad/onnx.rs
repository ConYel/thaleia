//! ONNX-based Silero VAD implementation
//!
//! Uses the patched vad-rs crate with ort 2.0.0-rc.12 support.

use crate::vad::engine::{VadBackend, VadConfig, VadResult, VadState};
use crate::Result;

/// Silero VAD - ONNX-based implementation
///
/// Uses the Silero VAD model via the vad-rs crate (patched for ort 2.0.0-rc.12).
#[derive(Debug)]
pub struct OnnxVad {
    /// Configuration
    config: VadConfig,
    /// Inner ONNX VAD model
    inner: vad_rs::Vad,
    /// Current state
    state: VadState,
    /// Speech duration counter (ms)
    speech_duration_ms: u32,
    /// Silence duration counter (ms)
    silence_duration_ms: u32,
}

impl OnnxVad {
    /// Create a new ONNX-based Silero VAD instance.
    pub fn new(config: VadConfig) -> Result<Self> {
        // Get model path - download if needed
        let model_path = get_model_path()?;

        // Create the ONNX VAD with 16kHz sample rate
        let inner = vad_rs::Vad::new(&model_path, 16000)
            .map_err(|e| crate::error::Error::VadError(format!("Failed to create VAD: {:?}", e)))?;

        Ok(Self {
            config,
            inner,
            state: VadState::Idle,
            speech_duration_ms: 0,
            silence_duration_ms: 0,
        })
    }
}

impl VadBackend for OnnxVad {
    fn detect_speech(&mut self, audio: &[f32], sample_rate: u32) -> Result<VadResult> {
        // Run ONNX inference
        let vad_result = self
            .inner
            .compute(audio)
            .map_err(|e| crate::error::Error::VadError(format!("VAD inference failed: {:?}", e)))?;

        let probability = vad_result.prob;
        let is_speech = probability > self.config.threshold;

        // Update state machine
        let samples_ms = (audio.len() as u32 * 1000) / sample_rate.max(1);

        if is_speech {
            self.speech_duration_ms += samples_ms;
            self.silence_duration_ms = 0;

            if (self.state == VadState::Idle || self.state == VadState::Silence)
                && self.speech_duration_ms >= self.config.min_speech_duration_ms
            {
                self.state = VadState::Speaking;
            }
        } else {
            self.silence_duration_ms += samples_ms;
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

/// Get the path to the Silero VAD model.
///
/// Downloads the model if not already cached.
fn get_model_path() -> Result<std::path::PathBuf> {
    // Try multiple cache locations in order of preference
    let possible_dirs: Vec<std::path::PathBuf> = vec![
        std::path::PathBuf::from("/home/devuser/.cache/thaleia"),
        std::path::PathBuf::from("/tmp/thaleia_cache"),
    ];

    let mut cache_dir = None;
    for dir in &possible_dirs {
        if std::fs::create_dir_all(dir).is_ok() {
            cache_dir = Some(dir.clone());
            break;
        }
    }

    // Fallback to system cache dir
    if cache_dir.is_none()
        && let Some(dirs_dir) = dirs::cache_dir()
    {
        let dir = dirs_dir.join("thaleia");
        if std::fs::create_dir_all(&dir).is_ok() {
            cache_dir = Some(dir);
        }
    }

    let cache_dir = cache_dir.ok_or_else(|| {
        crate::error::Error::Config("Could not find writable cache directory".into())
    })?;

    let model_path = cache_dir.join("silero_vad.onnx");

    // Download if not exists
    if !model_path.exists() {
        download_model(&model_path)?;
    }

    Ok(model_path)
}

/// Download the Silero VAD model.
fn download_model(path: &std::path::Path) -> Result<()> {
    let url = "https://github.com/thewh1teagle/vad-rs/releases/download/v0.1.0/silero_vad.onnx";

    // Use reqwest to download
    let mut response = reqwest::blocking::get(url).map_err(|e| {
        crate::error::Error::Network(format!("Failed to download VAD model: {}", e))
    })?;

    if !response.status().is_success() {
        return Err(crate::error::Error::Network(format!(
            "Failed to download VAD model: HTTP {}",
            response.status()
        )));
    }

    let mut file = std::fs::File::create(path)
        .map_err(|e| crate::error::Error::Config(format!("Failed to create model file: {}", e)))?;

    let _ = response.copy_to(&mut file);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vad::VadSystem;
    use std::time::Instant;

    #[test]
    #[ignore] // Requires network to download model
    fn test_onnx_vad_with_model() {
        println!("Testing ONNX VAD with actual model...");

        // Create VAD - this will download model if needed
        let start = Instant::now();
        let mut vad = VadSystem::new().expect("Failed to create VAD");
        println!("VAD created in {:?}", start.elapsed());

        // Check if ready
        assert!(vad.is_ready(), "VAD should be ready");

        // Test with silent audio
        let silent_audio: Vec<f32> = vec![0.0; 16000];
        let result = vad
            .detect_speech(&silent_audio, 16000)
            .expect("Failed to detect");
        println!(
            "Silent audio: prob={:.3}, speaking={}",
            result.probability, result.is_speaking
        );

        // Test with speech-like audio
        let speech_audio: Vec<f32> = (0..16000).map(|i| (i as f32 * 0.05).sin() * 0.5).collect();

        let result = vad
            .detect_speech(&speech_audio, 16000)
            .expect("Failed to detect");
        println!(
            "Speech audio: prob={:.3}, speaking={}",
            result.probability, result.is_speaking
        );

        // Verify VAD runs without crashing - probability depends on audio content
        // The sine wave doesn't contain actual speech features
        assert!(
            result.probability >= 0.0 && result.probability <= 1.0,
            "Probability should be in valid range"
        );
    }

    #[test]
    fn test_onnx_vad_placeholder() {
        // Placeholder test - actual ONNX VAD needs model
        // This tests the module compiles
        let config = VadConfig::default();
        assert_eq!(config.threshold, 0.5);
    }
}
