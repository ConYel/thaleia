//! MCP Tools for Thaleia (Legacy implementation)
//!
//! NOTE: This is the OLD implementation kept for backward compatibility.
//! The new implementation uses rmcp_server.rs with the official rmcp SDK.
//!
//! Implements the core tools exposed via MCP protocol:
//! - speak: Text-to-speech synthesis
//! - listen: Speech-to-text transcription
//! - list_voices: Show available voices
//! - get_status: Current system status

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

use crate::session::SessionState;
use crate::{Result, is_debug};

/// Debug print helper - only prints if debug mode is enabled
macro_rules! dbg_print {
    ($($arg:tt)*) => {
        if is_debug() {
            use std::io::Write;
            let _ = std::io::stderr().write_all(b"\x1b[1;36m"); // Cyan for debug
            eprintln!($($arg)*);
            let _ = std::io::stderr().write_all(b"\x1b[0m"); // Reset
        }
    };
}

/// Color codes for debug output
pub mod colors {
    pub const RESET: &str = "\x1b[0m";
    pub const BOLD: &str = "\x1b[1m";
    pub const RED: &str = "\x1b[31m";
    pub const GREEN: &str = "\x1b[32m";
    pub const YELLOW: &str = "\x1b[33m";
    pub const BLUE: &str = "\x1b[34m";
    pub const MAGENTA: &str = "\x1b[35m";
    pub const CYAN: &str = "\x1b[36m";
}

/// MCP Tool definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

/// MCP Tool call arguments
#[derive(Debug, Clone, Deserialize)]
pub struct ToolCall {
    pub name: String,
    pub arguments: serde_json::Value,
}

/// MCP Tool response content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolContent {
    #[serde(rename = "type")]
    pub content_type: String,
    pub text: String,
}

/// MCP Tool response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResponse {
    pub content: Vec<ToolContent>,
    pub is_error: Option<bool>,
}

impl ToolResponse {
    /// Create text response
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            content: vec![ToolContent {
                content_type: "text".to_string(),
                text: text.into(),
            }],
            is_error: None,
        }
    }

    /// Create error response
    pub fn error(text: impl Into<String>) -> Self {
        Self {
            content: vec![ToolContent {
                content_type: "text".to_string(),
                text: text.into(),
            }],
            is_error: Some(true),
        }
    }
}

/// Available MCP tools
pub const MCP_TOOLS: &[(&str, &str, &str)] = &[
    (
        "thaleia_speak",
        "Convert text to speech using Kokoro TTS. Speaks the text aloud using the specified voice.",
        r#"{
            "type": "object",
            "properties": {
                "text": {
                    "type": "string",
                    "description": "Text to speak"
                },
                "voice": {
                    "type": "string",
                    "description": "Voice ID (e.g., 'af_bella', 'am_adam'). If not specified, uses default voice."
                }
            },
            "required": ["text"]
        }"#,
    ),
    (
        "thaleia_listen",
        "Listen to microphone and transcribe speech to text using Whisper STT.",
        r#"{
            "type": "object",
            "properties": {
                "timeout": {
                    "type": "number",
                    "description": "Maximum listening duration in seconds (default: 10)"
                },
                "save_to": {
                    "type": "string",
                    "description": "Save captured audio to WAV file (optional)"
                }
            }
        }"#,
    ),
    (
        "thaleia_list_voices",
        "List all available TTS voices with their names and languages.",
        r#"{
            "type": "object",
            "properties": {}
        }"#,
    ),
    (
        "thaleia_get_status",
        "Get current Thaleia MCP server status including session info and token budget.",
        r#"{
            "type": "object",
            "properties": {}
        }"#,
    ),
];

/// Tool executor trait
pub trait ToolExecutor: Send + Sync {
    /// Execute a tool by name
    fn execute(&self, call: ToolCall) -> Result<ToolResponse>;
}

/// Tool executor implementation using Thaleia core
pub struct ThaleiaToolExecutor {
    tts: Arc<Mutex<Option<thaleia_core::TtsEngine>>>,
    #[cfg(feature = "whisper")]
    stt: Arc<Mutex<Option<thaleia_core::WhisperEngine>>>,
    session: Arc<Mutex<SessionState>>,
}

impl ThaleiaToolExecutor {
    /// Create new executor
    pub fn new() -> Self {
        Self {
            tts: Arc::new(Mutex::new(None)),
            #[cfg(feature = "whisper")]
            stt: Arc::new(Mutex::new(None)),
            session: Arc::new(Mutex::new(SessionState::new())),
        }
    }

    /// Execute tool by name
    pub async fn execute_tool(&self, call: ToolCall) -> Result<ToolResponse> {
        match call.name.as_str() {
            "thaleia_speak" => self.execute_speak(call.arguments).await,
            "thaleia_listen" => self.execute_listen(call.arguments).await,
            "thaleia_list_voices" => self.execute_list_voices().await,
            "thaleia_get_status" => self.execute_status().await,
            _ => Ok(ToolResponse::error(format!("Unknown tool: {}", call.name))),
        }
    }

    /// Execute speak tool
    async fn execute_speak(&self, args: serde_json::Value) -> Result<ToolResponse> {
        let text = args
            .get("text")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'text' argument"))?;

        let voice = args.get("voice").and_then(|v| v.as_str());

        // Get or initialize TTS engine
        let mut tts_guard = self.tts.lock().await;
        if tts_guard.is_none() {
            *tts_guard = Some(thaleia_core::TtsEngine::new().await?);
        }
        let tts = tts_guard.as_mut().unwrap();

        // Synthesize
        tracing::info!("MCP speak: {:?}", text);
        let samples = tts.synthesize(text, voice)?;

        // Play audio using unified audio system
        let mut audio = thaleia_core::AudioEngine::new();
        if audio.is_available() {
            audio.audio_system_mut().play(&samples, 24000)?;
        } else {
            tracing::warn!("Audio not available, skipping playback");
        }

        Ok(ToolResponse::text(format!("Spoke: {}", text)))
    }

    /// Execute listen tool
    #[cfg(feature = "whisper")]
    async fn execute_listen(&self, args: serde_json::Value) -> Result<ToolResponse> {
        use std::path::Path;
        use thaleia_core::SttBackend;

        let timeout = args.get("timeout").and_then(|v| v.as_f64()).unwrap_or(10.0) as u64;
        let save_to = args.get("save_to").and_then(|v| v.as_str());

        // Get or initialize STT engine
        let mut stt_guard = self.stt.lock().await;
        if stt_guard.is_none() {
            *stt_guard =
                Some(thaleia_core::WhisperEngine::new(thaleia_core::ModelSize::Tiny).await?);
        }
        let stt = stt_guard.as_mut().unwrap();

        dbg_print!(
            "{}🎤{} MCP listen: timeout={}s",
            colors::CYAN,
            colors::RESET,
            timeout
        );

        // Check if microphone is available
        if !thaleia_core::AudioCapture::is_available() {
            dbg_print!("{}❌{} No microphone found", colors::RED, colors::RESET);
            return Ok(ToolResponse::error(
                "No microphone found. Please connect a microphone.",
            ));
        }

        dbg_print!(
            "{}🎙️  Recording... speak now!{}",
            colors::YELLOW,
            colors::RESET
        );

        // Capture audio from microphone (blocking call)
        let captured = capture_microphone(timeout)?;

        dbg_print!(
            "{}⏹️  Stopped. Got {}{}{} samples",
            colors::BLUE,
            colors::BOLD,
            captured.samples.len(),
            colors::RESET
        );

        // Save to file if requested
        let mut response_text = String::new();
        if let Some(path_str) = save_to {
            let path = Path::new(path_str);
            match captured.save_wav(path) {
                Ok(saved_path) => {
                    dbg_print!(
                        "{}💾{} Audio saved to: {}",
                        colors::GREEN,
                        colors::RESET,
                        saved_path
                    );
                    response_text = format!("Audio saved to: {}\n", saved_path);
                }
                Err(e) => {
                    dbg_print!(
                        "{}❌{} Failed to save audio: {}",
                        colors::RED,
                        colors::RESET,
                        e
                    );
                    response_text = format!("Failed to save audio: {}\n", e);
                }
            }
        }

        let audio = captured.samples;

        if audio.is_empty() {
            dbg_print!("{}❌{} No audio captured", colors::RED, colors::RESET);
            return Ok(ToolResponse::text(
                "No audio captured. Try speaking louder or closer to the microphone.",
            ));
        }

        dbg_print!("{}🔄  Transcribing...{}", colors::MAGENTA, colors::RESET);

        // Transcribe (use 44100Hz sample rate from SDL2)
        let result = stt.transcribe(&audio, captured.sample_rate)?;
        let text = result.text;

        dbg_print!("{}✅  You said:{} {}", colors::GREEN, colors::RESET, text);

        // Add to response
        response_text.push_str(&text);

        // Add to session history
        let mut session = self.session.lock().await;
        session.add_exchange(&text, "");

        Ok(ToolResponse::text(response_text))
    }

    /// Execute listen tool (no whisper feature)
    #[cfg(not(feature = "whisper"))]
    async fn execute_listen(&self, _args: serde_json::Value) -> Result<ToolResponse> {
        Ok(ToolResponse::error(
            "STT not enabled. Rebuild with --features whisper",
        ))
    }

    /// Execute list_voices tool
    async fn execute_list_voices(&self) -> Result<ToolResponse> {
        let mut tts_guard = self.tts.lock().await;
        if tts_guard.is_none() {
            *tts_guard = Some(thaleia_core::TtsEngine::new().await?);
        }
        let tts = tts_guard.as_mut().unwrap();

        let voices = tts.list_voices();

        let text = voices
            .iter()
            .map(|v| format!("{:?}: {} ({})", v.id, v.name, v.language))
            .collect::<Vec<_>>()
            .join("\n");

        Ok(ToolResponse::text(text))
    }

    /// Execute get_status tool
    async fn execute_status(&self) -> Result<ToolResponse> {
        let session = self.session.lock().await;
        let (used, max) = session.budget_info();

        let status = format!(
            "Thaleia MCP Server\n\
             Session mode: {:?}\n\
             Token budget: {}/{} ({:.1}% used)",
            session.mode,
            used,
            max,
            (used as f64 / max as f64) * 100.0
        );

        Ok(ToolResponse::text(status))
    }
}

impl Default for ThaleiaToolExecutor {
    fn default() -> Self {
        Self::new()
    }
}

/// Capture audio from microphone
fn capture_microphone(timeout_secs: u64) -> Result<thaleia_core::capture::CapturedAudio> {
    let capture = thaleia_core::AudioCapture::new()
        .map_err(|e| anyhow::anyhow!("Failed to create audio capture: {}", e))?;

    let duration = Duration::from_secs(timeout_secs);
    capture
        .capture(duration)
        .map_err(|e| anyhow::anyhow!("Failed to capture audio: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_definitions() {
        for (name, desc, schema) in MCP_TOOLS {
            assert!(!name.is_empty());
            assert!(!desc.is_empty());
            // Verify schema is valid JSON
            serde_json::from_str::<serde_json::Value>(schema).unwrap();
        }
    }

    #[tokio::test]
    async fn test_executor_creation() {
        let executor = ThaleiaToolExecutor::new();
        let session = executor.session.lock().await;
        assert_eq!(session.mode, crate::session::MemoryMode::Standard);
    }
}
