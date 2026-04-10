//! Audio capture module
//!
//! Uses AudioSystem for cross-platform audio capture.

use crate::audio::AudioSystem;
use crate::{Error, Result, thaleia_debug};
use std::path::Path;
use std::time::Duration;

/// Capture configuration
#[derive(Debug, Clone)]
pub struct CaptureConfig {
    /// Sample rate (default 16000 for Whisper compatibility)
    pub sample_rate: u32,
    /// Number of channels (default 1 for mono)
    pub channels: u16,
}

impl Default for CaptureConfig {
    fn default() -> Self {
        Self {
            sample_rate: 16_000,
            channels: 1,
        }
    }
}

/// Audio captured from microphone
#[derive(Debug)]
pub struct CapturedAudio {
    /// Audio samples as f32 (normalized -1.0 to 1.0)
    pub samples: Vec<f32>,
    /// Actual sample rate used
    pub sample_rate: u32,
    /// Number of channels
    pub channels: u16,
}

impl CapturedAudio {
    /// Convert to i16 samples (for Whisper)
    pub fn to_i16_samples(&self) -> Vec<i16> {
        self.samples
            .iter()
            .map(|&s| (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16)
            .collect()
    }

    /// Save audio to WAV file (16-bit PCM, mono)
    ///
    /// Saves at native sample rate. Whisper handles resampling internally.
    /// Returns the path on success.
    pub fn save_wav(&self, path: &Path) -> Result<String> {
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: self.sample_rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        let mut writer =
            hound::WavWriter::create(path, spec).map_err(|e| Error::AudioDevice(e.to_string()))?;

        for &sample in &self.samples {
            let s16 = (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
            writer
                .write_sample(s16)
                .map_err(|e| Error::AudioDevice(e.to_string()))?;
        }

        writer
            .finalize()
            .map_err(|e| Error::AudioDevice(e.to_string()))?;

        Ok(path.to_string_lossy().to_string())
    }
}

/// Capture audio from microphone using rodio
pub struct AudioCapture;

impl AudioCapture {
    /// Create new audio capture
    pub fn new() -> Result<Self> {
        Ok(Self)
    }

    /// Capture audio for specified duration using AudioSystem
    ///
    /// Note: Sample rate depends on the selected backend.
    /// - SDL2: 44100 Hz (fixed)
    /// - Rodio: Varies by system (typically 44100 or 48000 Hz)
    pub fn capture(&self, duration: Duration) -> Result<CapturedAudio> {
        let mut system = AudioSystem::new();

        thaleia_debug!(
            "AudioCapture: Created new AudioSystem with backend: {}",
            system.backend_name()
        );

        let samples = system
            .capture(duration)
            .map_err(|e| Error::AudioDevice(format!("Audio capture failed: {}", e)))?;

        // Sample rate depends on backend - SDL2 uses 44100Hz
        // Rodio queries actual device sample rate
        let sample_rate = match system.backend_name() {
            "sdl2" => 44100,
            "rodio" => 44100, // Would need backend API change to get actual rate
            _ => 44100,
        };

        thaleia_debug!(
            "AudioCapture: Captured {} samples using {} backend at {}Hz",
            samples.len(),
            system.backend_name(),
            sample_rate
        );

        Ok(CapturedAudio {
            samples,
            sample_rate,
            channels: 1,
        })
    }

    /// Check if microphone is available
    pub fn is_available() -> bool {
        let system = AudioSystem::new();
        system.is_available()
    }
}

impl Default for AudioCapture {
    fn default() -> Self {
        Self::new().expect("Default audio capture should work")
    }
}

/// Information about an audio device
#[derive(Debug, Clone)]
pub struct AudioDeviceInfo {
    /// Device name
    pub name: String,
    /// Whether this is the default input device
    pub is_default: bool,
}

/// List all available audio input devices
pub fn list_audio_devices() -> Result<Vec<AudioDeviceInfo>> {
    use rodio::microphone;

    let inputs = microphone::available_inputs()
        .map_err(|e| Error::AudioDevice(format!("Failed to list input devices: {}", e)))?;

    Ok(inputs
        .iter()
        .map(|input| AudioDeviceInfo {
            name: input.to_string(),
            is_default: false, // rodio doesn't expose default device info
        })
        .collect())
}

/// Print audio device diagnostic info to stderr
/// Returns true if at least one input device was found
pub fn print_audio_diagnostics() -> Result<bool> {
    use rodio::microphone;

    tracing::info!("Audio Diagnostics - checking audio devices");

    // Check environment variables
    tracing::debug!("Audio: Checking environment variables");
    print_env("PULSE_SERVER");
    print_env("PULSE_SINK");
    print_env("PULSE_SOURCE");
    print_env("ALSA_CARD");
    print_env("PIPEWIRE_RUNTIME_DIR");
    print_env("XDG_RUNTIME_DIR");

    // List input devices
    tracing::info!("Scanning for input devices (microphones)");
    let inputs = match microphone::available_inputs() {
        Ok(inputs) => inputs,
        Err(e) => {
            tracing::error!("Failed to list input devices: {}", e);
            return Ok(false);
        }
    };

    if inputs.is_empty() {
        tracing::warn!("No input devices found!");
        tracing::info!("Troubleshooting steps:");
        tracing::info!("  - Linux (PipeWire): Check if PipeWire is running: wpctl status");
        tracing::info!("  - Linux (PulseAudio): Check if PulseAudio is running");
        tracing::info!("  - Linux (ALSA): Check ~/.asoundrc and /etc/asound.conf");
        tracing::info!("  - macOS: Check System Preferences > Privacy > Microphone");
        tracing::info!("  - Qubes: Configure audio input via qubes.xml or dom0 settings");
        tracing::info!("  - USB: Ensure device is connected and recognized (lsusb)");
        return Ok(false);
    }

    for (i, input) in inputs.iter().enumerate() {
        tracing::info!("Input device {}: {}", i + 1, input);
    }

    // Also check output devices using rodio
    tracing::info!("Checking output devices (speakers/headphones)");
    match rodio::DeviceSinkBuilder::open_default_sink() {
        Ok(_) => tracing::info!("Default output device available"),
        Err(_) => tracing::warn!("No output devices found"),
    }

    Ok(true)
}

fn print_env(key: &str) {
    if let Ok(val) = std::env::var(key) {
        tracing::debug!("  {}={}", key, val);
    }
}

#[cfg(test)]
#[cfg(feature = "playback")]
mod tests {
    use super::*;

    #[test]
    fn test_capture_config_default() {
        let config = CaptureConfig::default();
        assert_eq!(config.sample_rate, 16_000);
        assert_eq!(config.channels, 1);
    }

    #[test]
    fn test_captured_audio_to_i16() {
        let audio = CapturedAudio {
            samples: vec![0.0, 0.5, 1.0, -0.5, -1.0],
            sample_rate: 16000,
            channels: 1,
        };
        let i16_samples = audio.to_i16_samples();
        assert_eq!(i16_samples.len(), 5);
        assert_eq!(i16_samples[2], i16::MAX);
        // -1.0 gets converted to -1.0 * 32767 = -32767.0, cast to i16 = -32767
        assert_eq!(i16_samples[4], -32767);
    }

    #[test]
    fn test_audio_device_info_debug() {
        let info = AudioDeviceInfo {
            name: "Test Mic".to_string(),
            is_default: true,
        };
        let debug_str = format!("{:?}", info);
        assert!(debug_str.contains("Test Mic"));
    }

    #[test]
    fn test_captured_audio_save_wav() {
        use std::fs;

        let audio = CapturedAudio {
            samples: vec![0.0, 0.5, 1.0, -0.5, -1.0],
            sample_rate: 16000,
            channels: 1,
        };

        let path = std::path::Path::new("/tmp/test_capture.wav");
        let result = audio.save_wav(path);
        assert!(result.is_ok());

        // Verify file exists and has correct format
        let metadata = fs::metadata(path).unwrap();
        assert!(metadata.len() > 0);

        // Clean up
        let _ = fs::remove_file(path);
    }
}
