//! Dialogue state definitions
//!
//! Defines the states in the dialogue state machine.

/// Dialogue state representing the current phase of interaction
///
/// # State Description
///
/// - `Idle`: Waiting for user input (wake word or push-to-talk)
/// - `Listening`: Actively listening for user speech (VAD active)
/// - `Processing`: Processing user input (STT + LLM)
/// - `Speaking`: Responding to user (TTS active)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DialogueState {
    /// Waiting for wake word or push-to-talk
    #[default]
    Idle,
    /// Actively listening for speech (VAD monitoring)
    Listening,
    /// Processing input (transcription + generation)
    Processing,
    /// Generating TTS response
    Speaking,
}

impl DialogueState {
    /// Check if this state can be interrupted by user speech
    ///
    /// Only the Speaking state can be interrupted - user can stop
    /// the AI from talking at any time.
    pub fn can_interrupt(&self) -> bool {
        matches!(self, DialogueState::Speaking)
    }

    /// Check if this state is active (not Idle)
    pub fn is_active(&self) -> bool {
        !matches!(self, DialogueState::Idle)
    }
}

impl std::fmt::Display for DialogueState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DialogueState::Idle => write!(f, "Idle"),
            DialogueState::Listening => write!(f, "Listening"),
            DialogueState::Processing => write!(f, "Processing"),
            DialogueState::Speaking => write!(f, "Speaking"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_is_idle() {
        assert_eq!(DialogueState::default(), DialogueState::Idle);
    }

    #[test]
    fn test_can_interrupt_only_speaking() {
        assert!(!DialogueState::Idle.can_interrupt());
        assert!(!DialogueState::Listening.can_interrupt());
        assert!(!DialogueState::Processing.can_interrupt());
        assert!(DialogueState::Speaking.can_interrupt());
    }

    #[test]
    fn test_is_active() {
        assert!(!DialogueState::Idle.is_active());
        assert!(DialogueState::Listening.is_active());
        assert!(DialogueState::Processing.is_active());
        assert!(DialogueState::Speaking.is_active());
    }

    #[test]
    fn test_display() {
        assert_eq!(DialogueState::Idle.to_string(), "Idle");
        assert_eq!(DialogueState::Listening.to_string(), "Listening");
        assert_eq!(DialogueState::Processing.to_string(), "Processing");
        assert_eq!(DialogueState::Speaking.to_string(), "Speaking");
    }
}
