//! Pipeline configuration

/// Voice pipeline configuration
///
/// Controls which components are enabled and their settings.
#[derive(Debug, Clone)]
pub struct PipelineConfig {
    /// Enable wake word detection (default: true)
    pub enable_wake_word: bool,
    /// Enable voice activity detection (default: true)
    /// If false, requires manual push-to-talk
    pub enable_vad: bool,
    /// VAD configuration
    pub vad_config: Option<crate::vad::VadConfig>,
    /// Wake word configuration  
    pub wake_word_config: Option<crate::wake_word::WakeWordConfig>,
    /// STT configuration (requires whisper feature)
    #[cfg(feature = "whisper")]
    pub stt_config: Option<crate::stt::SttConfig>,
    /// TTS configuration
    pub tts_config: Option<crate::tts::TtsConfig>,
    /// Silence timeout in seconds (default: 3)
    pub silence_timeout_secs: u32,
    /// Maximum audio buffer duration in seconds (default: 30)
    pub max_audio_buffer_secs: u32,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            enable_wake_word: true,
            enable_vad: true,
            vad_config: None,
            wake_word_config: None,
            #[cfg(feature = "whisper")]
            stt_config: None,
            tts_config: None,
            silence_timeout_secs: 3,
            max_audio_buffer_secs: 30,
        }
    }
}

/// TTS configuration placeholder
///
/// Currently empty but can be extended with voice, speed, etc.
#[derive(Debug, Clone, Default)]
pub struct TtsConfig {
    // Reserved for future TTS configuration
    // e.g., voice_id, speed, etc.
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = PipelineConfig::default();
        assert!(config.enable_wake_word);
        assert!(config.enable_vad);
        assert_eq!(config.silence_timeout_secs, 3);
        assert_eq!(config.max_audio_buffer_secs, 30);
    }

    #[test]
    fn test_custom_config() {
        let config = PipelineConfig {
            enable_wake_word: false,
            enable_vad: true,
            silence_timeout_secs: 5,
            ..Default::default()
        };

        assert!(!config.enable_wake_word);
        assert!(config.enable_vad);
        assert_eq!(config.silence_timeout_secs, 5);
    }
}
