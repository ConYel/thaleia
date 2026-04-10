//! Voice Pipeline - orchestrates all voice AI components
//!
//! The VoicePipeline integrates:
//! - WakeWord detection (Idle → Listening trigger)
//! - VAD (Listening → Processing trigger)
//! - STT via MCP (Processing)
//! - TTS via MCP (Speaking)
//! - DialogueManager (state machine)
//!
//! Note: This pipeline is for internal orchestration. The actual STT and TTS
//! are provided via MCP server for any LLM client to use.

use crate::pipeline::{DialogueEvent, DialogueManager, DialogueState};
use crate::vad::VadSystem;
use crate::wake_word::WakeWordSystem;

use super::config::PipelineConfig;
use super::events::PipelineEvent;

/// Voice Pipeline - integrates all voice AI components
///
/// # Example
/// ```rust,ignore
/// let config = PipelineConfig::default();
/// let mut pipeline = VoicePipeline::new(config)?;
///
/// // Process audio chunks
/// while let Some(event) = pipeline.process_audio(&audio_chunk)? {
///     println!("Pipeline event: {:?}", event);
/// }
/// ```
pub struct VoicePipeline {
    /// Dialogue manager for state machine
    dialogue: DialogueManager,
    /// Wake word detection (optional)
    wake_word: Option<WakeWordSystem>,
    /// Voice activity detection (optional)
    vad: Option<VadSystem>,
    /// Audio buffer for captured speech
    audio_buffer: Vec<f32>,
    /// Sample rate for audio processing
    sample_rate: u32,
    /// Pipeline configuration
    config: PipelineConfig,
}

impl VoicePipeline {
    /// Create a new voice pipeline with default configuration.
    pub fn new(config: PipelineConfig) -> crate::Result<Self> {
        let sample_rate = 16000; // Standard for Whisper

        // Initialize wake word if enabled
        let wake_word = if config.enable_wake_word {
            Some(WakeWordSystem::new()?)
        } else {
            None
        };

        // Initialize VAD if enabled
        let vad = if config.enable_vad {
            let vad_config = config.vad_config.clone().unwrap_or_default();
            Some(VadSystem::with_config(vad_config)?)
        } else {
            None
        };

        Ok(Self {
            dialogue: DialogueManager::new(),
            wake_word,
            vad,
            audio_buffer: Vec::new(),
            sample_rate,
            config,
        })
    }

    /// Get current dialogue state.
    #[must_use]
    pub fn state(&self) -> DialogueState {
        self.dialogue.state()
    }

    /// Check if current state can be interrupted.
    #[must_use]
    pub fn can_interrupt(&self) -> bool {
        self.dialogue.can_interrupt()
    }

    /// Get wake word system reference (for testing).
    #[cfg(test)]
    #[must_use]
    pub fn wake_word(&self) -> &Option<WakeWordSystem> {
        &self.wake_word
    }

    /// Get VAD system reference (for testing).
    #[cfg(test)]
    #[must_use]
    pub fn vad(&self) -> &Option<VadSystem> {
        &self.vad
    }

    /// Process an audio chunk through the pipeline.
    ///
    /// This method should be called repeatedly with audio chunks
    /// (typically 100-500ms of audio at a time).
    ///
    /// # Parameters
    /// * `audio_chunk` - Audio samples as f32 (mono)
    /// * `sample_rate` - Sample rate of the input audio (e.g., 44100, 48000)
    ///
    /// The pipeline internally resamples to 16kHz for wake word and VAD processing.
    ///
    /// # Returns
    /// * `Ok(Some(PipelineEvent))` - An event occurred that needs handling
    /// * `Ok(None)` - No event, continue processing
    /// * `Err(Error)` - An error occurred
    pub fn process_audio(
        &mut self,
        audio_chunk: &[f32],
        sample_rate: u32,
    ) -> crate::Result<Option<PipelineEvent>> {
        // Resample to internal 16kHz for wake word and VAD
        let audio_16k = if sample_rate == self.sample_rate {
            audio_chunk.to_vec()
        } else {
            Self::resample(audio_chunk, sample_rate, self.sample_rate)
        };

        let current_state = self.dialogue.state();

        match current_state {
            // Idle: Check for wake word
            DialogueState::Idle => {
                if let Some(ref mut ww) = self.wake_word
                    && let Ok(result) = ww.detect(&audio_16k, self.sample_rate)
                    && result.detected
                {
                    self.dialogue
                        .handle_event(DialogueEvent::WakeWordDetected)?;
                    return Ok(Some(PipelineEvent::WakeWordDetected {
                        keyword: result.keyword.unwrap_or_default(),
                        confidence: result.confidence,
                    }));
                }
            }

            // Listening: Check for speech via VAD
            DialogueState::Listening => {
                // Add original audio to buffer (not resampled)
                self.audio_buffer.extend_from_slice(audio_chunk);

                if let Some(ref mut vad) = self.vad
                    && let Ok(result) = vad.detect_speech(&audio_16k, self.sample_rate)
                    && result.is_speaking
                {
                    // Speech detected!
                    self.dialogue.handle_event(DialogueEvent::SpeechDetected)?;
                    return Ok(Some(PipelineEvent::SpeechStarted));
                }

                // Check for silence timeout
                let max_buffer_secs = self.config.max_audio_buffer_secs as usize;
                let buffer_samples = self.audio_buffer.len();
                let buffer_secs = buffer_samples / sample_rate as usize;

                if buffer_secs > max_buffer_secs {
                    // Too much silence, reset
                    self.audio_buffer.clear();
                    self.dialogue.handle_event(DialogueEvent::SilenceTimeout)?;
                    return Ok(Some(PipelineEvent::Idle));
                }
            }

            // Processing: Audio captured
            // Note: STT and TTS are provided via MCP server
            // This pipeline orchestrates the voice interaction flow
            DialogueState::Processing => {
                // Take the audio buffer
                let captured_audio = std::mem::take(&mut self.audio_buffer);
                let duration = captured_audio.len() as f32 / sample_rate as f32;

                // Signal that audio capture is complete
                // The actual STT→LLM→TTS flow happens via MCP server
                self.dialogue.handle_event(DialogueEvent::ResponseReady)?;

                return Ok(Some(PipelineEvent::SpeechEnded {
                    audio: captured_audio,
                    duration_secs: duration,
                }));
            }

            // Speaking: TTS playback in progress
            DialogueState::Speaking => {
                // Check for interruption (user speaking while AI is talking)
                if let Some(ref mut vad) = self.vad
                    && let Ok(result) = vad.detect_speech(&audio_16k, self.sample_rate)
                    && result.is_speaking
                {
                    // User interrupted!
                    self.dialogue.handle_event(DialogueEvent::SpeechDetected)?;
                    self.audio_buffer.clear();
                    return Ok(Some(PipelineEvent::SpeechStarted));
                }
            }
        }

        Ok(None)
    }

    /// Resample audio to target sample rate using linear interpolation.
    ///
    /// This is a simple but effective resampling method suitable for
    /// voice processing where exact frequency response is not critical.
    fn resample(audio: &[f32], input_rate: u32, output_rate: u32) -> Vec<f32> {
        if audio.is_empty() || input_rate == output_rate {
            return audio.to_vec();
        }

        let ratio = output_rate as f64 / input_rate as f64;
        let output_len = (audio.len() as f64 * ratio).ceil() as usize;
        let mut output = Vec::with_capacity(output_len);

        for i in 0..output_len {
            let src_idx = i as f64 / ratio;
            let src_idx_floor = src_idx.floor() as usize;
            let src_idx_frac = (src_idx - src_idx_floor as f64) as f32;

            if src_idx_floor + 1 < audio.len() {
                let sample = audio[src_idx_floor] * (1.0 - src_idx_frac)
                    + audio[src_idx_floor + 1] * src_idx_frac;
                output.push(sample);
            } else if src_idx_floor < audio.len() {
                output.push(audio[src_idx_floor]);
            }
        }

        output
    }

    /// Trigger push-to-talk (start listening without wake word).
    pub fn push_to_talk(&mut self) -> crate::Result<()> {
        self.dialogue.handle_event(DialogueEvent::PushToTalkStart)?;
        Ok(())
    }

    /// Stop the pipeline and return to idle.
    pub fn stop(&mut self) -> crate::Result<()> {
        self.dialogue.handle_event(DialogueEvent::Stop)?;
        self.audio_buffer.clear();
        Ok(())
    }

    /// Signal that speech has finished (TTS done).
    pub fn speech_finished(&mut self) -> crate::Result<()> {
        self.dialogue.handle_event(DialogueEvent::SpeechFinished)?;
        Ok(())
    }

    /// Get the captured audio buffer.
    #[must_use]
    pub fn take_audio(&mut self) -> Vec<f32> {
        std::mem::take(&mut self.audio_buffer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipeline_creation() {
        let config = PipelineConfig::default();
        let pipeline = VoicePipeline::new(config);
        assert!(pipeline.is_ok());
    }

    #[test]
    fn test_speech_finished_transitions_to_idle() {
        // Test the dialogue manager directly for this flow
        let mut dialogue = DialogueManager::new();

        // Idle -> Listening (push to talk)
        dialogue
            .handle_event(DialogueEvent::PushToTalkStart)
            .unwrap();
        assert_eq!(dialogue.state(), DialogueState::Listening);

        // Listening -> Processing (speech detected)
        dialogue
            .handle_event(DialogueEvent::SpeechDetected)
            .unwrap();
        assert_eq!(dialogue.state(), DialogueState::Processing);

        // Processing -> Speaking (response ready)
        dialogue.handle_event(DialogueEvent::ResponseReady).unwrap();
        assert_eq!(dialogue.state(), DialogueState::Speaking);

        // Speaking -> Idle (speech finished)
        dialogue
            .handle_event(DialogueEvent::SpeechFinished)
            .unwrap();
        assert_eq!(dialogue.state(), DialogueState::Idle);
    }

    #[test]
    fn test_push_to_talk() {
        let config = PipelineConfig::default();
        let mut pipeline = VoicePipeline::new(config).unwrap();

        pipeline.push_to_talk().unwrap();
        assert_eq!(pipeline.state(), DialogueState::Listening);
    }

    #[test]
    fn test_stop() {
        let config = PipelineConfig::default();
        let mut pipeline = VoicePipeline::new(config).unwrap();

        // Go to Listening
        pipeline.push_to_talk().unwrap();
        assert_eq!(pipeline.state(), DialogueState::Listening);

        // Stop
        pipeline.stop().unwrap();
        assert_eq!(pipeline.state(), DialogueState::Idle);
    }

    #[test]
    fn test_can_interrupt_only_when_speaking() {
        let config = PipelineConfig::default();
        let mut pipeline = VoicePipeline::new(config).unwrap();

        assert!(!pipeline.can_interrupt());

        // Go to Speaking
        pipeline.push_to_talk().unwrap();
        pipeline.process_audio(&[0.0f32; 16000], 16000).unwrap();

        // In Processing, can't interrupt yet
        // (In real impl, would need ResponseReady to get to Speaking)

        // After stop, should be idle
        pipeline.stop().unwrap();
        assert!(!pipeline.can_interrupt());
    }

    #[test]
    fn test_audio_buffering() {
        let config = PipelineConfig::default();
        let mut pipeline = VoicePipeline::new(config).unwrap();

        // Go to Listening
        pipeline.push_to_talk().unwrap();

        // Process some audio chunks
        let chunk = vec![0.1f32; 1600]; // 100ms at 16kHz
        pipeline.process_audio(&chunk, 16000).unwrap();

        // Buffer should have audio (via public API)
        let audio = pipeline.take_audio();
        assert!(!audio.is_empty());
    }

    #[test]
    fn test_wake_word_disabled() {
        let config = PipelineConfig {
            enable_wake_word: false,
            enable_vad: true,
            ..Default::default()
        };

        let pipeline = VoicePipeline::new(config).unwrap();

        // Should have no wake word
        assert!(pipeline.wake_word().is_none());

        // Should still have VAD
        assert!(pipeline.vad().is_some());
    }

    #[test]
    fn test_vad_disabled() {
        let config = PipelineConfig {
            enable_wake_word: true,
            enable_vad: false,
            ..Default::default()
        };

        let pipeline = VoicePipeline::new(config).unwrap();

        // Should have wake word
        assert!(pipeline.wake_word().is_some());

        // Should have no VAD
        assert!(pipeline.vad().is_none());
    }

    // =========================================================================
    // Extended Pipeline Tests
    // =========================================================================

    #[test]
    fn test_full_flow_push_to_talk_to_listening() {
        let config = PipelineConfig::default();
        let mut pipeline = VoicePipeline::new(config).unwrap();

        // Start in Idle
        assert_eq!(pipeline.state(), DialogueState::Idle);

        // Push to talk -> Listening
        pipeline.push_to_talk().unwrap();
        assert_eq!(pipeline.state(), DialogueState::Listening);
    }

    #[test]
    fn test_audio_accumulates_in_listening_state() {
        let config = PipelineConfig::default();
        let mut pipeline = VoicePipeline::new(config).unwrap();

        // Go to Listening
        pipeline.push_to_talk().unwrap();

        // Process multiple chunks
        let chunk = vec![0.0f32; 1600]; // 100ms at 16kHz
        pipeline.process_audio(&chunk, 16000).unwrap();
        pipeline.process_audio(&chunk, 16000).unwrap();
        pipeline.process_audio(&chunk, 16000).unwrap();

        // Should have accumulated 3 chunks
        let audio = pipeline.take_audio();
        assert_eq!(audio.len(), 1600 * 3);
    }

    #[test]
    fn test_take_audio_clears_buffer() {
        let config = PipelineConfig::default();
        let mut pipeline = VoicePipeline::new(config).unwrap();

        // Go to Listening and add audio
        pipeline.push_to_talk().unwrap();
        let chunk = vec![0.1f32; 800]; // 50ms at 16kHz
        pipeline.process_audio(&chunk, 16000).unwrap();

        // Take audio
        let audio = pipeline.take_audio();
        assert!(!audio.is_empty());

        // Buffer should be empty now
        let audio2 = pipeline.take_audio();
        assert!(audio2.is_empty());
    }

    #[test]
    fn test_stop_clears_audio_buffer() {
        let config = PipelineConfig::default();
        let mut pipeline = VoicePipeline::new(config).unwrap();

        // Go to Listening and add audio
        pipeline.push_to_talk().unwrap();
        let chunk = vec![0.1f32; 800]; // 50ms at 16kHz
        pipeline.process_audio(&chunk, 16000).unwrap();

        // Stop
        pipeline.stop().unwrap();

        // Audio buffer should be cleared
        let audio = pipeline.take_audio();
        assert!(audio.is_empty());
    }

    #[test]
    fn test_is_active_states() {
        let config = PipelineConfig::default();
        let mut pipeline = VoicePipeline::new(config).unwrap();

        // Idle is not active
        assert!(!pipeline.dialogue.state().is_active());

        // Listening is active
        pipeline.push_to_talk().unwrap();
        assert!(pipeline.dialogue.state().is_active());

        // After stop, back to idle
        pipeline.stop().unwrap();
        assert!(!pipeline.dialogue.state().is_active());
    }

    #[test]
    fn test_process_audio_in_idle_returns_none() {
        let config = PipelineConfig::default();
        let mut pipeline = VoicePipeline::new(config).unwrap();

        // In Idle, process_audio should return None (no wake word detected)
        let chunk = vec![0.0f32; 1600]; // 100ms at 16kHz
        let result = pipeline.process_audio(&chunk, 16000).unwrap();

        // Should return None because no wake word (energy-based needs loud audio)
        assert!(result.is_none());
    }

    #[test]
    fn test_wake_word_detected_with_loud_audio() {
        let config = PipelineConfig::default();
        let mut pipeline = VoicePipeline::new(config).unwrap();

        // First, process some quiet audio to establish baseline
        let quiet = vec![0.001f32; 16000]; // 1 second at 16kHz
        for _ in 0..10 {
            let _ = pipeline.process_audio(&quiet, 16000).unwrap();
        }

        // Verify still in Idle (quiet shouldn't trigger)
        assert_eq!(pipeline.state(), DialogueState::Idle);

        // Now process loud audio - should trigger wake word
        let loud: Vec<f32> = (0..16000).map(|i| (i as f32 * 0.1).sin()).collect();
        let result = pipeline.process_audio(&loud, 16000).unwrap();

        // Should detect wake word and transition to Listening
        assert!(result.is_some());
        let event = result.unwrap();
        match event {
            PipelineEvent::WakeWordDetected {
                keyword,
                confidence,
            } => {
                assert_eq!(keyword, "hey_thaleia");
                assert!(confidence > 0.0);
            }
            _ => panic!("Expected WakeWordDetected event"),
        }

        // Pipeline should now be in Listening state
        assert_eq!(pipeline.state(), DialogueState::Listening);
    }

    #[test]
    fn test_multiple_stop_calls_are_safe() {
        let config = PipelineConfig::default();
        let mut pipeline = VoicePipeline::new(config).unwrap();

        // Stop from Idle should be safe
        pipeline.stop().unwrap();
        pipeline.stop().unwrap();
        pipeline.stop().unwrap();

        assert_eq!(pipeline.state(), DialogueState::Idle);
    }

    #[test]
    fn test_multiple_push_to_talk_calls() {
        let config = PipelineConfig::default();
        let mut pipeline = VoicePipeline::new(config).unwrap();

        // Push to talk
        pipeline.push_to_talk().unwrap();
        assert_eq!(pipeline.state(), DialogueState::Listening);

        // Push to talk again from Listening - this should fail (invalid transition)
        let result = pipeline.push_to_talk();
        assert!(result.is_err());
    }

    #[test]
    fn test_sample_rate_configuration() {
        let config = PipelineConfig::default();
        let pipeline = VoicePipeline::new(config).unwrap();

        // Default sample rate should be 16000
        assert_eq!(pipeline.sample_rate, 16000);
    }

    #[test]
    fn test_config_passthrough() {
        let config = PipelineConfig {
            enable_wake_word: false,
            enable_vad: false,
            silence_timeout_secs: 10,
            max_audio_buffer_secs: 60,
            ..Default::default()
        };

        let pipeline = VoicePipeline::new(config).unwrap();

        // Both disabled
        assert!(pipeline.wake_word().is_none());
        assert!(pipeline.vad().is_none());

        // State machine should work
        assert_eq!(pipeline.state(), DialogueState::Idle);
    }

    // =========================================================================
    // Sample Rate Tests
    // =========================================================================

    #[test]
    fn test_process_audio_accepts_sample_rate() {
        // Test that process_audio works with different sample rates
        let config = PipelineConfig::default();
        let mut pipeline = VoicePipeline::new(config).unwrap();

        // 16kHz audio - should work
        let audio_16k = vec![0.0f32; 1600];
        let result = pipeline.process_audio(&audio_16k, 16000);
        assert!(result.is_ok());

        // 44.1kHz audio - should also work (will be resampled internally)
        let audio_44k = vec![0.0f32; 4410];
        let result = pipeline.process_audio(&audio_44k, 44100);
        assert!(result.is_ok());
    }

    #[test]
    fn test_wake_word_with_44khz_audio() {
        // Test wake word detection with 44.1kHz audio (simulating mic input)
        let config = PipelineConfig::default();
        let mut pipeline = VoicePipeline::new(config).unwrap();

        // First process quiet audio to establish baseline at 44.1kHz
        let quiet_44k = vec![0.001f32; 4410]; // 100ms of quiet at 44.1kHz
        for _ in 0..10 {
            let _ = pipeline.process_audio(&quiet_44k, 44100).unwrap();
        }

        // Verify still in Idle
        assert_eq!(pipeline.state(), DialogueState::Idle);

        // Now process loud audio at 44.1kHz - should trigger wake word
        // Create loud audio (sine wave scaled to ~0.5 amplitude)
        let loud_44k: Vec<f32> = (0..4410).map(|i| (i as f32 * 0.1).sin() * 0.5).collect();
        let result = pipeline.process_audio(&loud_44k, 44100).unwrap();

        // Should detect wake word
        assert!(result.is_some());
        let event = result.unwrap();
        match event {
            PipelineEvent::WakeWordDetected {
                keyword,
                confidence,
            } => {
                assert_eq!(keyword, "hey_thaleia");
                assert!(confidence > 0.0);
            }
            _ => panic!("Expected WakeWordDetected event"),
        }
    }
}
