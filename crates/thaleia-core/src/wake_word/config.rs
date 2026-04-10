//! Wake Word configuration and result types
//!
//! Provides domain types for wake word detection configuration and results.

/// Wake word detection configuration
///
/// # Example
/// ```rust,ignore
/// use thaleia_core::wake_word::WakeWordConfig;
///
/// let config = WakeWordConfig::default();
/// // Or customize:
/// let config = WakeWordConfig {
///     keywords: vec!["hey_thaleia".to_string()],
///     sample_rate: 16000,
///     threshold: 0.7,
/// };
/// ```
#[derive(Debug, Clone)]
pub struct WakeWordConfig {
    /// Wake word keywords to detect
    /// Default: ["hey_thaleia"]
    pub keywords: Vec<String>,
    /// Audio sample rate (typically 16000)
    pub sample_rate: u32,
    /// Detection threshold (0.0 - 1.0)
    /// Higher = more strict (fewer false positives)
    /// Lower = more sensitive (more false positives)
    pub threshold: f32,
}

impl Default for WakeWordConfig {
    fn default() -> Self {
        Self {
            keywords: vec!["hey_thaleia".to_string()],
            sample_rate: 16000,
            threshold: 0.5,
        }
    }
}

/// Result of wake word detection
///
/// # Example
/// ```rust,ignore
/// use thaleia_core::wake_word::WakeWordResult;
///
/// let result = WakeWordResult {
///     detected: true,
///     keyword: Some("hey_thaleia".to_string()),
///     confidence: 0.95,
/// };
/// ```
#[derive(Debug, Clone)]
pub struct WakeWordResult {
    /// Whether a wake word was detected
    pub detected: bool,
    /// The detected keyword (if detected)
    pub keyword: Option<String>,
    /// Detection confidence (0.0 - 1.0)
    pub confidence: f32,
}

impl WakeWordResult {
    /// Create a "not detected" result
    #[must_use]
    pub fn not_detected() -> Self {
        Self {
            detected: false,
            keyword: None,
            confidence: 0.0,
        }
    }

    /// Create a detected result
    #[must_use]
    pub fn detected(keyword: impl Into<String>, confidence: f32) -> Self {
        Self {
            detected: true,
            keyword: Some(keyword.into()),
            confidence,
        }
    }
}

impl Default for WakeWordResult {
    fn default() -> Self {
        Self::not_detected()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = WakeWordConfig::default();
        assert_eq!(config.keywords, vec!["hey_thaleia".to_string()]);
        assert_eq!(config.sample_rate, 16000);
        assert_eq!(config.threshold, 0.5);
    }

    #[test]
    fn test_result_not_detected() {
        let result = WakeWordResult::not_detected();
        assert!(!result.detected);
        assert!(result.keyword.is_none());
        assert_eq!(result.confidence, 0.0);
    }

    #[test]
    fn test_result_detected() {
        let result = WakeWordResult::detected("hey_thaleia", 0.95);
        assert!(result.detected);
        assert_eq!(result.keyword.as_deref(), Some("hey_thaleia"));
        assert!((result.confidence - 0.95).abs() < 0.001);
    }

    #[test]
    fn test_result_default() {
        let result = WakeWordResult::default();
        assert!(!result.detected);
    }
}
