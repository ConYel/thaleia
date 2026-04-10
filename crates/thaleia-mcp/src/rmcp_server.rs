//! Thaleia MCP Server - Voice AI integration
//!
//! Integrates thaleia-core TTS (Kokoro) and STT (Whisper) with MCP protocol.
//! Uses AudioThreadManager to bridge async MCP with sync audio resources.
//!
//! Follows SOLID principles: SRP, OCP, ISP, DIP.

use rmcp::{
    ErrorData as McpError, ServerHandler, ServiceExt,
    handler::server::tool::ToolRouter,
    handler::server::wrapper::Parameters,
    model::{
        CallToolResult, Content, Implementation, InitializeResult, ProtocolVersion,
        ServerCapabilities, ServerInfo,
    },
    tool, tool_handler, tool_router,
};

use crate::audio_manager::AudioThreadManager;
use thaleia_core::is_debug;
use thaleia_core::thaleia_debug;

/// Session mode for memory management
#[derive(Debug, Clone, Copy, Default)]
pub enum SessionMode {
    #[default]
    Standard,
    Ephemeral,
    Longterm,
}

/// Tool parameter structs
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[schemars(description = "Parameters for text-to-speech")]
pub struct SpeakParams {
    #[schemars(description = "Text to speak")]
    pub text: String,
    #[schemars(description = "Voice ID (e.g., af_bella, am_adam)")]
    pub voice: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[schemars(description = "Parameters for speech-to-text")]
pub struct ListenParams {
    #[schemars(description = "Max listening duration in seconds")]
    pub timeout: Option<f64>,
    #[schemars(description = "Audio source name (e.g., vchan_input, default for auto-detect)")]
    pub source: Option<String>,
}

/// User-friendly error messages following Thaleia's character
fn user_friendly_error(error: &str) -> String {
    if error.contains("audio") || error.contains("playback") || error.contains("device") {
        "I can't find my voice right now. Check your audio settings!".to_string()
    } else if error.contains("microphone") || error.contains("capture") || error.contains("input") {
        "I can't hear you. Check if my microphone is connected!".to_string()
    } else if error.contains("synthesis") || error.contains("TTS") {
        "I had trouble finding the right words. Let me try again!".to_string()
    } else if error.contains("transcription") || error.contains("Whisper") || error.contains("STT")
    {
        "I didn't catch that. Could you speak again?".to_string()
    } else if error.contains("download") || error.contains("network") || error.contains("model") {
        "I'm downloading my voice models. This might take a moment...".to_string()
    } else {
        format!("Oops! Something went wrong: {}", error)
    }
}

/// Thaleia MCP Handler - using rmcp macros
///
/// Follows SRP: Only responsible for MCP tool handling
/// Uses AudioThreadManager for audio operations (bridges async/sync)
#[derive(Clone)]
pub struct ThaleiaHandler {
    tool_router: ToolRouter<Self>,
    session_mode: SessionMode,
    audio_manager: AudioThreadManager,
}

impl ThaleiaHandler {
    pub fn new(mode: SessionMode) -> Self {
        let audio_manager = AudioThreadManager::new().expect("Failed to initialize audio manager");

        Self {
            tool_router: Self::tool_router(),
            session_mode: mode,
            audio_manager,
        }
    }
}

/// Tool implementations using #[tool] macro
///
/// Following OCP: New tools can be added without modifying existing ones
#[tool_router]
impl ThaleiaHandler {
    /// List all available TTS voices from Kokoro
    #[tool(
        name = "thaleia_list_voices",
        description = "List all available TTS voices"
    )]
    async fn list_voices(&self) -> Result<CallToolResult, McpError> {
        let voices = self.audio_manager.list_voices().await;
        let voice_list = voices.join("\n");

        Ok(CallToolResult::success(vec![Content::text(voice_list)]))
    }

    /// Convert text to speech using Kokoro TTS and play audio
    #[tool(
        name = "thaleia_speak",
        description = "Convert text to speech using Kokoro TTS"
    )]
    async fn speak(
        &self,
        Parameters(SpeakParams { text, voice }): Parameters<SpeakParams>,
    ) -> Result<CallToolResult, McpError> {
        let voice_str = voice.as_deref();

        match self.audio_manager.synthesize(&text, voice_str).await {
            Ok(response) => {
                thaleia_debug!(
                    "Synthesized {} audio samples at {}Hz",
                    response.samples.len(),
                    response.sample_rate
                );

                Ok(CallToolResult::success(vec![Content::text(format!(
                    "Spoke: '{}' with voice: {}",
                    text,
                    voice_str.unwrap_or("af_sarah")
                ))]))
            }
            Err(e) => {
                tracing::error!("TTS error: {}", e);
                Err(McpError::internal_error(user_friendly_error(&e), None))
            }
        }
    }

    /// Listen to microphone and transcribe speech using Whisper
    #[tool(
        name = "thaleia_listen",
        description = "Listen to microphone and transcribe speech"
    )]
    async fn listen(
        &self,
        Parameters(ListenParams { timeout, source }): Parameters<ListenParams>,
    ) -> Result<CallToolResult, McpError> {
        let timeout_secs = timeout.unwrap_or(30.0) as u64;

        match self
            .audio_manager
            .transcribe(timeout_secs, source.as_deref())
            .await
        {
            Ok(response) => {
                thaleia_debug!("Transcribed: {}", response.text);

                // Build response with debug info
                let mut response_text = response.text.clone();
                if let Some(ref debug) = response.debug {
                    response_text.push_str(&format!(
                        "\n[Debug: {} backend, {} samples]",
                        debug.backend, debug.samples
                    ));
                }

                Ok(CallToolResult::success(vec![Content::text(response_text)]))
            }
            Err(e) => {
                tracing::error!("STT error: {}", e);
                Err(McpError::internal_error(user_friendly_error(&e), None))
            }
        }
    }

    /// List all available audio input sources
    #[tool(
        name = "thaleia_list_sources",
        description = "List all available audio input sources"
    )]
    async fn list_sources(&self) -> Result<CallToolResult, McpError> {
        match self.audio_manager.list_sources().await {
            Ok(sources) => {
                if sources.is_empty() {
                    Ok(CallToolResult::success(vec![Content::text(
                        "No audio input sources found".to_string(),
                    )]))
                } else {
                    Ok(CallToolResult::success(vec![Content::text(
                        sources.join("\n"),
                    )]))
                }
            }
            Err(e) => {
                tracing::error!("List sources error: {}", e);
                Err(McpError::internal_error(
                    format!("Failed to list sources: {}", e),
                    None,
                ))
            }
        }
    }

    /// Get current Thaleia MCP server status
    #[tool(
        name = "thaleia_get_status",
        description = "Get current Thaleia MCP server status"
    )]
    async fn get_status(&self) -> Result<CallToolResult, McpError> {
        // Get audio debug info from the audio thread
        let audio_debug = self.audio_manager.get_audio_debug();

        let mut status_lines = vec![
            format!("Thaleia MCP Server v{}", env!("CARGO_PKG_VERSION")),
            format!("Session mode: {:?}", self.session_mode),
            "TTS: Kokoro (enabled)".to_string(),
            "STT: Whisper (enabled)".to_string(),
            "Audio: Enabled (thread-based)".to_string(),
        ];

        // Add audio debug info
        if !audio_debug.debug_file.is_empty() {
            status_lines.push("".to_string());
            status_lines.push("=== Audio Debug ===".to_string());
            for line in audio_debug.debug_file.lines() {
                status_lines.push(format!("   {}", line));
            }
        }

        Ok(CallToolResult::success(vec![Content::text(
            status_lines.join("\n"),
        )]))
    }
}

/// ServerHandler implementation using #[tool_handler] macro
#[tool_handler]
impl ServerHandler for ThaleiaHandler {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(
            ServerCapabilities::builder()
                .enable_tools()
                .build(),
        )
        .with_server_info(Implementation::new("thaleia", env!("CARGO_PKG_VERSION")))
        .with_protocol_version(ProtocolVersion::V_2024_11_05)
        .with_instructions("Thaleia provides voice AI tools: speak (TTS), listen (STT), list_voices, get_status. Use these to give Thaleia ears and a mouth!")
    }

    async fn initialize(
        &self,
        request: rmcp::model::InitializeRequestParams,
        _context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> Result<InitializeResult, McpError> {
        // Debug: Log what we received (if debug enabled)
        if is_debug() {
            thaleia_debug!("initialize request:");
            thaleia_debug!("  protocol_version: {:?}", request.protocol_version);
            thaleia_debug!("  client_info: {:?}", request.client_info);
        }

        tracing::info!("Thaleia MCP Server connected");

        Ok(self.get_info())
    }
}

// =============================================================================
// Server Execution
// =============================================================================

/// Run MCP server with stdio transport
pub fn run_server() -> Result<(), Box<dyn std::error::Error>> {
    let mode = std::env::var("THALEIA_MODE")
        .map(|m| match m.as_str() {
            "ephemeral" => SessionMode::Ephemeral,
            "longterm" => SessionMode::Longterm,
            _ => SessionMode::Standard,
        })
        .unwrap_or(SessionMode::Standard);

    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async move {
        let handler = ThaleiaHandler::new(mode);
        let server = handler.serve(rmcp::transport::stdio()).await?;
        server.waiting().await?;
        Ok::<(), Box<dyn std::error::Error>>(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rmcp::handler::server::wrapper::Parameters;

    /// Test that list_voices returns voices
    #[tokio::test]
    async fn test_list_voices() {
        let handler = ThaleiaHandler::new(SessionMode::Standard);
        let result = handler.list_voices().await;

        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(!result.content.is_empty());

        // Use Content::text() to create content, then match on the result
        let text = &result.content[0];
        // In rmcp v1.3, content is accessed differently - check if it contains text
        let content_str = format!("{:?}", text);
        assert!(
            content_str.contains("af_")
                || content_str.contains("Sarah")
                || content_str.contains("voice"),
            "Expected voice content, got: {}",
            content_str
        );
    }

    /// Test speak structure
    #[tokio::test]
    async fn test_speak() {
        let handler = ThaleiaHandler::new(SessionMode::Standard);

        // Create properly wrapped parameters
        let params = Parameters(SpeakParams {
            text: "Hello".to_string(),
            voice: Some("af_sarah".to_string()),
        });
        let result = handler.speak(params).await;

        assert!(result.is_ok());
    }

    /// Test listen structure
    #[tokio::test]
    async fn test_listen() {
        let handler = ThaleiaHandler::new(SessionMode::Standard);

        // Create properly wrapped parameters
        let params = Parameters(ListenParams {
            timeout: Some(5.0),
            source: None,
        });
        let result = handler.listen(params).await;

        assert!(result.is_ok());
    }

    /// Test status shows version
    #[tokio::test]
    async fn test_get_status() {
        let handler = ThaleiaHandler::new(SessionMode::Standard);
        let result = handler.get_status().await;

        assert!(result.is_ok());
        let result = result.unwrap();

        // Check that we got some content back
        assert!(!result.content.is_empty(), "Expected non-empty content");
    }
}
