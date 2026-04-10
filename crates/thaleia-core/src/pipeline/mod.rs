//! Pipeline module for Thaleia voice AI
//!
//! Provides dialogue management with state machine for natural conversations.
//!
//! # Architecture
//!
//! - [`DialogueState`] - Represents the current state of the dialogue
//! - [`DialogueEvent`] - Events that trigger state transitions
//! - [`DialogueManager`] - Manages dialogue flow and handles interruptions
//!
//! # State Machine
//!
//! ```text
//!       +---------+
//!       |  Idle   |
//!       +----+----+
//!            |
//!      Wake/PTT
//!            |
//!            v
//!    +-------+-------+
//!    |  Listening     |<--------+
//!    +-------+-------+         |
//!            |                 |
//!     Speech Detected          |
//!            |                 |
//!            v                 |
//!    +-------+-------+         |
//!    | Processing       |      |
//!    +-------+-------+         |
//!            |                 |
//!         Response             |
//!            |                 |
//!            v                 |
//!    +-------+-------+         |
//!    |   Speaking        |------+
//!    +-------------------+
//!            |
//!      Speech Detected (Interruption)
//!            |
//!            v
//!    +-------+-------+
//!    |  Listening     |
//!    +----------------+
//! ```
//!
//! # Usage
//!
//! ```rust,ignore
//! use thaleia_core::pipeline::{DialogueManager, DialogueEvent, DialogueState};
//!
//! let mut manager = DialogueManager::new();
//!
//! // Handle wake word or push-to-talk
//! manager.handle_event(DialogueEvent::WakeWordDetected)?;
//! assert_eq!(manager.state(), DialogueState::Listening);
//!
//! // VAD detects speech
//! manager.handle_event(DialogueEvent::SpeechDetected)?;
//! assert_eq!(manager.state(), DialogueState::Processing);
//!
//! // ... process and respond ...
//! manager.handle_event(DialogueEvent::ResponseReady)?;
//! assert_eq!(manager.state(), DialogueState::Speaking);
//!
//! // User interrupts
//! manager.handle_event(DialogueEvent::SpeechDetected)?;
//! assert_eq!(manager.state(), DialogueState::Listening);
//! ```

pub mod events;
pub mod manager;
pub mod state;

pub use events::DialogueEvent;
pub use manager::DialogueManager;
pub use state::DialogueState;

#[cfg(test)]
mod tests {
    use super::*;

    /// Test that DialogueManager starts in Idle state
    #[test]
    fn test_initial_state_is_idle() {
        let manager = DialogueManager::new();
        assert_eq!(manager.state(), DialogueState::Idle);
    }

    /// Test wake word transitions from Idle to Listening
    #[test]
    fn test_wake_word_transitions_to_listening() {
        let mut manager = DialogueManager::new();

        manager
            .handle_event(DialogueEvent::WakeWordDetected)
            .unwrap();

        assert_eq!(manager.state(), DialogueState::Listening);
    }

    /// Test push-to-talk transitions from Idle to Listening
    #[test]
    fn test_push_to_talk_transitions_to_listening() {
        let mut manager = DialogueManager::new();

        manager
            .handle_event(DialogueEvent::PushToTalkStart)
            .unwrap();

        assert_eq!(manager.state(), DialogueState::Listening);
    }

    /// Test speech detected transitions from Listening to Processing
    #[test]
    fn test_speech_detected_transitions_to_processing() {
        let mut manager = DialogueManager::new();

        // First, get to Listening state
        manager
            .handle_event(DialogueEvent::WakeWordDetected)
            .unwrap();
        assert_eq!(manager.state(), DialogueState::Listening);

        // Then detect speech
        manager.handle_event(DialogueEvent::SpeechDetected).unwrap();

        assert_eq!(manager.state(), DialogueState::Processing);
    }

    /// Test response ready transitions from Processing to Speaking
    #[test]
    fn test_response_ready_transitions_to_speaking() {
        let mut manager = DialogueManager::new();

        // Get to Processing state
        manager
            .handle_event(DialogueEvent::WakeWordDetected)
            .unwrap();
        manager.handle_event(DialogueEvent::SpeechDetected).unwrap();
        assert_eq!(manager.state(), DialogueState::Processing);

        // Response ready
        manager.handle_event(DialogueEvent::ResponseReady).unwrap();

        assert_eq!(manager.state(), DialogueState::Speaking);
    }

    /// Test speech finished transitions from Speaking to Idle
    #[test]
    fn test_speech_finished_transitions_to_idle() {
        let mut manager = DialogueManager::new();

        // Get to Speaking state
        manager
            .handle_event(DialogueEvent::WakeWordDetected)
            .unwrap();
        manager.handle_event(DialogueEvent::SpeechDetected).unwrap();
        manager.handle_event(DialogueEvent::ResponseReady).unwrap();
        assert_eq!(manager.state(), DialogueState::Speaking);

        // Speech finished
        manager.handle_event(DialogueEvent::SpeechFinished).unwrap();

        assert_eq!(manager.state(), DialogueState::Idle);
    }

    /// Test interruption during Speaking transitions to Listening
    #[test]
    fn test_interruption_during_speaking_transitions_to_listening() {
        let mut manager = DialogueManager::new();

        // Get to Speaking state
        manager
            .handle_event(DialogueEvent::WakeWordDetected)
            .unwrap();
        manager.handle_event(DialogueEvent::SpeechDetected).unwrap();
        manager.handle_event(DialogueEvent::ResponseReady).unwrap();
        assert_eq!(manager.state(), DialogueState::Speaking);

        // User interrupts (speech detected while speaking)
        manager.handle_event(DialogueEvent::SpeechDetected).unwrap();

        // Should transition back to Listening (not Processing - we interrupt mid-speech)
        assert_eq!(manager.state(), DialogueState::Listening);
    }

    /// Test silence timeout transitions from Listening to Idle
    #[test]
    fn test_silence_timeout_transitions_to_idle() {
        let mut manager = DialogueManager::new();

        // Get to Listening state
        manager
            .handle_event(DialogueEvent::WakeWordDetected)
            .unwrap();
        assert_eq!(manager.state(), DialogueState::Listening);

        // Silence timeout
        manager.handle_event(DialogueEvent::SilenceTimeout).unwrap();

        assert_eq!(manager.state(), DialogueState::Idle);
    }

    /// Test invalid transition returns error
    #[test]
    fn test_invalid_transition_returns_error() {
        let mut manager = DialogueManager::new();

        // Can't go directly from Idle to Processing
        let result = manager.handle_event(DialogueEvent::SpeechDetected);

        assert!(result.is_err());
    }

    /// Test stop event transitions to Idle from any state
    #[test]
    fn test_stop_event_transitions_to_idle() {
        let mut manager = DialogueManager::new();

        // From Idle
        manager.handle_event(DialogueEvent::Stop).unwrap();
        assert_eq!(manager.state(), DialogueState::Idle);

        // From Listening
        manager
            .handle_event(DialogueEvent::WakeWordDetected)
            .unwrap();
        manager.handle_event(DialogueEvent::Stop).unwrap();
        assert_eq!(manager.state(), DialogueState::Idle);

        // From Processing
        manager
            .handle_event(DialogueEvent::WakeWordDetected)
            .unwrap();
        manager.handle_event(DialogueEvent::SpeechDetected).unwrap();
        manager.handle_event(DialogueEvent::Stop).unwrap();
        assert_eq!(manager.state(), DialogueState::Idle);

        // From Speaking
        manager
            .handle_event(DialogueEvent::WakeWordDetected)
            .unwrap();
        manager.handle_event(DialogueEvent::SpeechDetected).unwrap();
        manager.handle_event(DialogueEvent::ResponseReady).unwrap();
        manager.handle_event(DialogueEvent::Stop).unwrap();
        assert_eq!(manager.state(), DialogueState::Idle);
    }

    /// Test can_interrupt returns true only during Speaking
    #[test]
    fn test_can_interrupt() {
        let mut manager = DialogueManager::new();

        // Not interruptible in Idle
        assert!(!manager.can_interrupt());

        // Not interruptible in Listening
        manager
            .handle_event(DialogueEvent::WakeWordDetected)
            .unwrap();
        assert!(!manager.can_interrupt());

        // Not interruptible in Processing
        manager.handle_event(DialogueEvent::SpeechDetected).unwrap();
        assert!(!manager.can_interrupt());

        // Interruptible during Speaking
        manager.handle_event(DialogueEvent::ResponseReady).unwrap();
        assert!(manager.can_interrupt());
    }
}
