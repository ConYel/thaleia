//! Kokoro TTS Engine Implementation
//!
//! Uses the Kokoro-82M model for high-quality text-to-speech synthesis.
//! Auto-downloads models on first use (~310MB for model, ~27MB for voices).
//!
//! ## Features
//!
//! - CPU-optimized inference
//! - Multiple voice presets (26+ voices)
//! - English and other language support
//! - Streaming synthesis support
//! - Download progress tracking
//!
//! ## Performance
//!
//! - 0.5-2s time-to-first-audio
//! - 167.9x realtime synthesis speed (HiFi-GAN vocoder)
//! - 82M parameter model (tiny compared to 7B+ LLMs)
//!
//! ## Caching
//!
//! Model files are cached in `~/.cache/k/` (shared with kokoro-tiny).
//!
//! ## References
//!
//! - Kokoro Model: https://github.com/hexgrad/kokoro
//! - Kokoro-Tiny Rust: https://github.com/8b-is/kokoro-tiny

use crate::error::{Error, Result};
use crate::tts::{Voice, VoiceId};

#[cfg(feature = "kokoro")]
use kokoro_tiny::{FileType, Progress, TtsEngine as KokoroEngine};

/// Kokoro-based TTS Engine
///
/// This engine uses the Kokoro-82M model for high-quality, CPU-optimized
/// text-to-speech synthesis.
pub struct KokoroTtsEngine {
    inner: KokoroEngine,
}

impl std::fmt::Debug for KokoroTtsEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KokoroTtsEngine").finish()
    }
}

/// Audio output from synthesis
#[cfg(feature = "kokoro")]
#[allow(dead_code)]
pub type AudioSamples = Vec<f32>;

#[cfg(feature = "kokoro")]
#[allow(dead_code)]
impl KokoroTtsEngine {
    /// Create a new Kokoro TTS engine
    ///
    /// This will download the model (~310MB) and voice files (~27MB) on first run.
    /// Files are cached in `~/.cache/k/`.
    pub async fn new() -> Result<Self> {
        use std::io::Write;

        println!("🎭 Initializing Thaleia voice...");

        // Get cache directory
        let cache_dir = std::env::var("HOME")
            .map(|h| std::path::PathBuf::from(h).join(".cache").join("k"))
            .unwrap_or_else(|_| std::path::PathBuf::from(".cache/k"));

        let model_path = cache_dir.join("0.onnx");
        let voices_path = cache_dir.join("0.bin");

        // Total expected download size (model ~310MB + voices ~27MB)
        const TOTAL_SIZE: u64 = 337_000_000;

        // Progress bar state - use Arc<Mutex> for shared mutable state
        let last_percent = std::sync::Arc::new(std::sync::Mutex::new(0u32));

        let last_percent_clone = last_percent.clone();
        let on_progress = move |progress: Progress| {
            let percent = (progress.bytes_downloaded as f64 / TOTAL_SIZE as f64 * 100.0) as u32;

            // Only update when percentage changes
            let mut last = last_percent_clone.lock().unwrap();
            if percent != *last {
                *last = percent;

                let downloaded_mb = progress.bytes_downloaded as f64 / 1_000_000.0;
                let total_mb = TOTAL_SIZE as f64 / 1_000_000.0;
                let file_name = match progress.current_file {
                    FileType::Model => "model",
                    FileType::Voices => "voices",
                };

                // Draw progress bar
                let bar_width = 40;
                let filled = (percent as usize * bar_width / 100).min(bar_width);
                let empty = bar_width - filled;

                print!(
                    "\r🎭 Downloading {}: [{}{}] {}% ({:.1}MB / {:.1}MB)   ",
                    file_name,
                    "█".repeat(filled),
                    "░".repeat(empty),
                    percent,
                    downloaded_mb,
                    total_mb
                );
                std::io::stdout().flush().ok();
            }
        };

        // Initialize Kokoro engine with progress tracking
        let inner = KokoroEngine::with_progress(
            model_path.to_str().unwrap_or("0.onnx"),
            voices_path.to_str().unwrap_or("0.bin"),
            on_progress,
        )
        .await
        .map_err(|e| Error::TtsSynthesis(format!("Failed to initialize Kokoro: {}", e)))?;

        // Clear progress line
        println!();
        println!("   ✅ Voice model ready!");

        Ok(Self { inner })
    }

    /// Synthesize text to audio samples
    ///
    /// Returns raw PCM audio samples (f32, mono, 24000 Hz).
    #[allow(dead_code)]
    pub fn synthesize_text(&mut self, text: &str, voice: Option<&str>) -> Result<AudioSamples> {
        // Apply pronunciation fixes for words that espeak-ng mispronounces
        let text = Self::fix_pronunciations(text);
        self.inner
            .synthesize(&text, voice, None, None)
            .map_err(|e| Error::TtsSynthesis(format!("Synthesis failed: {}", e)))
    }

    /// Fix pronunciations for words that espeak-ng mispronounces
    fn fix_pronunciations(text: &str) -> String {
        // Map of words that need pronunciation fixes
        // Greek Θάλεια (Thalia) = "Thal-yah"
        let replacements = [
            // (original, phonetic_replacement)
            ("Thaleia", "Thal-yah"),
            ("Thaleia's", "Thal-yah's"),
            ("thaleia", "Thal-yah"),
            ("thaleia's", "Thal-yah's"),
        ];

        let mut result = text.to_string();
        for (original, replacement) in &replacements {
            // Only replace whole words
            result = result.replace(&format!(" {} ", original), &format!(" {} ", replacement));
            result = result.replace(&format!("{},", original), &format!("{},", replacement));
            result = result.replace(&format!("{}.", original), &format!("{}.", replacement));
            result = result.replace(&format!("{}!", original), &format!("{}!", replacement));
            result = result.replace(&format!("{}?", original), &format!("{}?", replacement));
            result = result.replace(&format!("{}'", original), &format!("{}'", replacement));
            result = result.replace(&format!("'{}", original), &format!("'{}", replacement));
            result = result.replace(
                &format!("\"{}\"", original),
                &format!("\"{}\"", replacement),
            );
        }
        result
    }

    /// List available voices
    ///
    /// Note: Kokoro uses voice packs. This returns the built-in voice IDs.
    #[allow(dead_code)]
    pub fn list_voices(&self) -> Vec<Voice> {
        vec![
            // American Female
            Voice::new(VoiceId::new("af_sarah"), "Sarah", "en-US"),
            Voice::new(VoiceId::new("af_sky"), "Sky", "en-US"),
            Voice::new(VoiceId::new("af_bella"), "Bella", "en-US"),
            Voice::new(VoiceId::new("af_nicole"), "Nicole", "en-US"),
            Voice::new(VoiceId::new("af_nicole_low"), "Nicole (Low)", "en-US"),
            // American Male
            Voice::new(VoiceId::new("am_adam"), "Adam", "en-US"),
            Voice::new(VoiceId::new("am_michael"), "Michael", "en-US"),
            // British Female
            Voice::new(VoiceId::new("bf_emma"), "Emma", "en-GB"),
            Voice::new(VoiceId::new("bf_isabella"), "Isabella", "en-GB"),
            // British Male
            Voice::new(VoiceId::new("bm_george"), "George", "en-GB"),
            Voice::new(VoiceId::new("bm_lewis"), "Lewis", "en-GB"),
            // Other languages
            Voice::new(VoiceId::new("ef_dora"), "Dora", "es"),
            Voice::new(VoiceId::new("ef_giulia"), "Giulia", "it"),
            Voice::new(VoiceId::new("jf_ai"), "Ai", "ja"),
            Voice::new(VoiceId::new("zf_xiaoni"), "Xiaoni", "zh"),
        ]
    }

    /// Get the default voice
    #[allow(dead_code)]
    pub fn default_voice(&self) -> VoiceId {
        VoiceId::new("af_sky")
    }
}

// =============================================================================
// Stubs for when Kokoro is not enabled
// =============================================================================

#[cfg(not(feature = "kokoro"))]
use crate::error::Result;
#[cfg(not(feature = "kokoro"))]
use crate::tts::Voice;

/// Kokoro TTS Engine (stub when feature not enabled)
#[cfg(not(feature = "kokoro"))]
#[derive(Debug)]
pub struct KokoroTtsEngine;

#[cfg(not(feature = "kokoro"))]
impl KokoroTtsEngine {
    /// Create a new Kokoro TTS engine
    ///
    /// # Panics
    ///
    /// Always panics when the `kokoro` feature is not enabled.
    pub async fn new() -> Result<Self> {
        Err(Error::TtsSynthesis(
            "Kokoro support not enabled. Build with --features kokoro".to_string(),
        ))
    }
}

#[cfg(test)]
#[cfg(feature = "kokoro")]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore = "Requires OpenSSL dev libraries and model download"]
    async fn test_kokoro_synthesis() {
        let mut engine = KokoroTtsEngine::new().await.unwrap();
        let audio = engine
            .synthesize_text("Hello from Thaleia!", Some("af_sarah"))
            .unwrap();
        assert!(!audio.is_empty());
        assert!(audio.iter().all(|s| (-1.0..=1.0).contains(s)));
    }

    // Note: test_voice_list is tested via the public API
    // which returns a static list regardless of engine state
}
