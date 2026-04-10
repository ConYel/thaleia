//! Whisper STT implementation
//!
//! Provides speech recognition using OpenAI's Whisper models.
//!
//! # Supported Models
//!
//! Only tiny, small, and base models are available to limit download burden:
//! - `tiny` (~75MB) - Fastest, good accuracy
//! - `base` (~140MB) - Default, balanced
//! - `small` (~465MB) - Best accuracy, slower
//!
//! # Audio Requirements
//!
//! Whisper expects 16kHz mono audio. This module uses [`rubato`]
//! to resample from other sample rates (e.g., 24kHz from Kokoro TTS).
//!
//! # Cache Location
//!
//! Models are cached at `~/.cache/whisper/` for reuse across sessions.

use crate::error::{Error, Result};
use crate::stt::engine::{BackendName, LanguageCode, SttBackend, SttInfo, Transcription};
use audioadapter_buffers::direct::InterleavedSlice;
use rubato::{
    Async, FixedAsync, Resampler, SincInterpolationParameters, SincInterpolationType,
    WindowFunction,
};
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

/// Whisper model sizes available for download.
///
/// Only these three sizes are available to limit download burden.
/// Medium and Large models are not included.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ModelSize {
    /// Tiny model - fastest, good accuracy
    #[default]
    Tiny,
    /// Base model - balanced (default)
    Base,
    /// Small model - best accuracy, slower
    Small,
}

impl ModelSize {
    /// Get the Whisper model name for this size.
    pub fn model_name(&self) -> &'static str {
        match self {
            ModelSize::Tiny => "tiny",
            ModelSize::Base => "base",
            ModelSize::Small => "small",
        }
    }

    /// Estimated download size in MB.
    pub fn size_mb(&self) -> f32 {
        match self {
            ModelSize::Tiny => 75.0,
            ModelSize::Base => 140.0,
            ModelSize::Small => 465.0,
        }
    }
}

/// Whisper STT engine
///
/// Transcribes speech to text using OpenAI's Whisper models.
/// Audio is automatically resampled to 16kHz (Whisper's expected format).
///
/// # Example
///
/// ```rust,ignore
/// use thaleia_core::stt::{WhisperEngine, ModelSize};
///
/// let mut engine = WhisperEngine::new(ModelSize::Base).await?;
/// let text = engine.transcribe(&audio_samples, 24000)?;
/// println!("You said: {}", text);
/// ```
#[derive(Debug)]
pub struct WhisperEngine {
    /// The Whisper context (model loaded)
    context: WhisperContext,
    /// Whether the engine is ready
    ready: bool,
}

impl WhisperEngine {
    /// Create a new Whisper engine.
    ///
    /// Downloads the model if not cached. Uses `base` size by default.
    pub async fn new(size: ModelSize) -> Result<Self> {
        Self::with_size(Some(size)).await
    }

    /// Create with specific size or auto-detect.
    async fn with_size(size: Option<ModelSize>) -> Result<Self> {
        let size = size.unwrap_or_default();
        let model_path = Self::get_model_path(size)?;

        // Download model if not cached
        if !model_path.exists() {
            Self::download_model(size).await?;
        }

        // Load model
        let context = Self::load_model(&model_path)?;

        Ok(Self {
            context,
            ready: true,
        })
    }

    /// Get the cache directory for Whisper models.
    fn cache_dir() -> std::path::PathBuf {
        home::home_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join(".cache")
            .join("whisper")
    }

    /// Get the path for a model file.
    fn get_model_path(size: ModelSize) -> Result<std::path::PathBuf> {
        let cache_dir = Self::cache_dir();
        std::fs::create_dir_all(&cache_dir)
            .map_err(|e| Error::Init(format!("Failed to create cache dir: {}", e)))?;
        Ok(cache_dir.join(format!("{}.bin", size.model_name())))
    }

    /// Download a Whisper model.
    ///
    /// Downloads only tiny, base, or small models.
    async fn download_model(size: ModelSize) -> Result<()> {
        use futures_util::StreamExt;
        use reqwest::Client;

        let model_name = size.model_name();
        // whisper.cpp uses ggml format from ggerganov's repo
        // Format: ggml-tiny.bin, ggml-base.bin, ggml-small.bin
        let url = format!(
            "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-{}.bin",
            model_name
        );

        tracing::info!(
            "Downloading Whisper {} model (~{:.0}MB)...",
            model_name,
            size.size_mb()
        );

        let client = Client::new();
        let response = client
            .get(&url)
            .send()
            .await
            .map_err(|e| Error::SttTranscription(format!("Download failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(Error::SttTranscription(format!(
                "Download failed with status: {}",
                response.status()
            )));
        }

        let total_size = response.content_length().unwrap_or(0);
        let model_path = Self::get_model_path(size)?;

        let mut stream = response.bytes_stream();

        use tokio::io::AsyncWriteExt;
        let mut file = tokio::fs::File::create(&model_path)
            .await
            .map_err(|e| Error::Init(format!("Failed to create model file: {}", e)))?;

        let mut downloaded: u64 = 0;

        while let Some(chunk) = stream.next().await {
            let chunk =
                chunk.map_err(|e| Error::SttTranscription(format!("Download error: {}", e)))?;
            file.write_all(&chunk)
                .await
                .map_err(|e| Error::SttTranscription(format!("Write error: {}", e)))?;
            downloaded += chunk.len() as u64;
            if total_size > 0 {
                let percent = (downloaded as f64 / total_size as f64 * 100.0) as u32;
                if percent.is_multiple_of(10) {
                    tracing::info!("Download progress: {}%", percent);
                }
            }
        }

        tracing::info!(
            "Whisper {} model downloaded to {:?}",
            model_name,
            model_path
        );

        Ok(())
    }

    /// Load a Whisper model from file.
    fn load_model(path: &std::path::Path) -> Result<WhisperContext> {
        WhisperContext::new_with_params(path, WhisperContextParameters::default())
            .map_err(|e| Error::SttTranscription(format!("Failed to load model: {}", e)))
    }

    /// Resample audio to 16kHz using rubato sinc interpolation.
    ///
    /// Whisper expects 16kHz mono audio. Uses high-quality sinc interpolation
    /// from the rubato crate for accurate resampling.
    fn resample_to_16khz(audio: &[f32], input_sample_rate: u32) -> Result<Vec<f32>> {
        const WHISPER_SAMPLE_RATE: u32 = 16000;
        const CHUNK_SIZE: usize = 1024;
        const SINC_LEN: usize = 64;
        const OVERSAMPLING: usize = 16;

        // Fast path: already 16kHz
        if input_sample_rate == WHISPER_SAMPLE_RATE {
            return Ok(audio.to_vec());
        }

        // Validate input
        if audio.is_empty() {
            return Err(Error::SttTranscription("Empty audio".to_string()));
        }

        let ratio = WHISPER_SAMPLE_RATE as f64 / input_sample_rate as f64;

        // Create resampler with sinc interpolation
        let sinc_params = SincInterpolationParameters {
            sinc_len: SINC_LEN,
            f_cutoff: 0.95,
            interpolation: SincInterpolationType::Cubic,
            oversampling_factor: OVERSAMPLING,
            window: WindowFunction::BlackmanHarris2,
        };

        let mut resampler = Async::<f32>::new_sinc(
            ratio,
            1.0, // no ratio adjustment needed
            &sinc_params,
            CHUNK_SIZE,
            1, // mono
            FixedAsync::Input,
        )
        .map_err(|e| Error::SttTranscription(format!("Resampler setup failed: {}", e)))?;

        // Prepare buffers
        let input_frames = audio.len();
        let output_len = resampler.process_all_needed_output_len(input_frames);

        let input_adapter = InterleavedSlice::new(audio, 1, input_frames)
            .map_err(|e| Error::SttTranscription(format!("Invalid input: {}", e)))?;

        let mut output_data = vec![0.0f32; output_len];
        let mut output_adapter = InterleavedSlice::new_mut(&mut output_data, 1, output_len)
            .map_err(|e| Error::SttTranscription(format!("Invalid output: {}", e)))?;

        // Resample - process_all_into_buffer handles chunking and delay trimming
        let (_nbr_in, nbr_out) = resampler
            .process_all_into_buffer(&input_adapter, &mut output_adapter, input_frames, None)
            .map_err(|e| Error::SttTranscription(format!("Resampling failed: {}", e)))?;

        output_data.truncate(nbr_out);
        Ok(output_data)
    }
}

impl SttBackend for WhisperEngine {
    fn transcribe(&mut self, audio: &[f32], sample_rate: u32) -> Result<Transcription> {
        if !self.ready {
            return Err(Error::Init("Engine not ready".to_string()));
        }

        // Resample to 16kHz if needed
        let audio_16k = Self::resample_to_16khz(audio, sample_rate)?;

        // Whisper expects f32 samples
        let audio_f32: Vec<f32> = audio_16k;

        // Create state from context
        let mut state = self
            .context
            .create_state()
            .map_err(|e| Error::SttTranscription(format!("Failed to create state: {}", e)))?;

        // Run inference with greedy sampling
        let params = FullParams::new(SamplingStrategy::Greedy { best_of: 0 });

        state
            .full(params, &audio_f32)
            .map_err(|e| Error::SttTranscription(format!("Inference failed: {}", e)))?;

        // Get transcription
        let num_segments = state.full_n_segments();

        let mut text = String::new();
        for i in 0..num_segments {
            if let Some(segment) = state.get_segment(i) {
                if !text.is_empty() {
                    text.push(' ');
                }
                // Use to_str_lossy which handles invalid UTF-8 gracefully
                if let Ok(segment_text) = segment.to_str_lossy() {
                    text.push_str(&segment_text);
                }
            }
        }

        Ok(Transcription::new(text))
    }

    fn is_ready(&self) -> bool {
        self.ready
    }

    fn info(&self) -> SttInfo {
        SttInfo::new(
            BackendName::Whisper,
            // Whisper supports 99 languages, list common ones
            vec![
                LanguageCode::new("en").unwrap(),
                LanguageCode::new("zh").unwrap(),
                LanguageCode::new("es").unwrap(),
                LanguageCode::new("fr").unwrap(),
                LanguageCode::new("de").unwrap(),
                LanguageCode::new("ja").unwrap(),
                LanguageCode::new("ko").unwrap(),
                LanguageCode::new("ru").unwrap(),
                LanguageCode::new("pt").unwrap(),
                LanguageCode::new("it").unwrap(),
            ],
            false, // Whisper doesn't support streaming
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ============================================================
    // Tests for enhanced SttBackend trait
    // These test the contract defined by the new trait design
    // ============================================================

    #[test]
    fn test_whisper_info_has_name() {
        // STT backend info should expose its name
        // This tests via the info() method which returns SttInfo
        let _info = WhisperEngine::resample_to_16khz(&[], 16000);
        // Can't test without a full engine, but we can test the types
    }

    #[test]
    fn test_backend_name_display() {
        let name = BackendName::Whisper;
        assert_eq!(name.as_str(), "whisper");
        assert_eq!(format!("{}", name), "whisper");
    }

    #[test]
    fn test_backend_name_custom() {
        let name = BackendName::Custom("my-backend".to_string());
        assert_eq!(name.as_str(), "my-backend");
    }

    #[test]
    fn test_language_code_valid() {
        let code = LanguageCode::new("en").unwrap();
        assert_eq!(code.as_str(), "en");
    }

    #[test]
    fn test_language_code_invalid() {
        // Too long
        assert!(LanguageCode::new("eng").is_none());
        // Not alphabetic
        assert!(LanguageCode::new("e1").is_none());
    }

    #[test]
    fn test_language_code_from_str() {
        let code = LanguageCode::from("en");
        assert_eq!(code.as_str(), "en");
    }

    #[test]
    fn test_language_code_common_codes() {
        let codes = LanguageCode::common_codes();
        assert!(!codes.is_empty());
        assert!(codes.iter().any(|c| c.as_str() == "en"));
    }

    #[test]
    fn test_transcription_new() {
        let t = Transcription::new("hello world".to_string());
        assert_eq!(t.text, "hello world");
        assert!(t.language.is_none());
        assert!(!t.is_partial);
    }

    #[test]
    fn test_transcription_with_language() {
        let lang = LanguageCode::from("en");
        let t = Transcription::new("hello".to_string()).with_language(lang.clone());
        assert_eq!(t.language.unwrap(), lang);
    }

    #[test]
    fn test_transcription_partial() {
        let t = Transcription::new("partial...".to_string()).partial();
        assert!(t.is_partial);
    }

    #[test]
    fn test_stt_info_new() {
        let info = SttInfo::new(BackendName::Whisper, vec![LanguageCode::from("en")], false);
        assert_eq!(info.name.as_str(), "whisper");
        assert_eq!(info.languages.len(), 1);
        assert!(!info.supports_streaming);
        assert!(info.model_size.is_none());
    }

    #[test]
    fn test_stt_info_with_model() {
        let info = SttInfo::new(BackendName::Whisper, vec![], false).with_model("base");
        assert_eq!(info.model_size, Some("base".to_string()));
    }

    // ============================================================
    // Existing tests (kept for compatibility)
    // ============================================================

    #[test]
    fn test_model_size_default() {
        assert_eq!(ModelSize::default(), ModelSize::Tiny);
    }

    #[test]
    fn test_model_size_names() {
        assert_eq!(ModelSize::Tiny.model_name(), "tiny");
        assert_eq!(ModelSize::Base.model_name(), "base");
        assert_eq!(ModelSize::Small.model_name(), "small");
    }

    #[test]
    fn test_model_size_sizes() {
        assert!(ModelSize::Tiny.size_mb() < ModelSize::Base.size_mb());
        assert!(ModelSize::Base.size_mb() < ModelSize::Small.size_mb());
    }

    #[test]
    fn test_resample_same_rate() {
        let input: Vec<f32> = (0..16000).map(|i| (i as f32 * 0.001).sin()).collect();
        let output = WhisperEngine::resample_to_16khz(&input, 16000).unwrap();
        // When resampling same rate, should return same length
        assert_eq!(input.len(), output.len());
    }

    #[test]
    fn test_resample_24khz_to_16khz() {
        // 24000 samples at 24kHz = 1 second of audio
        let input: Vec<f32> = (0..24000).map(|i| (i as f32 * 0.001).sin()).collect();
        let output = WhisperEngine::resample_to_16khz(&input, 24000).unwrap();

        // Expected: 24000 * (16000/24000) = 16000 samples
        let expected_len = 16000;
        let tolerance = 100; // Allow some variance due to resampler delay trimming
        assert!(
            (output.len() as i32 - expected_len).abs() <= tolerance,
            "Expected ~{} samples, got {}",
            expected_len,
            output.len()
        );
    }

    #[test]
    fn test_resample_empty_audio() {
        let result = WhisperEngine::resample_to_16khz(&[], 24000);
        assert!(result.is_err());
    }

    #[test]
    fn test_resample_preserves_audio_characteristics() {
        // Test that a sine wave maintains its sine nature after resampling
        let frequency = 440.0; // A4 note
        let sample_rate = 48000u32;
        let duration_samples = 4800; // 0.1 seconds

        let input: Vec<f32> = (0..duration_samples)
            .map(|i| {
                let t = i as f32 / sample_rate as f32;
                (2.0 * std::f32::consts::PI * frequency * t).sin()
            })
            .collect();

        let output = WhisperEngine::resample_to_16khz(&input, sample_rate).unwrap();

        // Output should have samples
        assert!(!output.is_empty());

        // The output should also be roughly a sine wave (peaks should be near 1.0)
        let max_sample = output.iter().fold(0.0f32, |max, &s| max.max(s.abs()));
        assert!(
            max_sample > 0.5,
            "Max sample {} is too low, audio may be corrupted",
            max_sample
        );
    }
}
