# Thaleia Architecture: Comprehensive Technical Specification

> *"The Joyful Muse of Comedy, rendered in Rust"*

---

## ⚠️ Target Architecture - Not Yet Implemented

This document describes Thaleia's **target architecture** - the system we're building toward, not what's currently implemented.

**For the current implementation state, see:**
- [PLAN.md](./PLAN.md) - Roadmap with phases and progress
- [README.md](./README.md) - Current features and status

---

## Table of Contents

1. [System Overview](#system-overview)
2. [Core Components](#core-components)
3. [Data Flow](#data-flow)
4. [Audio Pipeline](#audio-pipeline)
5. [Plugin Architecture](#plugin-architecture)
6. [MCP Server Design](#mcp-server-design)
7. [Concurrency Model](#concurrency-model)
8. [Error Handling](#error-handling)
9. [Configuration](#configuration)

---

## System Overview

Thaleia follows a **modular monolith** architecture:

```
┌─────────────────────────────────────────────────────────────────┐
│                         THALEIA                                  │
│                   Rust Binary Distribution                        │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐ │
│  │   CLI/TUI   │  │   MCP/JSON  │  │   WebSocket (Future)   │ │
│  └──────┬──────┘  └──────┬──────┘  └───────────┬─────────────┘ │
│         │                │                      │               │
│  ┌──────┴────────────────┴──────────────────────┴────────────┐ │
│  │                    CORE ENGINE                             │ │
│  │                                                         │ │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌────────┐  │ │
│  │  │   VAD    │─▶│   STT    │─▶│   LLM    │─▶│   TTS   │  │ │
│  │  │ (Silero) │  │(Whisper) │  │ (Plugin) │  │(Kokoro)│  │ │
│  │  └──────────┘  └──────────┘  └──────────┘  └────────┘  │ │
│  │         │                                                   │ │
│  │  ┌──────┴──────────────────────────────────────────────┐  │ │
│  │  │           Session Manager (Context/Memory)            │  │ │
│  │  └──────────────────────────────────────────────────────┘  │ │
│  │                                                         │ │
│  │  ┌──────────────────────────────────────────────────────┐  │ │
│  │  │              Plugin System (Hot-reload)                │  │ │
│  │  └──────────────────────────────────────────────────────┘  │ │
│  │                                                         │ │
│  └─────────────────────────────────────────────────────────┘ │
│                              │                                 │
│  ┌───────────────────────────┴─────────────────────────────┐  │
│  │              Audio I/O Layer (rodio/cpal)                │  │
│  │                                                         │  │
│  │  🎤 Input Devices ───▶ Buffer ───▶ Processing           │  │
│  │                                                         │  │
│  │  Processing ───▶ Buffer ───▶ 🔊 Output Devices         │  │
│  │                                                         │  │
│  └─────────────────────────────────────────────────────────┘  │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## Core Components

### 1. Audio Engine (`thaleia-audio`)

The foundation layer handling all audio I/O.

#### Responsibilities
- Enumerate and select audio devices
- Capture microphone input with low latency
- Play audio output with minimal jitter
- Handle audio format conversion
- Implement audio ducking

#### Technology
```rust
// Primary: rodio (high-level audio)
// Fallback: cpal (low-level control)
```

#### Key Types
```rust
pub struct AudioEngine {
    input_device: Device,
    output_device: Device,
    sample_rate: u32,        // 16000 or 24000 Hz
    channels: u16,          // Mono (1) for processing
    buffer_size: usize,     // Power of 2
}

pub enum AudioFormat {
    Pcm16,      // Standard for Whisper
    Float32,   // For processing
    Mulaw,     // For some codecs
}
```

#### Latency Optimization
- **Ring buffers**: Lock-free SPSC queues
- **Chunked processing**: Process in 512-1024 sample chunks
- **Device selection**: Prefer low-latency devices
- **Buffer tuning**: Balance latency vs stability

### 2. Voice Activity Detection (`thaleia-vad`)

Detects when user starts/finishes speaking.

#### Algorithm: Silero VAD
```rust
// Based on Silero VAD (https://github.com/snakers4/silero-vad)
// Optimized RNN for edge devices
pub struct VadEngine {
    model: SileroVad,
    threshold: f32,         // 0.5 default
    min_speech_duration_ms: u32,
    min_silence_duration_ms: u32,
}
```

#### State Machine
```
                    ┌─────────────┐
                    │    IDLE     │◀──────────────────┐
                    └──────┬──────┘                    │
                           │ (audio > threshold)       │
                           ▼                           │
                    ┌─────────────┐                    │
              ┌────▶│  SPEAKING   │────┐               │
              │     └─────────────┘    │               │
              │       │               │ (silence >     │
              │       │               │  min_silence)  │
              │       ▼               │               │
              │   ┌───────────┐       │               │
              └────│  ENDING   │───────┘               │
                  └───────────┘                       │
                         │                           │
                         │ (complete speech)          │
                         ▼                           │
                  ┌─────────────┐                    │
                  │   SEGMENT   │────────────────────┘
                  │  (to STT)   │
                  └─────────────┘
```

#### Performance Target
| Metric | Target |
|--------|--------|
| Detection Latency | <50ms |
| Accuracy | >95% |
| CPU Usage | <5% |

### 3. Speech-to-Text (`thaleia-stt`)

Converts audio segments to text.

#### Primary: Whisper.rs
```rust
// Based on whisper.cpp with Rust bindings
pub struct SttEngine {
    model: WhisperModel,
    language: Language,
    task: TranscriptionTask,  // Transcribe or Translate
}

pub struct TranscriptionConfig {
    pub language: Option<String>,     // "en" default
    pub max_segment_duration: f32,    // 30s max
    pub word_timestamps: bool,
    pub temperature: f32,
}
```

#### Model Variants
| Model | Parameters | Size | Speed | Accuracy |
|-------|------------|------|-------|----------|
| Whisper Tiny | 39M | ~75MB | 10x | Good |
| Whisper Base | 74M | ~150MB | 7x | Better |
| Whisper Small | 244M | ~500MB | 2.5x | Best |
| Whisper Large | 1550M | ~3GB | 1x | Excellent |

#### Streaming Optimization
```rust
// For real-time transcription, use partial results
pub struct StreamingTranscriber {
    // Process audio in chunks while speaking
    // Return interim results for faster feedback
}
```

#### Performance Target
| Metric | Target |
|--------|--------|
| Real-time Factor (RTF) | <0.25 (4x realtime) |
| Latency (E2E) | <200ms |
| Word Error Rate | <5% (English) |

### 4. Text-to-Speech (`thaleia-tts`)

Converts text to speech audio.

#### Primary: Kokoro-82M
```rust
// Based on kokoro-onnx for Rust
pub struct TtsEngine {
    model: KokoroModel,
    voices: VoiceRegistry,
    default_voice: VoiceId,
}

pub struct SynthesisRequest {
    pub text: String,
    pub voice: VoiceId,
    pub speed: f32,          // 0.5 to 2.0
    pub pitch: Option<f32>,
    pub emotion: Option<Emotion>,
}
```

#### Voice System
```rust
// Kokoro uses voice packs (AVG files)
pub struct Voice {
    pub id: VoiceId,
    pub name: String,          // "af_sky", "am_adam"
    pub pack_path: PathBuf,
    pub emotion_tags: Vec<Emotion>,
}

// Voice mixing: "af_sky.6+af_nicole.4"
pub fn blend_voices(v1: VoiceId, v2: VoiceId, ratio: (f32, f32)) -> Voice;
```

#### Emotion Mapping (Thaleia's Character)
```rust
// Thaleia maps emotions to appropriate voices
pub enum Emotion {
    Happy,     // af_bella, af_nicole
    Sad,       // af_heart
    Alert,     // am_adam, af_nicole
    Calm,      // af_sky, af_sarah
    Playful,   // af_bella (brightest)
}
```

#### Streaming Mode
```rust
// For low latency, stream chunks as generated
pub struct StreamingSynthesizer {
    model: KokoroModel,
    chunk_duration_ms: u32,  // 100ms chunks
}

impl Stream for StreamingSynthesizer {
    // Yields audio chunks as generated
    // Allows playback to start before full synthesis
}
```

#### Performance Target
| Metric | Target |
|--------|--------|
| Time-to-First-Audio (TTFA) | <50ms |
| Real-time Factor (RTF) | <0.1 (10x realtime) |
| MOS Score | >4.3 |

### 5. Session Manager (`thaleia-session`)

Manages conversation context and state.

```rust
pub struct Session {
    id: SessionId,
    history: Vec<Exchange>,
    metadata: SessionMetadata,
}

pub struct Exchange {
    user_audio: Option<Vec<u8>>,
    user_text: String,
    response_text: String,
    response_audio: Option<Vec<u8>>,
    timestamp: DateTime<Utc>,
    metadata: ExchangeMetadata,
}

pub struct SessionManager {
    sessions: RwLock<HashMap<SessionId, Session>>,
    max_history: usize,
}
```

#### Context Management
- Sliding window for recent exchanges
- Summarization for long conversations
- Configurable memory depth

---

## Data Flow

### Full Conversation Pipeline

```
User speaks
    │
    ▼
┌─────────────────┐
│   Microphone    │  16kHz, 16-bit mono PCM
│   Capture       │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│      VAD        │  Detect speech boundaries
│   (Silero)      │  Output: Speech segments
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│      STT        │  Whisper transcription
│   (Whisper)     │  Output: Text
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│      LLM        │  Process via MCP or plugin
│   (External)    │  Output: Response text
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│      TTS        │  Kokoro synthesis
│   (Kokoro)      │  Output: Audio chunks
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│   Speaker Out   │  16kHz, 16-bit mono PCM
│                 │  Optional: audio ducking
└─────────────────┘
```

### Streaming Pipeline (Optimized)

```
User speaks (continuous)           Response audio (continuous)
         │                                  ▲
         ▼                                  │
┌─────────────────┐                        │
│   Ring Buffer   │  Circular audio buffer │
│   (5 seconds)   │                        │
└────────┬────────┘                        │
         │                                 │
         ▼                                 │
┌─────────────────┐    Parallel    ┌───────┴───────┐
│   VAD Streaming │─────────────────▶│  Segment     │
│   (continuous)  │                  │  Detection   │
└─────────────────┘                  └───────┬─────┘
                                              │
                                              ▼
                                      ┌───────────────┐
                                      │  Batch to STT │
                                      │  (every 5-15s)│
                                      └───────┬───────┘
                                              │
                                              ▼
                                      ┌───────────────┐
                                      │  LLM Request  │
                                      └───────┬───────┘
                                              │
                                              ▼
                                      ┌───────────────┐
                                      │ Stream to TTS │
                                      │ (token-by-token)│
                                      └───────┬───────┘
                                              │
                                              │
──────────────────────────────────────────────┘
              Continuous response streaming
```

---

## Audio Pipeline

### Input Path (Recording)

```rust
// Simplified audio capture
async fn capture_audio(ctx: &AudioContext) -> Result<AudioChunk> {
    // 1. Select microphone
    let device = ctx.default_input_device()?;
    
    // 2. Create input stream
    let stream = device.build_input_stream(&Config {
        sample_rate: 16000,
        channels: 1,
        format: SampleFormat::I16,
    })?;
    
    // 3. Collect audio
    let buffer = Arc::new(AtomicBuffer::new(16000 * 10)); // 10s buffer
    stream.play();
    
    // 4. Return chunk
    Ok(AudioChunk { data: buffer, sample_rate: 16000 })
}
```

### Output Path (Playback)

```rust
// Simplified audio playback
async fn play_audio(ctx: &AudioContext, chunk: &AudioChunk) -> Result<()> {
    // 1. Select speaker
    let device = ctx.default_output_device()?;
    
    // 2. Create output stream
    let stream = device.build_output_stream(&Config {
        sample_rate: 24000,  // Kokoro native rate
        channels: 1,
        format: SampleFormat::I16,
    })?;
    
    // 3. Play with optional ducking
    if config.duck_audio {
        lower_other_audio();
    }
    
    stream.write(chunk.data())?;
    stream.play()
}
```

### Audio Ducking

When Thaleia speaks, lower other audio:

```rust
pub fn duck_audio(&self) {
    // Lower system volume to 20%
    // This prevents audio from other apps from competing
    // User can configure threshold
}

pub fn unduck_audio(&self) {
    // Restore previous volume
}
```

---

## Plugin Architecture

Thaleia uses a **plugin system** for extensibility:

```
┌─────────────────────────────────────────────────────────────────┐
│                      THALEIA CORE                                │
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │                    Plugin Interface                          │ │
│  │                                                             │ │
│  │  trait SttPlugin { ... }  // Swap Whisper for anything   │ │
│  │  trait TtsPlugin { ... }  // Swap Kokoro for anything    │ │
│  │  trait LlmPlugin { ... }  // Add new LLM providers        │ │
│  │  trait VadPlugin { ... }  // Swap Silero for anything     │ │
│  │                                                             │ │
│  └─────────────────────────────────────────────────────────────┘ │
│                              │                                   │
│         ┌────────────────────┼────────────────────┐            │
│         ▼                    ▼                    ▼            │
│  ┌─────────────┐      ┌─────────────┐      ┌─────────────┐   │
│  │ whisper.rs  │      │  kokoro.rs  │      │  silero.rs   │   │
│  │  (builtin)  │      │  (builtin)  │      │  (builtin)   │   │
│  └─────────────┘      └─────────────┘      └─────────────┘   │
│                              │                                   │
│         ┌────────────────────┼────────────────────┐            │
│         ▼                    ▼                    ▼            │
│  ┌─────────────┐      ┌─────────────┐      ┌─────────────┐   │
│  │ faster-whis │      │ fish-speech │      │  webRTC VAD  │   │
│  │  (plugin)   │      │   (plugin)  │      │   (plugin)   │   │
│  └─────────────┘      └─────────────┘      └─────────────┘   │
└─────────────────────────────────────────────────────────────────┘
```

### Plugin Trait Definitions

```rust
/// Speech-to-Text Plugin
pub trait SttPlugin: Send + Sync {
    fn name(&self) -> &str;
    
    async fn transcribe(
        &self,
        audio: &[i16],
        config: &SttConfig,
    ) -> Result<Transcription, SttError>;
    
    async fn transcribe_streaming(
        &self,
        audio_stream: Pin<Box<dyn Stream<Item = Vec<i16>> + Send>>,
        config: &SttConfig,
    ) -> Result<Pin<Box<dyn Stream<Item = TranscriptionChunk> + Send>>, SttError>;
    
    fn supported_languages(&self) -> Vec<&str>;
}

/// Text-to-Speech Plugin
pub trait TtsPlugin: Send + Sync {
    fn name(&self) -> &str;
    
    async fn synthesize(
        &self,
        text: &str,
        config: &TtsConfig,
    ) -> Result<AudioData, TtsError>;
    
    async fn synthesize_streaming(
        &self,
        text: &str,
        config: &TtsConfig,
    ) -> Result<Pin<Box<dyn Stream<Item = AudioChunk> + Send + '_>>, TtsError>;
    
    fn list_voices(&self) -> Vec<Voice>;
    
    fn default_voice(&self) -> VoiceId;
}

/// Language Model Plugin
pub trait LlmPlugin: Send + Sync {
    fn name(&self) -> &str;
    
    async fn generate(
        &self,
        prompt: &str,
        context: &[Exchange],
        config: &LlmConfig,
    ) -> Result<String, LlmError>;
    
    async fn generate_streaming(
        &self,
        prompt: &str,
        context: &[Exchange],
        config: &LlmConfig,
    ) -> Result<Pin<Box<dyn Stream<Item = String> + Send>>, LlmError>;
}
```

### Plugin Loading

```rust
// Load plugins from ~/.config/thaleia/plugins/
pub fn load_plugins() -> Result<PluginRegistry> {
    let plugin_dir = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("thaleia")
        .join("plugins");
    
    let registry = PluginRegistry::new();
    
    for entry in fs::read_dir(plugin_dir)? {
        let path = entry?.path();
        if path.extension() == Some("thaleia-plugin".into()) {
            registry.load(path)?;
        }
    }
    
    Ok(registry)
}
```

---

## MCP Server Design

### Protocol Implementation

Thaleia implements the **Model Context Protocol** for AI tool integration:

```rust
// MCP JSON-RPC message handling
pub struct McpServer {
    transport: StdioTransport,
    handlers: HashMap<String, ToolHandler>,
    session: SessionManager,
}

impl McpServer {
    pub async fn run(&self) -> Result<()> {
        // Read JSON-RPC requests from stdin
        // Write responses to stdout
        loop {
            let request = self.transport.read_request().await?;
            let response = self.handle(request).await?;
            self.transport.write_response(response).await?;
        }
    }
}
```

### MCP Tools

```json
{
  "tools": [
    {
      "name": "speak",
      "description": "Convert text to speech and play audio. Use this to have Thaleia speak.",
      "inputSchema": {
        "type": "object",
        "properties": {
          "text": {
            "type": "string",
            "description": "The text to speak"
          },
          "voice": {
            "type": "string",
            "description": "Voice ID (default: af_sky)",
            "default": "af_sky"
          },
          "speed": {
            "type": "number",
            "description": "Speech speed 0.5-2.0",
            "default": 1.0
          }
        },
        "required": ["text"]
      }
    },
    {
      "name": "listen",
      "description": "Capture audio from microphone and transcribe to text",
      "inputSchema": {
        "type": "object",
        "properties": {
          "timeout": {
            "type": "number",
            "description": "Max seconds to listen (default: 10)",
            "default": 10
          },
          "language": {
            "type": "string",
            "description": "Language code (default: en)",
            "default": "en"
          }
        }
      }
    },
    {
      "name": "list_voices",
      "description": "List all available TTS voices"
    },
    {
      "name": "set_voice",
      "description": "Set the default voice for Thaleia",
      "inputSchema": {
        "type": "object",
        "properties": {
          "voice": {
            "type": "string",
            "description": "Voice ID to set as default"
          }
        },
        "required": ["voice"]
      }
    }
  ]
}
```

### Session Handling

```rust
// Each MCP client gets a session
pub struct ClientSession {
    id: Uuid,
    history: Vec<Exchange>,
    default_voice: VoiceId,
}

impl ClientSession {
    pub async fn handle_tool_call(
        &mut self,
        tool: &str,
        params: Value,
    ) -> Result<Value, McpError> {
        match tool {
            "speak" => self.speak(params).await,
            "listen" => self.listen(params).await,
            "list_voices" => self.list_voices().await,
            _ => Err(McpError::UnknownTool(tool.into())),
        }
    }
}
```

---

## Concurrency Model

Thaleia uses **async/await** with Tokio for concurrency:

```rust
#[tokio::main]
async fn main() -> Result<()> {
    // Initialize components
    let audio = AudioEngine::new()?;
    let stt = SttEngine::new()?;
    let tts = TtsEngine::new()?;
    let session = SessionManager::new();
    
    // Run MCP server or CLI
    match config.mode {
        Mode::Mcp => run_mcp_server(audio, stt, tts, session).await,
        Mode::Cli => run_cli(audio, stt, tts, session).await,
    }
}
```

### Task Spawning

```rust
// Main conversation loop
pub async fn conversation_loop(
    audio: Arc<AudioEngine>,
    stt: Arc<SttEngine>,
    tts: Arc<TtsEngine>,
    mut session: SessionManager,
) -> Result<()> {
    loop {
        // 1. Listen for speech (VAD)
        let speech = audio.listen_until_speech().await?;
        
        // 2. Transcribe
        let text = stt.transcribe(&speech).await?;
        
        // 3. Process with LLM (via MCP or plugin)
        let response = process_with_llm(&text, &session).await?;
        
        // 4. Speak response
        let audio = tts.synthesize(&response).await?;
        audio.play().await?;
        
        // 5. Update session
        session.add_exchange(text, response);
    }
}
```

### Channel Communication

```rust
// For streaming, use channels
let (tx, rx) = mpsc::channel::<AudioChunk>(32);

// Producer: TTS generates chunks
let producer = {
    let tx = tx.clone();
    async move {
        tts.synthesize_streaming(text).await
    }
};

// Consumer: Play chunks as they arrive
let consumer = async move {
    while let Some(chunk) = rx.recv().await {
        audio.play_chunk(chunk).await?;
    }
};

tokio::join!(producer, consumer);
```

### Async/Sync Bridge Pattern

For MCP servers, audio resources (rodio, SDL2) are not `Send + Sync`.
Use a dedicated audio thread with channels to bridge async MCP with sync audio:

```rust
// Channel types for async ↔ sync bridge
pub struct AudioThreadManager {
    /// Sender for TTS requests (TTS engine is NOT Send)
    tts_tx: mpsc::Sender<TtsRequest>,
    /// Sender for STT requests (STT engine is NOT Send)
    stt_tx: mpsc::Sender<SttRequest>,
    /// Handle to the audio thread
    handle: JoinHandle<()>,
}

impl AudioThreadManager {
    /// Spawn a dedicated thread that owns audio resources
    pub fn new() -> Self {
        let (tts_tx, tts_rx) = mpsc::channel(1);
        let (stt_tx, stt_rx) = mpsc::channel(1);
        
        let handle = std::thread::spawn(move || {
            // This thread owns: TtsEngine, WhisperEngine, AudioSystem
            // None of these are Send, so they stay here
            let mut tts_engine = block_on(TtsEngine::new()).unwrap();
            let mut stt_engine = block_on(WhisperEngine::new()).unwrap();
            let mut audio = AudioSystem::new();
            
            loop {
                tokio::select! {
                    Some(req) = tts_rx.recv() => {
                        let samples = tts_engine.synthesize(&req.text, req.voice.as_deref()).unwrap();
                        audio.play(&samples, 24000).unwrap();
                        let _ = req.result_tx.send(Ok(samples));
                    }
                    Some(req) = stt_rx.recv() => {
                        let captured = audio.capture(req.duration).unwrap();
                        let text = stt_engine.transcribe(&captured.samples, captured.sample_rate).unwrap();
                        let _ = req.result_tx.send(Ok(text));
                    }
                    else => break,
                }
            }
        });
        
        Self { tts_tx, stt_tx, handle }
    }
}
```

---

## Error Handling

### Error Types

```rust
#[derive(Error, Debug)]
pub enum ThaleiaError {
    #[error("Audio device error: {0}")]
    AudioDevice(#[from] AudioDeviceError),
    
    #[error("STT processing error: {0}")]
    SttProcessing(String),
    
    #[error("TTS synthesis error: {0}")]
    TtsSynthesis(String),
    
    #[error("LLM communication error: {0}")]
    LlmCommunication(String),
    
    #[error("MCP protocol error: {0}")]
    McpProtocol(String),
    
    #[error("Plugin error: {0}")]
    Plugin(String),
    
    #[error("Configuration error: {0}")]
    Config(String),
}
```

### Recovery Strategies

```rust
impl ThaleiaError {
    pub fn is_recoverable(&self) -> bool {
        match self {
            ThaleiaError::AudioDevice(_) => true,
            ThaleiaError::SttProcessing(_) => true,
            ThaleiaError::TtsSynthesis(_) => true,
            ThaleiaError::LlmCommunication(_) => true,
            _ => false,
        }
    }
    
    pub fn user_message(&self) -> String {
        match self {
            ThaleiaError::AudioDevice(_) => 
                "I had trouble with audio. Is my microphone working?".into(),
            ThaleiaError::SttProcessing(_) => 
                "I didn't quite catch that. Could you repeat?".into(),
            ThaleiaError::TtsSynthesis(_) => 
                "I had trouble finding the right words. Let me try again!".into(),
            ThaleiaError::LlmCommunication(_) => 
                "I'm thinking... Just a moment!".into(),
            _ => "Oops! Something went wrong, but I'm still here!".into(),
        }
    }
}
```

---

## Configuration

### Config File (`~/.config/thaleia/config.toml`)

```toml
[thaleia]
name = "Thaleia"
version = "0.1.0"

[audio]
input_device = "default"
output_device = "default"
sample_rate = 16000
buffer_size = 512

[stt]
model = "whisper-base"
language = "en"
device = "cpu"  # cpu, cuda, metal

[tts]
model = "kokoro"
default_voice = "af_sky"
default_speed = 1.0

[vad]
enabled = true
threshold = 0.5
min_speech_ms = 250
min_silence_ms = 500

[llm]
provider = "mcp"  # or "ollama", "openai"
model = "claude"

[mcp]
server = "stdio"  # or "websocket"
```

### Environment Variables

```bash
# Override config
THALEA_CONFIG=/path/to/config.toml
THALEA_MODEL_DIR=/path/to/models
THALEA_LOG_LEVEL=debug

# Disable features
THALEA_NO_VAD=1
THALEA_NO_MCP=1
```

---

## Project Structure

```
thaleia/
├── Cargo.toml              # Workspace root
├── src/
│   ├── main.rs             # Entry point
│   ├── lib.rs              # Library interface
│   ├── cli.rs              # CLI interface
│   ├── mcp.rs              # MCP server
│   │
│   ├── audio/              # Audio engine
│   │   ├── mod.rs
│   │   ├── capture.rs
│   │   ├── playback.rs
│   │   └── ducking.rs
│   │
│   ├── stt/                # Speech-to-text
│   │   ├── mod.rs
│   │   ├── whisper.rs
│   │   └── streaming.rs
│   │
│   ├── tts/                # Text-to-speech
│   │   ├── mod.rs
│   │   ├── kokoro.rs
│   │   ├── voices.rs
│   │   └── streaming.rs
│   │
│   ├── vad/                # Voice activity detection
│   │   ├── mod.rs
│   │   └── silero.rs
│   │
│   ├── session/             # Conversation management
│   │   ├── mod.rs
│   │   └── history.rs
│   │
│   ├── plugin/              # Plugin system
│   │   ├── mod.rs
│   │   └── registry.rs
│   │
│   └── error.rs             # Error types
│
├── crates/
│   ├── thaleia-core/       # Core library
│   ├── thaleia-cli/        # CLI application
│   ├── thaleia-mcp/        # MCP server
│   ├── thaleia-plugin-whisper/  # Whisper plugin
│   └── thaleia-plugin-kokoro/   # Kokoro plugin
│
├── models/                 # ONNX models
│   ├── kokoro/
│   └── silero/
│
├── voices/                 # Voice packs
│
├── config/                 # Default configs
│
├── docs/                   # Documentation
│   ├── ARCHITECTURE.md
│   ├── RESEARCH.md
│   └── COMPARISON.md
│
└── tests/                  # Integration tests
```

---

## Appendix: Performance Budget

For end-to-end latency under 500ms:

| Component | Budget | Optimization |
|-----------|--------|--------------|
| Audio capture | 50ms | Ring buffer, 16kHz |
| VAD detection | 30ms | Silero optimized |
| STT processing | 150ms | Whisper base model |
| LLM inference | 100ms | Streaming response |
| TTS synthesis | 100ms | Kokoro streaming |
| Audio output | 50ms | Pre-buffered chunks |
| **Total** | **~480ms** | |

---

*"Architecture is the art of making complexity feel simple. Thaleia makes voice AI feel effortless."*
