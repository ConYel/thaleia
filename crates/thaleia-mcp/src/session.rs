//! Session management with green history
//!
//! Implements sparse, forgetful memory by default for environmental efficiency.
//! Token budget: 100K max per session.

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

// =============================================================================
// Constants
// =============================================================================

/// Hot storage: Last N full exchanges (full text)
const HOT_MAX: usize = 3;

/// Warm storage limits per mode
const WARM_MAX_STANDARD: usize = 20;
const WARM_MAX_LONGTERM: usize = 100;

/// Token budget maximum
const TOKEN_BUDGET_MAX: usize = 100_000;

// =============================================================================
// Types
// =============================================================================

/// Memory preset mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MemoryMode {
    /// No persistence - most green, no history
    #[default]
    Ephemeral,
    /// Compressed history - default mode
    Standard,
    /// Long-term memory - more retention
    Longterm,
}

/// Semantic note - compressed exchange representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticNote {
    /// Relative timestamp (seconds since session start)
    pub timestamp: u32,
    /// Compressed topic identifier
    pub topic: String,
    /// Sentiment: -1 (negative) to 1 (positive)
    pub sentiment: i8,
    /// Key outcome if any
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outcome: Option<String>,
}

/// Full exchange for recent history (hot tier)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Exchange {
    pub user: String,
    pub assistant: String,
    pub timestamp: u32,
}

/// Pinned memory - explicitly saved by user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PinnedMemory {
    pub content: String,
    pub created_at: u32,
    pub tags: Vec<String>,
}

/// Session state - minimal by design for green computing
#[derive(Debug, Clone)]
pub struct SessionState {
    /// Memory preset mode
    pub mode: MemoryMode,

    /// Session start time (relative seconds)
    pub start_time: u32,

    /// Current time counter (increments each interaction)
    time_counter: u32,

    /// Hot storage: Last N full exchanges (full text)
    hot_storage: VecDeque<Exchange>,

    /// Warm storage: Semantic notes (compressed)
    warm_storage: VecDeque<SemanticNote>,

    /// Cold storage: Pinned memories (user-requested saves)
    pinned_memories: Vec<PinnedMemory>,

    /// Token budget tracking (approximate)
    pub token_budget_used: usize,
}

impl SessionState {
    /// Create new session with default mode
    pub fn new() -> Self {
        Self {
            mode: MemoryMode::Standard,
            start_time: 0,
            time_counter: 0,
            hot_storage: VecDeque::with_capacity(HOT_MAX),
            warm_storage: VecDeque::new(),
            pinned_memories: Vec::new(),
            token_budget_used: 0,
        }
    }

    /// Create new session with specific mode
    pub fn with_mode(mode: MemoryMode) -> Self {
        Self {
            mode,
            start_time: 0,
            time_counter: 0,
            hot_storage: VecDeque::with_capacity(HOT_MAX),
            warm_storage: VecDeque::new(),
            pinned_memories: Vec::new(),
            token_budget_used: 0,
        }
    }

    /// Get current time marker
    fn current_time(&mut self) -> u32 {
        self.time_counter += 1;
        self.time_counter
    }

    /// Add exchange to session
    pub fn add_exchange(&mut self, user: &str, assistant: &str) -> Option<SemanticNote> {
        let time = self.current_time();

        // Skip ephemeral mode entirely
        if self.mode == MemoryMode::Ephemeral {
            self.token_budget_used += estimate_tokens(user) + estimate_tokens(assistant);
            return None;
        }

        // Always add to hot storage
        let exchange = Exchange {
            user: user.to_string(),
            assistant: assistant.to_string(),
            timestamp: time,
        };

        self.hot_storage.push_back(exchange);
        if self.hot_storage.len() > HOT_MAX {
            self.hot_storage.pop_front();
        }

        // Create semantic note
        let semantic = SemanticNote {
            timestamp: time,
            topic: extract_topic(user),
            sentiment: analyze_sentiment(user),
            outcome: extract_outcome(assistant),
        };

        // Add to warm storage (compressed)
        let warm_max = match self.mode {
            MemoryMode::Ephemeral => 0,
            MemoryMode::Standard => WARM_MAX_STANDARD,
            MemoryMode::Longterm => WARM_MAX_LONGTERM,
        };

        if warm_max > 0 {
            self.warm_storage.push_back(semantic.clone());
            if self.warm_storage.len() > warm_max {
                self.warm_storage.pop_front();
            }
        }

        self.token_budget_used += estimate_tokens(user) + estimate_tokens(assistant);

        Some(semantic)
    }

    /// Get session history (context for AI)
    pub fn get_context(&self) -> String {
        if self.mode == MemoryMode::Ephemeral {
            return String::new();
        }

        let mut context = String::new();

        if !self.hot_storage.is_empty() {
            context.push_str("Recent conversation:\n");
            for ex in &self.hot_storage {
                context.push_str(&format!("  User: {}\n", ex.user));
                context.push_str(&format!("  Thaleia: {}\n", ex.assistant));
            }
        }

        if (self.mode == MemoryMode::Standard || self.mode == MemoryMode::Longterm)
            && !self.warm_storage.is_empty()
        {
            context.push_str("\nTopics discussed: ");
            let topics: Vec<_> = self.warm_storage.iter().map(|n| n.topic.clone()).collect();
            context.push_str(&topics.join(", "));

            let avg_sentiment: i8 = self.warm_storage.iter().map(|n| n.sentiment).sum::<i8>()
                / self.warm_storage.len() as i8;
            context.push_str(&format!(
                "\nOverall mood: {}\n",
                match avg_sentiment {
                    -3..=-1 => "somewhat negative",
                    0 => "neutral",
                    1..=3 => "positive",
                    _ => "neutral",
                }
            ));
        }

        if !self.pinned_memories.is_empty() {
            context.push_str("\nPinned memories:\n");
            for mem in &self.pinned_memories {
                context.push_str(&format!("  - {}\n", mem.content));
            }
        }

        context
    }

    /// Pin a memory (user explicitly requests to remember)
    pub fn pin_memory(&mut self, content: &str, tags: Vec<String>) {
        let memory = PinnedMemory {
            content: content.to_string(),
            created_at: self.current_time(),
            tags,
        };
        self.pinned_memories.push(memory);
    }

    /// Get token budget info
    pub fn budget_info(&self) -> (usize, usize) {
        (self.token_budget_used, TOKEN_BUDGET_MAX)
    }

    /// Check if under budget
    pub fn under_budget(&self) -> bool {
        self.token_budget_used < TOKEN_BUDGET_MAX
    }
}

impl Default for SessionState {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Simple heuristics for semantic extraction (no ML needed)
// =============================================================================

/// Estimate token count (rough approximation)
fn estimate_tokens(text: &str) -> usize {
    text.len() / 4
}

/// Extract topic from text (simple keyword extraction)
fn extract_topic(text: &str) -> String {
    let text_lower = text.to_lowercase();

    let topics = [
        ("weather", "weather"),
        ("time", "time"),
        ("help", "help request"),
        ("code", "programming"),
        ("rust", "programming"),
        ("python", "programming"),
        ("music", "music"),
        ("movie", "entertainment"),
        ("book", "reading"),
        ("travel", "travel"),
        ("food", "food"),
        ("work", "work"),
        ("project", "project"),
        ("meeting", "meeting"),
        ("question", "question"),
        ("problem", "problem solving"),
        ("error", "debugging"),
        ("thanks", "gratitude"),
        ("sorry", "apology"),
        ("hello", "greeting"),
        ("bye", "farewell"),
    ];

    for (keyword, topic) in topics {
        if text_lower.contains(keyword) {
            return topic.to_string();
        }
    }

    text.split_whitespace()
        .next()
        .map(|w| {
            if w.len() > 3 {
                w.trim_matches(|c: char| !c.is_alphanumeric())
                    .to_lowercase()
            } else {
                "general".to_string()
            }
        })
        .unwrap_or_else(|| "general".to_string())
}

/// Analyze sentiment (simple heuristic)
fn analyze_sentiment(text: &str) -> i8 {
    let text_lower = text.to_lowercase();

    let mut score: i8 = 0;

    // Positive indicators
    if text_lower.contains("great") || text_lower.contains("good") || text_lower.contains("awesome")
    {
        score += 1;
    }
    if text_lower.contains("excellent")
        || text_lower.contains("perfect")
        || text_lower.contains("wonderful")
    {
        score += 2;
    }
    if text.contains('!') {
        score += 1;
    }
    if text.contains('?') {
        score -= 1;
    }

    // Negative indicators
    if text_lower.contains("bad") || text_lower.contains("terrible") || text_lower.contains("awful")
    {
        score -= 1;
    }
    if text_lower.contains("problem")
        || text_lower.contains("issue")
        || text_lower.contains("error")
    {
        score -= 1;
    }

    score.clamp(-3, 3)
}

/// Extract outcome from assistant response
fn extract_outcome(response: &str) -> Option<String> {
    let lower = response.to_lowercase();

    if lower.contains("decided") || lower.contains("agreed") {
        return Some("decision made".to_string());
    }

    if lower.contains("learned") || lower.contains("found out") || lower.contains("discovered") {
        return Some("information learned".to_string());
    }

    if lower.contains("will do") || lower.contains("going to") || lower.contains("created") {
        return Some("action planned".to_string());
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_new() {
        let session = SessionState::new();
        assert_eq!(session.mode, MemoryMode::Standard);
    }

    #[test]
    fn test_ephemeral_mode() {
        let mut session = SessionState::with_mode(MemoryMode::Ephemeral);
        session.add_exchange("Hello", "Hi there!");
        assert!(session.hot_storage.is_empty());
        assert!(session.warm_storage.is_empty());
    }

    #[test]
    fn test_sentiment_analysis() {
        assert!(analyze_sentiment("This is great!") > 0);
        assert!(analyze_sentiment("I have a problem") < 0);
        assert!(analyze_sentiment("What is the time?") <= 0);
    }

    #[test]
    fn test_topic_extraction() {
        // Note: topics are checked in order, first match wins
        assert_eq!(extract_topic("I need help with Rust code"), "help request");
        assert_eq!(extract_topic("Rust code is great"), "programming");
        assert_eq!(extract_topic("How is the weather?"), "weather");
        assert_eq!(extract_topic("Hello there"), "greeting"); // "hello" maps to "greeting"
    }

    #[test]
    fn test_context_generation() {
        let mut session = SessionState::new();
        session.add_exchange("I need help with Rust", "I can help with Rust!");
        let context = session.get_context();
        assert!(context.contains("Recent conversation"));
        assert!(context.contains("Rust"));
    }
}
