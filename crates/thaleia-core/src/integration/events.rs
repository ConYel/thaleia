//! Pipeline events
//!
//! Events emitted by the voice pipeline during processing.

/// Events emitted by the voice pipeline
///
/// These events indicate what's happening in the pipeline
/// and can be used to update UI, trigger actions, etc.
#[derive(Debug, Clone)]
pub enum PipelineEvent {
    /// Wake word was detected
    WakeWordDetected {
        /// The detected wake word keyword
        keyword: String,
        /// Detection confidence (0.0 - 1.0)
        confidence: f32,
    },
    /// Voice activity detected (user started speaking)
    SpeechStarted,
    /// Voice activity ended (user stopped speaking)
    SpeechEnded {
        /// Audio captured during speech
        audio: Vec<f32>,
        /// Duration in seconds
        duration_secs: f32,
    },
    /// Transcription complete
    TranscriptionComplete {
        /// Transcribed text
        text: String,
    },
    /// LLM response ready
    ResponseReady {
        /// Response text to speak
        text: String,
    },
    /// TTS playback started
    PlaybackStarted,
    /// TTS playback finished
    PlaybackFinished,
    /// Pipeline returned to idle
    Idle,
    /// Error occurred
    Error {
        /// Error message
        message: String,
    },
}

impl PipelineEvent {
    /// Check if this is a terminal event (ends the current interaction)
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            PipelineEvent::PlaybackFinished | PipelineEvent::Idle | PipelineEvent::Error { .. }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_terminal_events() {
        assert!(PipelineEvent::PlaybackFinished.is_terminal());
        assert!(PipelineEvent::Idle.is_terminal());
        assert!(
            PipelineEvent::Error {
                message: "test".to_string()
            }
            .is_terminal()
        );

        assert!(
            !PipelineEvent::WakeWordDetected {
                keyword: "hey".to_string(),
                confidence: 0.9
            }
            .is_terminal()
        );
        assert!(!PipelineEvent::SpeechStarted.is_terminal());
        assert!(
            !PipelineEvent::SpeechEnded {
                audio: vec![],
                duration_secs: 1.0
            }
            .is_terminal()
        );
    }
}
