//! Voice Pipeline Integration
//!
//! Integrates all voice AI components:
//! - Wake Word Detection
//! - Voice Activity Detection (VAD)
//! - Speech-to-Text (STT)
//! - Text-to-Speech (TTS)
//! - Dialogue Manager (state machine)
//!
//! # Architecture
//!
//! The pipeline orchestrates the flow from audio input to audio output:
//! ```text
//! Audio Input → WakeWord → VAD → STT → LLM → TTS → Audio Output
//!                              ↑                    ↓
//!                         DialogueManager ← State Machine
//! ```
//!
//! # Usage
//!
//! ```rust,ignore
//! use thaleia_core::integration::{VoicePipeline, PipelineConfig};
//!
//! let config = PipelineConfig::default();
//! let mut pipeline = VoicePipeline::new(config)?;
//!
//! // Run the pipeline (typically in an async loop)
//! while let Some(event) = pipeline.process_audio_chunk(&audio).await? {
//!     match event {
//!         PipelineEvent::WakeWordDetected => { ... }
//!         PipelineEvent::SpeechDetected => { ... }
//!         PipelineEvent::ResponseReady(text) => { ... }
//!         PipelineEvent::SpeechFinished => { ... }
//!     }
//! }
//! ```

pub mod config;
pub mod events;
pub mod pipeline;

pub use config::PipelineConfig;
pub use events::PipelineEvent;
pub use pipeline::VoicePipeline;

#[cfg(test)]
mod tests {
    use super::*;

    /// Test default pipeline config
    #[test]
    fn test_default_config() {
        let config = PipelineConfig::default();
        assert!(config.enable_wake_word);
        assert!(config.enable_vad);
    }

    /// Test pipeline creation with defaults
    #[test]
    fn test_pipeline_creation() {
        let config = PipelineConfig::default();
        let pipeline = VoicePipeline::new(config);
        assert!(pipeline.is_ok());
    }

    /// Test pipeline initial state
    #[test]
    fn test_pipeline_initial_state() {
        let config = PipelineConfig::default();
        let pipeline = VoicePipeline::new(config).unwrap();

        use crate::pipeline::DialogueState;
        assert_eq!(pipeline.state(), DialogueState::Idle);
    }

    /// Test config with wake word disabled
    #[test]
    fn test_config_wake_word_disabled() {
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

    /// Test config with vad disabled (push-to-talk mode)
    #[test]
    fn test_config_vad_disabled() {
        let config = PipelineConfig {
            enable_wake_word: true,
            enable_vad: false,
            ..Default::default()
        };

        let pipeline = VoicePipeline::new(config).unwrap();

        // Should have wake word but no VAD
        assert!(pipeline.wake_word().is_some());
        assert!(pipeline.vad().is_none());

        // Initial state is Idle, can't interrupt
        assert!(!pipeline.can_interrupt());
    }
}
