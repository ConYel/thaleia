//! Rodio/cpal backend for audio
//!
//! Uses rodio with cpal for ALSA audio on standard Linux.
//!
//! ## Requirements
//!
//! - ALSA development files (`libasound2-dev`)
//! - Audio hardware accessible (not blocked by Qubes)
//!
//! ## Capabilities
//!
//! | Feature | Status |
//! |---------|--------|
//! | Playback | ✅ |
//! | Capture | ✅ |
//! | Standard Linux | ✅ |
//! | Qubes/PulseAudio | ❌ |

use crate::audio::backends::{AudioBackend, AudioError};
use crate::thaleia_debug;
use rodio::{Decoder, DeviceSinkBuilder, Player, Source};
use std::fs::File;
use std::io::BufReader;
use std::time::Duration;

/// Rodio/cpal backend for audio playback and capture
///
/// This backend uses:
/// - rodio for high-level audio API
/// - cpal for low-level ALSA access
///
/// Best for standard Linux systems with direct audio hardware access.
#[derive(Debug)]
pub struct RodioBackend {
    _private: (),
}

impl RodioBackend {
    /// Create a new RodioBackend
    ///
    /// Returns error if rodio cannot be initialized.
    /// Use `is_available()` to check if audio actually works.
    pub fn new() -> Result<Self, AudioError> {
        Ok(Self { _private: () })
    }

    /// Check if ALSA audio devices are accessible
    ///
    /// This checks if playback and capture devices can be enumerated.
    /// Returns `true` if both playback and at least one input device exist.
    /// Note: We don't try to actually configure the microphone since that
    /// requires direct hardware access which may not be available in containers.
    pub fn check_available() -> bool {
        thaleia_debug!("Rodio: Checking audio availability...");

        // Test playback - must work
        if DeviceSinkBuilder::open_default_sink().is_err() {
            thaleia_debug!("Rodio: Playback device not available");
            return false;
        }
        thaleia_debug!("Rodio: Playback device OK");

        // Test capture - just check if devices can be listed
        // We don't try to actually configure the microphone because:
        // 1. In containers, /dev/snd may not be accessible
        // 2. PulseAudio/PipeWire devices work via enumeration but fail on open
        // 3. The actual capture will handle device errors if needed
        if let Ok(inputs) = rodio::microphone::available_inputs()
            && !inputs.is_empty()
        {
            thaleia_debug!("Rodio: Found {} input device(s)", inputs.len());
            thaleia_debug!("Rodio: Both playback and capture available");
            return true;
        }

        thaleia_debug!("Rodio: No capture devices found");
        false
    }
}

impl AudioBackend for RodioBackend {
    fn backend_name(&self) -> &'static str {
        "rodio"
    }

    fn is_available(&self) -> bool {
        Self::check_available()
    }

    fn play(&mut self, samples: &[f32], sample_rate: u32) -> Result<(), AudioError> {
        // Convert samples to WAV
        let wav_data = samples_to_wav(samples, sample_rate)
            .map_err(|e| AudioError::PlaybackFailed(e.to_string()))?;

        // Write to temp file (rodio needs a file or stream)
        let temp_path = "/tmp/thaleia_rodio_playback.wav";
        std::fs::write(temp_path, &wav_data)
            .map_err(|e| AudioError::PlaybackFailed(format!("Failed to write temp file: {}", e)))?;

        // Open and decode
        let file = File::open(temp_path)
            .map_err(|e| AudioError::PlaybackFailed(format!("Failed to open temp file: {}", e)))?;
        let reader = BufReader::new(file);
        let source = Decoder::try_from(reader)
            .map_err(|e| AudioError::PlaybackFailed(format!("Failed to decode audio: {}", e)))?;

        // Play using DeviceSinkBuilder and Player (rodio 0.22 API)
        let sink = DeviceSinkBuilder::open_default_sink().map_err(|e| {
            AudioError::PlaybackFailed(format!("Failed to open audio device: {}", e))
        })?;

        let mixer = sink.mixer();
        let player = Player::connect_new(mixer);
        player.append(source);
        player.sleep_until_end();

        tracing::debug!("Rodio: Played {} samples", samples.len());
        Ok(())
    }

    fn capture(&mut self, duration: Duration) -> Result<Vec<f32>, AudioError> {
        use rodio::microphone::{self, MicrophoneBuilder};

        // Get available inputs
        let inputs = microphone::available_inputs().map_err(|e| {
            AudioError::CaptureFailed(format!("Failed to list input devices: {}", e))
        })?;

        if inputs.is_empty() {
            return Err(AudioError::CaptureFailed("No input devices found".into()));
        }

        // Try each available device until one works
        // This is important for containerized environments where some devices
        // may be listed but not actually usable
        let mut last_error = None;

        for (i, device) in inputs.iter().enumerate() {
            tracing::info!("Rodio: Trying input device {}: {}", i + 1, device);

            match MicrophoneBuilder::new()
                .device(device.clone())
                .and_then(|b| b.default_config())
            {
                Ok(config) => {
                    match config.open_stream() {
                        Ok(mic) => {
                            let sample_rate = mic.sample_rate().get();
                            let channels = mic.channels().get();

                            tracing::debug!(
                                "Rodio: Microphone config: {} channels, {} Hz",
                                channels,
                                sample_rate
                            );

                            // Record for duration
                            let recording = mic.take_duration(duration);

                            // Collect samples - rodio 0.22 sources are directly iterable
                            let samples: Vec<f32> = recording.collect();

                            // Convert to mono if stereo
                            let audio_samples = if channels > 1 {
                                let chunk_size = samples.len() / channels as usize;
                                (0..chunk_size)
                                    .map(|i| {
                                        let sum: f32 = (0..channels as usize)
                                            .map(|ch| samples[i + ch * chunk_size])
                                            .sum();
                                        sum / channels as f32
                                    })
                                    .collect()
                            } else {
                                samples
                            };

                            tracing::debug!("Rodio: Captured {} samples", audio_samples.len());
                            return Ok(audio_samples);
                        }
                        Err(e) => {
                            tracing::debug!(
                                "Rodio: Failed to open stream on device {}: {}",
                                device,
                                e
                            );
                            last_error =
                                Some(format!("Failed to open stream on {}: {}", device, e));
                        }
                    }
                }
                Err(e) => {
                    tracing::debug!("Rodio: Failed to configure device {}: {}", device, e);
                    last_error = Some(format!("Failed to configure {}: {}", device, e));
                }
            }
        }

        // All devices failed
        Err(AudioError::CaptureFailed(format!(
            "Failed to capture from all {} available devices. Last error: {}",
            inputs.len(),
            last_error.unwrap_or_else(|| "Unknown error".to_string())
        )))
    }
}

/// Convert f32 samples to WAV bytes
pub fn samples_to_wav(samples: &[f32], sample_rate: u32) -> std::io::Result<Vec<u8>> {
    use hound::{WavSpec, WavWriter};
    use std::io::Cursor;

    let samples_i16: Vec<i16> = samples
        .iter()
        .map(|&s| (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16)
        .collect();

    let mut wav_data = Vec::new();
    let spec = WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    {
        let mut cursor = Cursor::new(&mut wav_data);
        let mut writer = WavWriter::new(&mut cursor, spec)
            .map_err(std::io::Error::other)?;

        for sample in samples_i16 {
            writer
                .write_sample(sample)
                .map_err(std::io::Error::other)?;
        }
        writer
            .finalize()
            .map_err(std::io::Error::other)?;
    }

    Ok(wav_data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backend_creation() {
        let backend = RodioBackend::new();
        assert!(backend.is_ok());
    }

    #[test]
    fn test_samples_to_wav() {
        let samples: Vec<f32> = vec![0.0f32; 16000];
        let wav = samples_to_wav(&samples, 16000).unwrap();

        // Verify WAV header
        assert!(wav.len() > 44);
        assert_eq!(&wav[0..4], b"RIFF");
        assert_eq!(&wav[8..12], b"WAVE");
    }
}
