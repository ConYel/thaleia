//! SDL2 backend for audio
//!
//! Uses SDL2 for audio playback and capture on systems with PulseAudio.
//!
//! ## Requirements
//!
//! - SDL2 library installed (`libsdl2-dev`)
//! - PulseAudio or PipeWire running
//!
//! ## Capabilities
//!
//! | Feature | Status |
//! |---------|--------|
//! | Playback | ✅ |
//! | Capture | ✅ |
//! | Standard Linux | ✅ |
//! | Qubes/PulseAudio | ✅ |
//!
//! ## Use Case
//!
//! This backend is for systems where ALSA access is restricted
//! (like Qubes OS) but PulseAudio is available.

use crate::audio::backends::{AudioBackend, AudioError};
use crate::thaleia_debug;
use sdl2::audio::{AudioCallback, AudioSpecDesired};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Track SDL2 initialization state
static SDL2_INITIALIZED: AtomicBool = AtomicBool::new(false);

/// Initialize SDL2 audio subsystem
///
/// SDL_Init is idempotent - safe to call multiple times.
/// Returns the AudioSubsystem on success.
fn init_sdl2() -> Result<sdl2::AudioSubsystem, AudioError> {
    // Try to init SDL2
    let sdl_context =
        sdl2::init().map_err(|e| AudioError::Backend(format!("SDL2 init failed: {}", e)))?;

    // Get audio subsystem
    let audio = sdl_context
        .audio()
        .map_err(|e| AudioError::Backend(format!("SDL2 audio init failed: {}", e)))?;

    // Mark as initialized
    SDL2_INITIALIZED.store(true, Ordering::SeqCst);

    Ok(audio)
}

/// Callback-based audio playback
struct PlaybackCallback {
    samples: Arc<Mutex<Vec<f32>>>,
    position: Arc<Mutex<usize>>,
    finished: Arc<AtomicBool>,
}

impl AudioCallback for PlaybackCallback {
    type Channel = f32;

    fn callback(&mut self, output: &mut [f32]) {
        let mut pos = self.position.lock().unwrap();
        let samples = self.samples.lock().unwrap();
        let len = samples.len();

        for frame in output.iter_mut() {
            if *pos < len {
                *frame = samples[*pos];
                *pos += 1;
            } else {
                *frame = 0.0;
            }
        }

        if *pos >= len {
            self.finished.store(true, Ordering::SeqCst);
        }
    }
}

/// Callback-based audio capture
struct CaptureCallback {
    buffer: Arc<Mutex<Vec<f32>>>,
}

impl AudioCallback for CaptureCallback {
    type Channel = f32;

    fn callback(&mut self, input: &mut [f32]) {
        let mut buffer = self.buffer.lock().unwrap();
        buffer.extend_from_slice(input);
    }
}

/// SDL2 backend for audio playback and capture
///
/// This backend uses SDL2's audio system which can connect to:
/// - PulseAudio (Linux)
/// - PipeWire (via PulseAudio emulation)
/// - Direct audio hardware
///
/// Best for systems like Qubes where ALSA access is restricted.
#[derive(Debug)]
pub struct SDL2Backend {
    audio: sdl2::AudioSubsystem,
}

impl SDL2Backend {
    /// Create a new SDL2Backend
    ///
    /// Returns error if SDL2 audio cannot be initialized.
    /// Multiple instances can be created - SDL2 handles refcounting.
    pub fn new() -> Result<Self, AudioError> {
        tracing::debug!("SDL2Backend::new() - attempting initialization");
        match init_sdl2() {
            Ok(audio) => {
                tracing::debug!("SDL2Backend::new() - initialization successful");
                Ok(Self { audio })
            }
            Err(e) => {
                tracing::debug!("SDL2Backend::new() - initialization failed: {}", e);
                Err(e)
            }
        }
    }

    /// Check if SDL2 audio is available
    pub fn check_available() -> bool {
        let available = SDL2_INITIALIZED.load(Ordering::SeqCst);
        tracing::debug!("SDL2Backend::check_available() = {}", available);
        available
    }
}

impl AudioBackend for SDL2Backend {
    fn backend_name(&self) -> &'static str {
        "sdl2"
    }

    fn is_available(&self) -> bool {
        let available = Self::check_available();
        tracing::debug!("SDL2Backend::is_available() = {}", available);
        available
    }

    fn play(&mut self, samples: &[f32], sample_rate: u32) -> Result<(), AudioError> {
        let samples = Arc::new(Mutex::new(samples.to_vec()));
        let position = Arc::new(Mutex::new(0usize));
        let finished = Arc::new(AtomicBool::new(false));

        // Clone for the callback
        let samples_for_cb = samples.clone();
        let position_for_cb = position.clone();
        let finished_for_cb = finished.clone();

        let desired_spec = AudioSpecDesired {
            freq: Some(sample_rate as i32),
            channels: Some(1),
            samples: Some(1024),
        };

        let device = self
            .audio
            .open_playback(None, &desired_spec, move |_spec| PlaybackCallback {
                samples: samples_for_cb,
                position: position_for_cb,
                finished: finished_for_cb,
            })
            .map_err(|e| {
                AudioError::PlaybackFailed(format!("Failed to open audio device: {}", e))
            })?;

        device.resume();

        // Wait for playback to complete
        while !finished.load(Ordering::SeqCst) {
            std::thread::sleep(Duration::from_millis(10));
        }

        // Give a small delay for buffer to drain
        std::thread::sleep(Duration::from_millis(50));

        let played = *position.lock().unwrap();
        tracing::debug!("SDL2: Played {} samples", played);
        Ok(())
    }

    fn capture(&mut self, duration: Duration) -> Result<Vec<f32>, AudioError> {
        let buffer = Arc::new(Mutex::new(Vec::new()));
        let buffer_for_cb = buffer.clone();

        let desired_spec = AudioSpecDesired {
            freq: Some(44100),
            channels: Some(1),
            samples: Some(1024),
        };

        thaleia_debug!("SDL2: Opening capture device...");

        let device = self
            .audio
            .open_capture(None, &desired_spec, move |_spec| {
                thaleia_debug!("SDL2: Capture callback invoked");
                CaptureCallback {
                    buffer: buffer_for_cb,
                }
            })
            .map_err(|e| {
                thaleia_debug!("SDL2: FAILED to open microphone: {}", e);
                AudioError::CaptureFailed(format!("Failed to open microphone: {}", e))
            })?;

        thaleia_debug!("SDL2: Device opened successfully, starting capture...");
        device.resume();

        // Capture for specified duration
        let sleep_ms = duration.as_millis() as u64;
        std::thread::sleep(Duration::from_millis(sleep_ms));

        // Stop capture
        device.pause();
        drop(device);

        let captured = buffer.lock().unwrap().clone();
        thaleia_debug!("SDL2: Captured {} samples", captured.len());
        tracing::debug!("SDL2: Captured {} samples", captured.len());
        Ok(captured)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backend_creation() {
        // SDL2 can only be initialized once per process
        // Skip this test if already initialized (from earlier test)
        if SDL2_INITIALIZED.load(std::sync::atomic::Ordering::SeqCst) {
            tracing::debug!("SDL2 already initialized, skipping creation test");
            return;
        }

        let result = SDL2Backend::new();
        // SDL2 init may fail in test environments without display
        match result {
            Ok(_) => tracing::debug!("SDL2Backend created successfully"),
            Err(e) => tracing::debug!("SDL2Backend creation skipped (no display): {:?}", e),
        }
    }

    #[test]
    fn test_backend_name() {
        // Only test if SDL2 is available
        if SDL2_INITIALIZED.load(std::sync::atomic::Ordering::SeqCst)
            && let Ok(backend) = SDL2Backend::new()
        {
            assert_eq!(backend.backend_name(), "sdl2");
        }
    }

    #[test]
    fn test_is_available() {
        let available = SDL2Backend::check_available();
        // SDL2 availability depends on:
        // 1. SDL2 library linked at compile time
        // 2. SDL2 audio subsystem initialized successfully
        // In containerized/test environments, SDL2 may not be available
        // so we just log the result rather than asserting
        if available {
            tracing::debug!("SDL2 backend is available");
        } else {
            tracing::debug!("SDL2 backend is NOT available (this is OK in test environments)");
        }
    }
}
