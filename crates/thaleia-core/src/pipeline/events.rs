//! Dialogue events that trigger state transitions
//!
//! Events represent user actions or system responses that cause
//! the dialogue state machine to transition between states.

/// Events that can occur during dialogue
///
/// These events drive the state machine transitions.
/// See the module documentation for the state diagram.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogueEvent {
    /// Wake word detected - start listening
    WakeWordDetected,
    /// Push-to-talk activated - start listening
    PushToTalkStart,
    /// Speech detected by VAD - process input
    SpeechDetected,
    /// No speech for timeout period - go back to idle
    SilenceTimeout,
    /// STT transcription complete, ready for LLM processing
    TranscriptionComplete,
    /// LLM response ready - start speaking
    ResponseReady,
    /// TTS playback finished - go back to idle
    SpeechFinished,
    /// Stop button pressed or system reset - go to idle
    Stop,
    /// Error occurred during processing
    Error,
}

impl DialogueEvent {
    /// Check if this event can trigger a transition from the given state
    pub fn is_valid_from(&self, state: super::state::DialogueState) -> bool {
        use super::state::DialogueState::*;

        match (state, self) {
            // From Idle: can start listening via wake word or PTT
            (Idle, DialogueEvent::WakeWordDetected) => true,
            (Idle, DialogueEvent::PushToTalkStart) => true,

            // From Listening: speech detected or silence timeout
            (Listening, DialogueEvent::SpeechDetected) => true,
            (Listening, DialogueEvent::SilenceTimeout) => true,
            (Listening, DialogueEvent::Stop) => true,

            // From Processing: response ready or error
            (Processing, DialogueEvent::ResponseReady) => true,
            (Processing, DialogueEvent::Error) => true,
            (Processing, DialogueEvent::Stop) => true,

            // From Speaking: speech finished or interrupted (SpeechDetected)
            (Speaking, DialogueEvent::SpeechFinished) => true,
            (Speaking, DialogueEvent::SpeechDetected) => true, // Interruption!
            (Speaking, DialogueEvent::Stop) => true,

            // Error is valid from any state
            (_, DialogueEvent::Error) => true,

            // Stop is valid from any state
            (_, DialogueEvent::Stop) => true,

            // All other transitions are invalid
            _ => false,
        }
    }
}

impl std::fmt::Display for DialogueEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DialogueEvent::WakeWordDetected => write!(f, "WakeWordDetected"),
            DialogueEvent::PushToTalkStart => write!(f, "PushToTalkStart"),
            DialogueEvent::SpeechDetected => write!(f, "SpeechDetected"),
            DialogueEvent::SilenceTimeout => write!(f, "SilenceTimeout"),
            DialogueEvent::TranscriptionComplete => write!(f, "TranscriptionComplete"),
            DialogueEvent::ResponseReady => write!(f, "ResponseReady"),
            DialogueEvent::SpeechFinished => write!(f, "SpeechFinished"),
            DialogueEvent::Stop => write!(f, "Stop"),
            DialogueEvent::Error => write!(f, "Error"),
        }
    }
}

impl std::error::Error for DialogueEvent {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_wake_word_from_idle() {
        use super::super::state::DialogueState::*;
        assert!(DialogueEvent::WakeWordDetected.is_valid_from(Idle));
        assert!(DialogueEvent::PushToTalkStart.is_valid_from(Idle));
    }

    #[test]
    fn test_valid_from_listening() {
        use super::super::state::DialogueState::*;
        assert!(DialogueEvent::SpeechDetected.is_valid_from(Listening));
        assert!(DialogueEvent::SilenceTimeout.is_valid_from(Listening));
        assert!(DialogueEvent::Stop.is_valid_from(Listening));
    }

    #[test]
    fn test_valid_from_processing() {
        use super::super::state::DialogueState::*;
        assert!(DialogueEvent::ResponseReady.is_valid_from(Processing));
        assert!(DialogueEvent::Error.is_valid_from(Processing));
        assert!(DialogueEvent::Stop.is_valid_from(Processing));
    }

    #[test]
    fn test_valid_from_speaking() {
        use super::super::state::DialogueState::*;
        assert!(DialogueEvent::SpeechFinished.is_valid_from(Speaking));
        assert!(DialogueEvent::SpeechDetected.is_valid_from(Speaking)); // interruption!
        assert!(DialogueEvent::Stop.is_valid_from(Speaking));
    }

    #[test]
    fn test_invalid_transitions() {
        use super::super::state::DialogueState::*;

        // Can't go directly from Idle to Processing
        assert!(!DialogueEvent::SpeechDetected.is_valid_from(Idle));

        // Can't go directly from Idle to Speaking
        assert!(!DialogueEvent::ResponseReady.is_valid_from(Idle));

        // Can't go from Listening directly to Speaking
        assert!(!DialogueEvent::ResponseReady.is_valid_from(Listening));

        // Can't go from Processing back to Listening
        assert!(!DialogueEvent::SilenceTimeout.is_valid_from(Processing));
    }

    #[test]
    fn test_error_and_stop_always_valid() {
        use super::super::state::DialogueState::*;

        for state in [Idle, Listening, Processing, Speaking] {
            assert!(DialogueEvent::Error.is_valid_from(state));
            assert!(DialogueEvent::Stop.is_valid_from(state));
        }
    }

    #[test]
    fn test_display() {
        assert_eq!(
            DialogueEvent::WakeWordDetected.to_string(),
            "WakeWordDetected"
        );
        assert_eq!(DialogueEvent::SpeechFinished.to_string(), "SpeechFinished");
    }
}
