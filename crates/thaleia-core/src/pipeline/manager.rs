//! Dialogue Manager - orchestrates the voice AI pipeline
//!
//! The DialogueManager handles the state machine for voice interactions,
//! managing transitions between Idle, Listening, Processing, and Speaking states.
//!
//! Key responsibilities:
//! - Track current dialogue state
//! - Handle state transitions based on events
//! - Support interruption (user can stop AI at any time)
//! - Provide a simple API for the voice pipeline

use crate::Error;

use super::events::DialogueEvent;
use super::state::DialogueState;

/// Dialogue Manager - manages voice AI interaction flow
///
/// # State Machine
///
/// The manager follows a simple state machine:
/// - Idle → Listening: Wake word or push-to-talk
/// - Listening → Processing: Speech detected by VAD
/// - Processing → Speaking: Response ready (TTS)
/// - Speaking → Idle: Speech finished
/// - Speaking → Listening: Interruption (user speaks)
///
/// # Interruption Handling
///
/// Users can interrupt the AI at any time. When speech is detected
/// while in the Speaking state, the manager immediately transitions
/// to Listening (stopping TTS playback).
///
/// # Example
///
/// ```rust,ignore
/// let mut manager = DialogueManager::new();
///
/// // User says wake word
/// manager.handle_event(DialogueEvent::WakeWordDetected)?;
/// assert_eq!(manager.state(), DialogueState::Listening);
///
/// // VAD detects speech
/// manager.handle_event(DialogueEvent::SpeechDetected)?;
/// assert_eq!(manager.state(), DialogueState::Processing);
///
/// // LLM generates response
/// manager.handle_event(DialogueEvent::ResponseReady)?;
/// assert_eq!(manager.state(), DialogueState::Speaking);
///
/// // User interrupts
/// manager.handle_event(DialogueEvent::SpeechDetected)?;
/// assert_eq!(manager.state(), DialogueState::Listening);
/// ```
#[derive(Debug)]
pub struct DialogueManager {
    /// Current dialogue state
    state: DialogueState,
}

impl DialogueManager {
    /// Create a new DialogueManager in Idle state
    #[must_use]
    pub fn new() -> Self {
        Self {
            state: DialogueState::default(),
        }
    }

    /// Get the current dialogue state
    #[must_use]
    pub fn state(&self) -> DialogueState {
        self.state
    }

    /// Check if the current state can be interrupted
    ///
    /// Returns true only when the AI is speaking - users can
    /// interrupt at any time during speech.
    #[must_use]
    pub fn can_interrupt(&self) -> bool {
        self.state.can_interrupt()
    }

    /// Check if the system is currently in an active state
    ///
    /// Active states are: Listening, Processing, Speaking
    #[must_use]
    pub fn is_active(&self) -> bool {
        self.state.is_active()
    }

    /// Handle a dialogue event and transition state
    ///
    /// # Errors
    ///
    /// Returns an error if the event is not valid for the current state.
    /// The Stop and Error events are always valid from any state.
    pub fn handle_event(&mut self, event: DialogueEvent) -> Result<(), Error> {
        // Stop and Error are always valid
        if event == DialogueEvent::Stop {
            self.state = DialogueState::Idle;
            return Ok(());
        }

        if event == DialogueEvent::Error {
            self.state = DialogueState::Idle;
            return Ok(());
        }

        // Check if transition is valid
        if !event.is_valid_from(self.state) {
            return Err(Error::Internal(format!(
                "Invalid transition: {:?} from {:?}",
                event, self.state
            )));
        }

        // Apply state transition
        self.state = self.transition(event);

        Ok(())
    }

    /// Determine next state based on event
    ///
    /// # Panics
    ///
    /// Panics if the event is not valid for the current state.
    /// This should never happen if `is_valid_from` is checked first.
    fn transition(&self, event: DialogueEvent) -> DialogueState {
        use DialogueState::*;

        match (self.state, event) {
            // Idle → Listening
            (Idle, DialogueEvent::WakeWordDetected) => Listening,
            (Idle, DialogueEvent::PushToTalkStart) => Listening,

            // Listening → Processing or Idle
            (Listening, DialogueEvent::SpeechDetected) => Processing,
            (Listening, DialogueEvent::SilenceTimeout) => Idle,

            // Processing → Speaking or Idle
            (Processing, DialogueEvent::ResponseReady) => Speaking,
            (Processing, DialogueEvent::Error) => Idle,

            // Speaking → Listening (interruption) or Idle (finished)
            (Speaking, DialogueEvent::SpeechDetected) => Listening, // Interruption!
            (Speaking, DialogueEvent::SpeechFinished) => Idle,

            // SAFETY: All valid transitions should be covered above.
            // This unreachable pattern ensures we catch bugs if is_valid_from
            // and transition get out of sync.
            _ => unreachable!(
                "Invalid state-event combination: {:?} + {:?}",
                self.state, event
            ),
        }
    }
}

impl Default for DialogueManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_manager_is_idle() {
        let manager = DialogueManager::new();
        assert_eq!(manager.state(), DialogueState::Idle);
    }

    #[test]
    fn test_is_active() {
        let mut manager = DialogueManager::new();

        assert!(!manager.is_active());

        manager
            .handle_event(DialogueEvent::WakeWordDetected)
            .unwrap();
        assert!(manager.is_active());

        manager.handle_event(DialogueEvent::SpeechDetected).unwrap();
        assert!(manager.is_active());

        manager.handle_event(DialogueEvent::ResponseReady).unwrap();
        assert!(manager.is_active());

        manager.handle_event(DialogueEvent::SpeechFinished).unwrap();
        assert!(!manager.is_active());
    }

    #[test]
    fn test_interruption_preserves_audio_context() {
        // When interrupted during Speaking, we go to Listening
        // (not Processing) - this allows the user to immediately
        // issue a new command without waiting for the previous
        // TTS to finish
        let mut manager = DialogueManager::new();

        manager
            .handle_event(DialogueEvent::WakeWordDetected)
            .unwrap();
        manager.handle_event(DialogueEvent::SpeechDetected).unwrap();
        manager.handle_event(DialogueEvent::ResponseReady).unwrap();

        assert_eq!(manager.state(), DialogueState::Speaking);

        // Interrupt!
        manager.handle_event(DialogueEvent::SpeechDetected).unwrap();

        // Should be Listening, not Processing
        assert_eq!(manager.state(), DialogueState::Listening);
    }

    #[test]
    fn test_full_conversation_flow() {
        let mut manager = DialogueManager::new();

        // Wake word
        manager
            .handle_event(DialogueEvent::WakeWordDetected)
            .unwrap();
        assert_eq!(manager.state(), DialogueState::Listening);

        // Speech detected
        manager.handle_event(DialogueEvent::SpeechDetected).unwrap();
        assert_eq!(manager.state(), DialogueState::Processing);

        // Response ready
        manager.handle_event(DialogueEvent::ResponseReady).unwrap();
        assert_eq!(manager.state(), DialogueState::Speaking);

        // Speech finished
        manager.handle_event(DialogueEvent::SpeechFinished).unwrap();
        assert_eq!(manager.state(), DialogueState::Idle);
    }

    #[test]
    fn test_push_to_talk_flow() {
        let mut manager = DialogueManager::new();

        // Push to talk
        manager
            .handle_event(DialogueEvent::PushToTalkStart)
            .unwrap();
        assert_eq!(manager.state(), DialogueState::Listening);

        // Speech detected
        manager.handle_event(DialogueEvent::SpeechDetected).unwrap();
        assert_eq!(manager.state(), DialogueState::Processing);
    }

    #[test]
    fn test_silence_timeout() {
        let mut manager = DialogueManager::new();

        manager
            .handle_event(DialogueEvent::WakeWordDetected)
            .unwrap();
        assert_eq!(manager.state(), DialogueState::Listening);

        manager.handle_event(DialogueEvent::SilenceTimeout).unwrap();
        assert_eq!(manager.state(), DialogueState::Idle);
    }

    #[test]
    fn test_stop_from_any_state() {
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

    #[test]
    fn test_error_from_any_state() {
        let mut manager = DialogueManager::new();

        manager
            .handle_event(DialogueEvent::WakeWordDetected)
            .unwrap();
        manager.handle_event(DialogueEvent::Error).unwrap();
        assert_eq!(manager.state(), DialogueState::Idle);
    }
}
