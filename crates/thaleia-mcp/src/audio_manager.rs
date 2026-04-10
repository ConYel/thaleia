//! Audio Thread Manager - bridges async MCP with sync audio resources
//!
//! Audio libraries (rodio, SDL2) are NOT Send+Sync. This module creates a
//! dedicated thread that owns audio resources and communicates via channels.

use std::thread;
use std::time::Duration;
use tokio::runtime::Builder as RuntimeBuilder;
use tokio::sync::{mpsc, oneshot};

use thaleia_core::audio::AudioSystem;
use thaleia_core::stt::SttBackend;
use thaleia_core::stt::{ModelSize, WhisperEngine};
use thaleia_core::tts::TtsEngine;
// Note: thaleia_debug is a macro from thaleia_core - use thaleia_core::thaleia_debug! when needed

/// TTS request - sent from async handler to sync audio thread
#[derive(Debug)]
pub struct TtsRequest {
    pub text: String,
    pub voice: Option<String>,
    pub result_tx: oneshot::Sender<Result<TtsResponse, String>>,
}

/// TTS response - returned from audio thread
#[derive(Debug)]
pub struct TtsResponse {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
}

/// STT request - sent from async handler to sync audio thread
#[derive(Debug)]
pub struct SttRequest {
    pub duration_secs: u64,
    pub source: Option<String>,
    pub result_tx: oneshot::Sender<Result<SttResponse, String>>,
}

/// STT response - returned from audio thread
#[derive(Debug)]
pub struct SttResponse {
    pub text: String,
    pub debug: Option<SttDebug>,
}

/// Debug info for STT transcription
#[derive(Debug)]
pub struct SttDebug {
    pub backend: String,
    pub samples: usize,
    pub sample_rate: u32,
    pub audio_available: bool,
}

/// Debug info for audio system
#[derive(Debug)]
pub struct AudioDebugInfo {
    pub debug_file: String,
}

/// Audio Thread Manager - owns the audio thread
///
/// This struct spawns a dedicated thread that owns audio resources.
/// It is Send and can be used from async contexts.
#[derive(Clone)]
pub struct AudioThreadManager {
    /// Channel for TTS requests
    tts_tx: mpsc::Sender<TtsRequest>,
    /// Channel for STT requests
    stt_tx: mpsc::Sender<SttRequest>,
}

impl AudioThreadManager {
    /// Create a new audio thread manager
    pub fn new() -> Result<Self, String> {
        let (tts_tx, tts_rx) = mpsc::channel(4);
        let (stt_tx, stt_rx) = mpsc::channel(4);

        // Spawn the audio thread - we don't need to track it
        let _ = thread::spawn(move || {
            Self::audio_thread_loop(tts_rx, stt_rx);
        });

        Ok(Self { tts_tx, stt_tx })
    }

    /// Audio thread main loop
    fn audio_thread_loop(
        mut tts_rx: mpsc::Receiver<TtsRequest>,
        mut stt_rx: mpsc::Receiver<SttRequest>,
    ) {
        // Initialize runtime for async operations
        let rt = RuntimeBuilder::new_current_thread()
            .enable_all()
            .build()
            .expect("Failed to create runtime");

        // Initialize engines
        let mut tts_engine = match rt.block_on(TtsEngine::new()) {
            Ok(e) => {
                tracing::info!("TTS engine initialized");
                Some(e)
            }
            Err(e) => {
                tracing::error!("Failed to initialize TTS: {}", e);
                None
            }
        };

        let mut stt_engine = match rt.block_on(WhisperEngine::new(ModelSize::Tiny)) {
            Ok(e) => {
                tracing::info!("STT engine initialized");
                Some(e)
            }
            Err(e) => {
                tracing::error!("Failed to initialize STT: {}", e);
                None
            }
        };

        let mut audio_system = AudioSystem::new();
        let audio_backend = audio_system.backend_name();
        let audio_available = audio_system.is_available();

        // Log environment for debugging
        let env_debug = format!(
            "Audio env - SDL_AUDIODRIVER: {}, PULSE_SERVER: {}, XDG_RUNTIME_DIR: {}",
            std::env::var("SDL_AUDIODRIVER").unwrap_or_default(),
            std::env::var("PULSE_SERVER").unwrap_or_default(),
            std::env::var("XDG_RUNTIME_DIR").unwrap_or_default()
        );

        tracing::info!(
            "Audio thread ready - backend: {}, available: {}",
            audio_backend,
            audio_available
        );
        thaleia_core::thaleia_debug!("   {}", env_debug);

        // Write debug file for external access
        let _ = std::fs::write(
            "/tmp/thaleia-audio-debug.log",
            format!(
                "backend={},available={}\n{}",
                audio_backend, audio_available, env_debug
            ),
        );

        loop {
            let tts_ready = tts_rx.try_recv();
            let stt_ready = stt_rx.try_recv();

            match (tts_ready, stt_ready) {
                (Ok(req), _) => {
                    // Handle TTS request
                    if let Some(ref mut engine) = tts_engine {
                        let voice = req.voice.as_deref().unwrap_or("af_sarah");
                        match engine.synthesize(&req.text, Some(voice)) {
                            Ok(samples) => {
                                // Play audio if available
                                if audio_system.is_available()
                                    && let Err(e) = audio_system.play(&samples, 24000)
                                {
                                    tracing::warn!("Playback warning: {}", e);
                                }
                                let _ = req.result_tx.send(Ok(TtsResponse {
                                    samples,
                                    sample_rate: 24000,
                                }));
                            }
                            Err(e) => {
                                let _ = req.result_tx.send(Err(format!("Synthesis failed: {}", e)));
                            }
                        }
                    } else {
                        let _ = req.result_tx.send(Err("TTS not initialized".to_string()));
                    }
                }
                (_, Ok(req)) => {
                    // Handle STT request
                    thaleia_core::thaleia_debug!(
                        "STT request received - source: {:?}, duration: {}s",
                        req.source,
                        req.duration_secs
                    );

                    if let Some(ref mut engine) = stt_engine {
                        // Set PULSE_SOURCE if explicitly provided
                        // Don't remove env var on auto-detect - let PulseAudio use default
                        if let Some(ref source) = req.source {
                            thaleia_core::thaleia_debug!("Setting PULSE_SOURCE to: {}", source);
                            // SAFETY: Setting env var for audio capture
                            unsafe { std::env::set_var("PULSE_SOURCE", source) };
                        } else {
                            // Auto-detect: use existing PULSE_SOURCE or PulseAudio default
                            let existing = std::env::var("PULSE_SOURCE")
                                .unwrap_or_else(|_| "default (PulseAudio)".to_string());
                            thaleia_core::thaleia_debug!("Using audio source: {}", existing);
                        };

                        // Debug: show current PULSE_SOURCE
                        let current_source =
                            std::env::var("PULSE_SOURCE").unwrap_or_else(|_| "not set".to_string());
                        thaleia_core::thaleia_debug!("Current PULSE_SOURCE: {}", current_source);

                        // Use existing audio_system (created at startup with SDL2 backend)
                        // This is the same approach as CLI uses
                        thaleia_core::thaleia_debug!(
                            "Using existing audio_system (backend: {})",
                            audio_system.backend_name()
                        );
                        thaleia_core::thaleia_debug!(
                            "Starting capture for {} seconds...",
                            req.duration_secs
                        );

                        let captured_samples = match audio_system
                            .capture(Duration::from_secs(req.duration_secs))
                        {
                            Ok(samples) => {
                                let sample_count = samples.len();
                                thaleia_core::thaleia_debug!("Captured {} samples", sample_count);

                                // Save captured audio to file for debugging
                                let debug_path = "/tmp/thaleia-debug-capture.wav";
                                let captured_audio = thaleia_core::capture::CapturedAudio {
                                    samples: samples.clone(),
                                    sample_rate: 44100,
                                    channels: 1,
                                };
                                if let Err(e) =
                                    captured_audio.save_wav(std::path::Path::new(debug_path))
                                {
                                    thaleia_core::thaleia_debug!(
                                        "Failed to save debug audio: {}",
                                        e
                                    );
                                } else {
                                    thaleia_core::thaleia_debug!(
                                        "Debug audio saved to: {}",
                                        debug_path
                                    );
                                }

                                samples
                            }
                            Err(e) => {
                                tracing::error!("Capture failed: {}", e);
                                let _ = req.result_tx.send(Err(format!("Capture failed: {}", e)));
                                continue;
                            }
                        };

                        // Check if we got meaningful audio
                        if captured_samples.is_empty() {
                            thaleia_core::thaleia_debug!(
                                "No audio captured - source may be silent"
                            );
                            let _ = req.result_tx.send(Ok(SttResponse {
                                text: "[BLANK_AUDIO]".to_string(),
                                debug: Some(SttDebug {
                                    backend: audio_backend.to_string(),
                                    samples: 0,
                                    sample_rate: 44100,
                                    audio_available: true,
                                }),
                            }));
                            continue;
                        }

                        // Transcribe
                        match engine.transcribe(&captured_samples, 44100) {
                            Ok(transcription) => {
                                thaleia_core::thaleia_debug!(
                                    "Transcription: {}",
                                    transcription.text
                                );
                                let _ = req.result_tx.send(Ok(SttResponse {
                                    text: transcription.text,
                                    debug: Some(SttDebug {
                                        backend: audio_backend.to_string(),
                                        samples: captured_samples.len(),
                                        sample_rate: 44100,
                                        audio_available: true,
                                    }),
                                }));
                            }
                            Err(e) => {
                                let _ = req
                                    .result_tx
                                    .send(Err(format!("Transcription failed: {}", e)));
                            }
                        }
                    } else {
                        let _ = req.result_tx.send(Err("STT not initialized".to_string()));
                    }
                }
                _ => {
                    // No requests, sleep briefly
                    thread::sleep(Duration::from_millis(10));
                }
            }
        }
    }

    /// Synthesize text to speech
    pub async fn synthesize(&self, text: &str, voice: Option<&str>) -> Result<TtsResponse, String> {
        let (result_tx, result_rx) = oneshot::channel();

        let request = TtsRequest {
            text: text.to_string(),
            voice: voice.map(|s| s.to_string()),
            result_tx,
        };

        self.tts_tx
            .send(request)
            .await
            .map_err(|e| format!("Failed to send TTS request: {}", e))?;

        result_rx
            .await
            .map_err(|e| format!("Failed to receive TTS response: {}", e))?
    }

    /// Transcribe audio from microphone
    pub async fn transcribe(
        &self,
        duration_secs: u64,
        source: Option<&str>,
    ) -> Result<SttResponse, String> {
        let (result_tx, result_rx) = oneshot::channel();

        let request = SttRequest {
            duration_secs,
            source: source.map(|s| s.to_string()),
            result_tx,
        };

        self.stt_tx
            .send(request)
            .await
            .map_err(|e| format!("Failed to send STT request: {}", e))?;

        result_rx
            .await
            .map_err(|e| format!("Failed to receive STT response: {}", e))?
    }

    /// List available audio sources
    pub async fn list_sources(&self) -> Result<Vec<String>, String> {
        // Use the existing list from capture module
        use thaleia_core::capture::list_audio_devices;

        match list_audio_devices() {
            Ok(devices) => Ok(devices.iter().map(|d| d.name.clone()).collect()),
            Err(e) => Err(format!("Failed to list sources: {}", e)),
        }
    }

    /// Get audio system debug info
    pub fn get_audio_debug(&self) -> AudioDebugInfo {
        // Read from debug file (written by audio thread)
        let debug_content =
            std::fs::read_to_string("/tmp/thaleia-audio-debug.log").unwrap_or_default();

        AudioDebugInfo {
            debug_file: debug_content,
        }
    }

    /// List available TTS voices
    pub async fn list_voices(&self) -> Vec<String> {
        vec![
            "af_sarah: Sarah (American Female)".to_string(),
            "af_sky: Sky (American Female)".to_string(),
            "af_bella: Bella (American Female)".to_string(),
            "af_nicole: Nicole (American Female)".to_string(),
            "am_adam: Adam (American Male)".to_string(),
            "am_michael: Michael (American Male)".to_string(),
            "bf_emma: Emma (British Female)".to_string(),
            "bf_isabella: Isabella (British Female)".to_string(),
            "bm_george: George (British Male)".to_string(),
            "bm_lewis: Lewis (British Male)".to_string(),
        ]
    }
}

impl Default for AudioThreadManager {
    fn default() -> Self {
        Self::new().expect("Failed to create AudioThreadManager")
    }
}
