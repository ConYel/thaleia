# Thaleia Implementation Plan

> *"From vision to reality: A step-by-step guide to building the best voice AI"*

---

## Documentation Guide

| Document | Purpose |
|-----------|---------|
| [README.md](./README.md) | Overview, quick start, features |
| [PLAN.md](./PLAN.md) | **This file** - Roadmap, progress, tasks |
| [ARCHITECTURE.md](./ARCHITECTURE.md) | **Target architecture** - Where we're heading |
| [CONTRIBUTING.md](./CONTRIBUTING.md) | Contribution guidelines |

---

## Thaleia Product Definition (2026-03-29)

### What Thaleia Is

**Thaleia = Voice Interface (Ears + Mouth)**
- Provides STT (Speech-to-Text) via MCP
- Provides TTS (Text-to-Speech) via MCP
- Any MCP-compatible LLM can use Thaleia as voice interface

```
┌─────────────────────────────────────────────────────────────┐
│                      Thaleia (Product)                      │
├─────────────────────────────────────────────────────────────┤
│  Role: Voice Interface (Ears + Mouth)                     │
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │  MCP Server                                          │   │
│  │  - thaleia_listen (STT: audio → text)              │   │
│  │  - thaleia_speak (TTS: text → audio)               │   │
│  │  - thaleia_list_voices                             │   │
│  └─────────────────────────────────────────────────────┘   │
│                           ▲                               │
│                           │ MCP Protocol                  │
│                           │                               │
│  ┌─────────────────────────────────────────────────────┐   │
│  │  CLI (for development/testing)                      │   │
│  │  - thaleia listen     → tests STT                  │   │
│  │  - thaleia speak     → tests TTS                   │   │
│  │  - thaleia pipeline  → end-to-end test             │   │
│  └─────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
                           ▲
                           │ (connects via MCP)
                           │
              ┌────────────┴────────────┐
              │   LLM (The "Brain")    │
              │  - Claude Desktop      │
              │  - OpenCode            │
              │  - Any MCP client     │
              └─────────────────────────┘
```

### What Thaleia Is NOT

- ❌ Not a standalone assistant with its own "brain"
- ❌ Does NOT call LLMs directly
- ❌ Does NOT have LLM backends

---

## Strategic Roadmap (2026-03-30)

### Why This Matters: Performance Analysis

For a **voice AI product**, MCP server throughput (4,845 RPS vs 3,000 RPS) is **irrelevant** because:

| Component | Time per Request | Comparison |
|-----------|-----------------|------------|
| Audio capture | 1-10 seconds | User speaking |
| STT (Whisper) | 0.5-3 seconds | CPU intensive |
| LLM response | 1-10+ seconds | Depends on complexity |
| TTS (Kokoro) | 0.5-2 seconds | Audio generation |
| **MCP transport** | **<1ms** | **Negligible** |

**The MCP transport is never the bottleneck for voice AI.**

### What Actually Makes Thaleia the "Best"

| Priority | Factor | Why It Matters |
|----------|--------|----------------|
| **1** | Voice Quality | STT accuracy, TTS naturalness |
| **2** | Latency | End-to-end <2 seconds |
| **3** | Natural Conversation | Wake word, VAD, interruption |
| **4** | Platform Support | Works on Qubes, Linux, laptops |
| **5** | Reliability | Doesn't crash, handles errors |
| **6** | MCP Transport | Works, supports stdio + HTTP |

### Implementation Phases

```
Phase 1: Production-Ready Foundation
├── ✅ Audio capture - SDL2 + Rodio backends working
├── ✅ STT (Whisper) - Working
├── ✅ TTS (Kokoro) - Working
├── 🔄 MCP Server - Using rmcp v1.3 (stdio working, HTTP pending)
├── 🔄 Audio Source Selection - Implemented but needs verification
├── 🔄 Debug System - In progress (needs testing)
└── 🔄 HTTP Transport - NOT implemented

Phase 2: Natural Conversation (Most Important for Users)
├── 🔄 VAD Integration - NOT connected to pipeline
├── 🔄 Wake Word - NOT implemented
├── 🔄 Interruption Handling - NOT implemented

Phase 3: Polish & Performance
├── 🔄 Streaming TTS - NOT implemented
├── 🔄 TTS Audio Ordering - Audio plays out of order for long text (investigation needed)
├── 🔄 Model Improvements - NOT implemented
└── 🔄 Platform Support - NOT implemented

Phase 4: Best Product
├── 🔄 Multiple Languages - NOT implemented
├── 🔄 Voice Customization - NOT implemented
└── 🔄 Enterprise Features - NOT implemented
```

### MCP Implementation Decision

**Implemented with rmcp v1.3**

Status:
- stdio transport: Working (compiles, needs full integration test)
- Tools: listen, speak, list_voices, get_status, list_sources (compiles)
- Audio source selection: Implemented (source parameter + list_sources tool) - needs verification
- Debug system: Started - tracing + thaleia_debug macro implemented, needs testing
- HTTP transport: NOT implemented

```rust
// Critical config for production
StreamableHttpServerConfig {
    stateful_mode: false,
    json_response: true,  // 3.8x performance improvement!
    ..Default::default()
}
```

### Two Usage Modes

| Mode | How it works |
|------|--------------|
| **Production** | User runs LLM client with MCP - connects to Thaleia MCP server |
| **Development** | `make pipeline` - CLI tests end-to-end with OpenCode CLI |

### Pipeline End-to-End Flow (Development/Testing)

```
make pipeline
    │
    ▼
┌──────────────────────────────────────────────────────┐
│  Thaleia CLI Pipeline (Dev Mode)                     │
│  1. Spawn MCP server as subprocess                   │
│  2. Call thaleia_listen → capture + STT → text    │
│  3. Receive transcription                           │
│  4. Call OpenCode CLI → get LLM response           │
│  5. Call thaleia_speak → TTS + playback          │
│  6. Verify all steps succeeded                      │
└──────────────────────────────────────────────────────┘
```

### Verification Steps

| Step | Verify |
|------|--------|
| Audio capture | samples > 0 |
| STT transcription | text not empty |
| OpenCode response | response not empty |
| TTS generation | audio samples > 0 |
| Audio playback | no error from audio system |

### ✅ COMPLETED
| Component | Status | Notes |
|-----------|--------|-------|
| Kokoro TTS | ✅ Working | rodio 0.22 playback |
| Whisper STT | ✅ Working | rubato resampling |
| **Pluggable STT** | ✅ **NEW** | Enum + factory pattern |
| MCP Server | ✅ Working | Green session history |
| Audio Capture | ✅ Working | rodio microphone |
| Audio Playback | ✅ Working | **Buffered only** |
| Container Audio | ✅ Working | PipeWire/PulseAudio |

### 🔄 IN PROGRESS
| Component | Status | Notes |
|-----------|--------|-------|
| Dialogue Manager | 🔄 Week 1 | State machine for conversation |
| MCP Audio Source Auto-Detection | 🔄 Week 1 | Fix microphone capture issue |

### Current Issue: MCP Microphone Not Working

**Symptom**: CLI captures audio correctly (217k samples), but MCP returns no response

**Root Cause**: MCP audio thread uses different audio source than CLI (system audio vs microphone)

**Solution**: Auto-detect available audio source and use first available input

#### Tasks:
- [ ] Phase 1: Auto-detect audio source in capture module
- [ ] Phase 2: Add `source` parameter to `thaleia_listen` tool
- [ ] Phase 3: Add `thaleia_list_sources` tool for debugging
- [ ] Phase 4: Update `thaleia_get_status` with source info
| Interruption Handling | 🔄 Week 1 | User can stop Thaleia anytime |
| Pipeline Modes | 🔄 Week 1 | Wake-word, VAD, push-to-talk |
| Wake Word | 🔄 Week 2 | Multiple wake words with Silero |
| VAD Integration | 🔄 Week 3 | Connect VAD to pipeline |

### ⚠️ KNOWN LIMITATIONS
| Limitation | Impact | Solution |
|------------|--------|----------|
| No streaming playback | High latency | Phase 5.1 |
| No pipeline system | Can't configure modes | Phase 4 (in progress) |
| No wake word | Always listening | Phase 4.2 (in progress) |
| No interruption handling | Can't stop Thaleia | Phase 4.1 (in progress) |

### 🎯 JUST COMPLETED: Pluggable STT Architecture
- Enhanced `SttBackend` trait with metadata methods
- Domain types: `BackendName`, `LanguageCode`, `SttInfo`, `Transcription`
- `SttSystem` factory (enum + factory pattern)
- Matches Audio module architecture

> ⭐ **Benchmark Reminder**: When Thaleia becomes production-ready, we should benchmark all ML components (VAD, Wake Word, STT, TTS) using scientific methodology.*

---

## Table of Contents

1. [Phase Overview](#phase-overview)
2. [Phase 0: Foundation](#phase-0-foundation)
3. [Phase 1: TTS Engine](#phase-1-tts-engine)
4. [Phase 2: STT Engine](#phase-2-stt-engine)
5. [Phase 3: MCP Server](#phase-3-mcp-server)
6. [Phase 4: VAD + Wake Word](#phase-4-vad--wake-word)
7. [Phase 5: Polish + Streaming](#phase-5-polish--streaming)
8. [Phase 6: Community + Growth](#phase-6-community--growth)
9. [Task Breakdown](#task-breakdown)
10. [Milestones](#milestones)

---

## Phase Overview

```
┌─────────────────────────────────────────────────────────────────────┐
│                        THALEIA ROADMAP                              │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  Phase 0: Foundation (Week 1-2)                                    │
│  ├── Project setup                                                 │
│  ├── Workspace structure                                           │
│  └── Audio engine basics                                           │
│                                                                     │
│  Phase 1: TTS Engine (Week 2-4)                                     │
│  ├── Kokoro integration                                            │
│  ├── Voice system                                                  │
│  └── CLI speak command                                             │
│                                                                     │
│  Phase 2: STT Engine (Week 4-6)                                    │
│  ├── Whisper integration                                           │
│  ├── Streaming transcription                                       │
│  └── CLI listen command                                            │
│                                                                     │
│  Phase 3: MCP Server (Week 6-8)                                    │
│  ├── MCP protocol implementation ✅                                  │
│  ├── Tool definitions ✅                                             │
│  ├── OpenCode integration ✅                                         │
│  └── Async/Sync bridge (channel-based audio thread) ✅              │
│                                                                     │
│  Phase 4: VAD + Wake Word (Week 8-10)                              │
│  ├── Silero VAD integration                                        │
│  ├── Wake word detection                                           │
│  └── Natural conversation flow                                     │
│                                                                     │
│  Phase 5: Polish + Streaming (Week 10-12)                          │
│  ├── Streaming synthesis                                           │
│  ├── Barge-in support                                              │
│  └── Performance optimization                                      │
│                                                                     │
│  Phase 6: Community + Growth (Ongoing)                             │
│  ├── Documentation                                                 │
│  ├── Plugin system                                                 │
│  └── Community building                                           │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Phase 0: Foundation

**Goal**: Set up the project structure and core audio handling

**Duration**: 2 weeks

### Tasks

#### 0.1 Project Setup
```bash
# Create Rust workspace
cargo new thaleia --workspace
cd thaleia

# Add crates
cargo add tokio --features full
cargo add anyhow
cargo add tracing
cargo add config
```

#### 0.2 Workspace Structure
```
thaleia/
├── Cargo.toml (workspace)
├── crates/
│   ├── thaleia-core/      # Core library
│   ├── thaleia-cli/       # CLI application
│   └── thaleia-mcp/       # MCP server
├── src/
│   └── main.rs
└── tests/
```

#### 0.3 Audio Engine
```rust
// thaleia-core/src/audio/mod.rs
pub struct AudioEngine {
    // Device enumeration
    // Input stream setup
    // Output stream setup
    // Format conversion
}
```

#### 0.4 Error Handling
```rust
// thaleia-core/src/error.rs
pub enum ThaleiaError {
    AudioDevice(String),
    SttError(String),
    TtsError(String),
    McpError(String),
}
```

#### Deliverables
- [ ] Working Rust workspace
- [ ] Audio device enumeration
- [ ] Basic audio capture/playback
- [ ] Error types defined

---

## Phase 1: TTS Engine

**Goal**: Get Thaleia speaking with Kokoro

**Duration**: 2 weeks

### Current Implementation Status

#### ✅ COMPLETED: Kokoro TTS
- Kokoro ONNX inference working
- 16 built-in voices
- `thaleia speak` command working
- rodio 0.22 audio playback

#### ⚠️ LIMITATION: No True Streaming
**Current behavior**: Buffered playback (not streaming)

```
User types text → Wait for full synthesis → Play entire audio
```

**How it works now**:
1. `AudioPlayer::play_samples()` synthesizes complete audio
2. Writes to `/tmp/thaleia_playback.wav`
3. Plays entire file via rodio

**Why not streaming?**
- Rodio doesn't stream well from memory
- Kokoro chunk yields not utilized
- User must wait for full synthesis before hearing anything

**What true streaming would enable**:
- Time to First Audio (TTFA) < 100ms
- Play chunks as they're generated
- Feel more responsive, like natural conversation

**See Phase 5.1 for streaming implementation plan.**

### Tasks

#### 1.1 Kokoro Integration
```rust
// thaleia-tts/src/kokoro.rs
pub struct KokoroEngine {
    model: InferenceSession,
    voices: VoiceRegistry,
}

impl KokoroEngine {
    pub async fn synthesize(&self, text: &str, voice: &Voice) -> Result<Audio> {
        // Tokenize text
        // Run ONNX inference
        // Generate audio
    }
}
```

#### 1.2 Voice System
```rust
// Voice management
pub struct Voice {
    id: String,
    name: String,
    pack_path: PathBuf,
}

pub fn list_available_voices() -> Vec<Voice> {
    // Scan voice pack directory
    // Return voice metadata
}
```

#### 1.3 CLI Speak Command
```bash
thaleia speak "Hello! I'm Thaleia!"
thaleia speak --voice af_bella "Happy to meet you!"
thaleia speak --speed 1.2 "Quick response!"
```

#### 1.4 Voice Character Mapping
```rust
// Thaleia's personality in voice selection
impl TtsEngine {
    fn select_voice_for_mood(&self, mood: &str) -> Voice {
        match mood {
            "happy" => self.voices.get("af_bella"),
            "calm" => self.voices.get("af_sky"),
            "playful" => self.voices.get("af_nicole"),
            _ => self.voices.get("af_sky"),
        }
    }
}
```

#### Deliverables
- [ ] Kokoro ONNX inference working
- [ ] Voice listing command
- [ ] `thaleia speak` command
- [ ] Voice selection by ID

---

## Phase 2: STT Engine

**Goal**: Add Whisper-based speech recognition

**Duration**: 2 weeks

### Current Implementation Status

#### ✅ COMPLETED: Pluggable STT Architecture
- `SttBackend` trait with metadata (info, languages, streaming support)
- Domain types: `BackendName`, `LanguageCode`, `SttInfo`, `Transcription`
- `SttSystem` factory (enum + factory pattern)
- Consistent with Audio module architecture
- Future-ready for additional backends (Qwen3-ASR)

### Tasks

#### 2.1 Whisper Integration
```rust
// thaleia-stt/src/whisper.rs
pub struct WhisperEngine {
    model: WhisperContext,
    params: FullParams,
}

impl WhisperEngine {
    pub async fn transcribe(&self, audio: &[i16]) -> Result<String> {
        // Convert to f32
        // Run Whisper inference
        // Extract text
    }
}
```

#### 2.2 Streaming Transcription
```rust
// For real-time use
pub struct StreamingWhisper {
    model: WhisperContext,
    buffer: RingBuffer,
}

impl StreamingWhisper {
    // Process audio chunks as they arrive
    // Return partial results
}
```

#### 2.3 CLI Listen Command
```bash
thaleia listen
thaleia listen --timeout 30
thaleia listen --language en
```

#### 2.4 Audio Preprocessing
```rust
// Resample to 16kHz
// Normalize volume
// Remove silence
pub fn preprocess_audio(raw: &[i16]) -> Vec<i16> {
    let resampled = resample_48_to_16(raw);
    let normalized = normalize(resampled);
    let trimmed = trim_silence(normalized);
    trimmed
}
```

#### Deliverables
- [x] Whisper transcription working ✅
- [x] `thaleia listen` command ✅ (CLI listen command in commands.rs)
- [x] Audio preprocessing pipeline (resampling) ✅ (resample_to_16khz in whisper.rs)
- [ ] Streaming transcription (future enhancement)

---

## Phase 3: MCP Server

**Goal**: Connect Thaleia to Claude and other AI tools

**Duration**: 2 weeks

### ✅ COMPLETED
- MCP server running with rmcp SDK
- Tools: speak, listen, list_voices, get_status
- OpenCode integration tested
- **Async/Sync Bridge**: Channel-based audio thread (fixes SDL2 non-Send issue)

### 🔄 IN PROGRESS: Audio Source Auto-Detection

**Status (April 5, 2026)**:
- Fixed MCP to use existing audio_system (same as CLI)
- Added `source` parameter to `thaleia_listen` tool
- Added `thaleia_list_sources` tool
- **Needs verification**: Test that MCP microphone capture works end-to-end

### 🔄 IN PROGRESS: Unified Debug System (April 6, 2026)

**Problem**: Development debugging used ~40 raw `eprintln!` statements scattered throughout code.

**Started**:
- Created `thaleia-core/src/debug.rs` with debug utilities
- Added `tracing-subscriber` dependency
- Updated entry points with `--debug` flag support
- Replaced most `eprintln!` with `tracing` and `thaleia_debug!` macros

**Remaining**:
- Full end-to-end testing of debug output
- Verify `thaleia --debug listen` works
- Verify `thaleia-mcp --debug` works

**Problem Identified (2026-04-05)**:
- CLI `thaleia listen` captures correctly (217k samples from microphone)
- MCP `thaleia_listen` returns no response (capture silently fails)
- Debug shows SDL2 backend is selected but capture returns empty/wrong audio
- Root cause: PulseAudio default source may be system audio, not microphone

**Solution - Auto-Detect with User Override**:
1. **Auto-detect**: Use first available audio input as default (not PulseAudio's implicit default)
2. **Explicit source**: Allow user to specify source via parameter
3. **List sources**: Add tool to discover available sources

**Implementation**:
- [x] Added debug logging to audio_manager.rs
- [x] Use existing audio_system (fixed backend selection)
- [x] Add `source` parameter to `thaleia_listen` 
- [x] Add `thaleia_list_sources` tool
- [x] Update `thaleia_get_status` with source info
- [x] DEBUG: Unified debug system implemented (April 6, 2026)

### Tasks

#### 3.1 MCP Protocol Implementation
```rust
// thaleia-mcp/src/server.rs
pub struct McpServer {
    transport: StdioTransport,
    handlers: HashMap<String, ToolHandler>,
}

#[async_trait]
impl McpServer {
    async fn handle_request(&self, req: Request) -> Response {
        match req.method {
            "tools/list" => self.list_tools(),
            "tools/call" => self.call_tool(req.params),
            _ => Response::error("Method not found"),
        }
    }
}
```

#### 3.2 Tool Definitions
```json
{
  "tools": [
    {
      "name": "speak",
      "description": "Have Thaleia speak text aloud",
      "inputSchema": {
        "type": "object",
        "properties": {
          "text": { "type": "string" },
          "voice": { "type": "string" },
          "speed": { "type": "number" }
        }
      }
    },
    {
      "name": "listen",
      "description": "Listen for speech and transcribe",
      "inputSchema": {
        "type": "object",
        "properties": {
          "timeout": { "type": "number" }
        }
      }
    },
    {
      "name": "list_voices",
      "description": "List available voices"
    }
  ]
}
```

#### 3.3 Claude Desktop Integration
```bash
# Claude Desktop config
# ~/.claude/settings.json or claude_desktop_config.json
{
  "mcpServers": {
    "thaleia": {
      "command": "thaleia",
      "args": ["mcp", "--stdio"]
    }
  }
}
```

#### 3.4 Testing with Claude
```
User: "Ask Thaleia to introduce herself"
Claude: *calls thaleia speak tool*
Thaleia: "Hello! I'm Thaleia, the joyful Muse..."
```

#### Deliverables
- [ ] MCP server running
- [ ] All tools implemented
- [ ] Claude Desktop integration tested
- [ ] Documentation for MCP setup

---

## Phase 4: VAD + Pipeline + Wake Word + Interruption

**Goal**: Natural conversation with voice activation and interruption handling

**Duration**: 4 weeks

**Target Latency**: 1-2 seconds end-to-end

### Current Implementation Status

#### ✅ COMPLETED: ONNX-Based VAD (Module Only)
- **Type**: Silero VAD using ONNX model
- **Implementation**: Patched vad-rs crate for ort 2.0.0-rc.12 compatibility
- **Tests**: All 8 tests passing
- **Model**: Cached in ~/.cache/thaleia/ (~2MB)
- **Status**: ✅ Working in isolation, NOT yet integrated into pipeline

### Configuration
- **Default threshold**: 0.5 (speech probability)
- **Min speech duration**: 250ms
- **Min silence duration**: 300ms
- **Sample rate**: 16000 Hz

---

## Architecture: Dialogue Manager + State Machine

### Design Rationale

| Requirement | Solution |
|------------|----------|
| Multiple modes | Configurable trigger (wake-word / VAD / manual) |
| Interruption | State machine handles transitions cleanly |
| 1-2s latency | Streaming where possible, sequential for MVP |
| Multiple wake words | Silero wake word (we already have ONNX loaded) |

### Dialogue States
```rust
pub enum DialogueState {
    Idle,           // Waiting for trigger (wake word / VAD / manual)
    Listening,      // Capturing audio, VAD monitoring
    Processing,     // STT → LLM → TTS preparation
    Speaking,       // Playing audio response
    Interrupted,    // User interrupted, transitioning back to Listening
}
```

### State Transition Diagram
```
                    ┌─────────────┐
                    │    IDLE    │◀───────────────────────┐
                    └──────┬──────┘                        │
                           │ wake_word OR manual_start     │
                           ▼                               │
                    ┌─────────────┐                        │
         ┌─────────▶│  LISTENING  │                        │
         │          └──────┬──────┘                        │
         │                 │ silence_detected               │
         │                 ▼                                │
         │          ┌─────────────┐                         │
         │          │ PROCESSING  │                         │
         │          └──────┬──────┘                         │
         │                 │ response_ready                 │
         │                 ▼                                │
         │          ┌─────────────┐                         │
         │    ┌────▶│  SPEAKING   │────┐                  │
         │    │     └─────────────┘    │                  │
         │    │                        │ tts_done           │
         │    │ user_speaks           │                    │
         │    └────────────────────────┘                  │
         │                                                 │
         └─────────────────────────────────────────────────┘
                           │
              user_interrupted (from any state)
```

### Interruption Scenarios Handled
| Scenario | Action |
|----------|--------|
| User speaks while Thaleia is listening | Cancel current capture, start new |
| User speaks while Thaleia is processing | Cancel LLM request |
| User speaks while Thaleia is speaking | Stop TTS immediately, go to Listening |

---

## Pipeline Modes

| Mode | Wake Word | VAD | STT | Latency | Use Case |
|------|-----------|-----|-----|---------|----------|
| wake-vad-stt | ✅ | ✅ | ✅ | 1-2s | Natural conversation |
| vad | ❌ | ✅ | ✅ | 1-2s | Voice activation only |
| push-to-talk | ❌ | ❌ | ✅ | 2-3s | Manual trigger |
| stt-only | ❌ | ❌ | ✅ | - | Audio file → text |
| tts-only | ❌ | ❌ | ❌ | - | Text → speech |

---

## Implementation Schedule

### Week 1: Core Pipeline + State Machine

#### 4.1 Dialogue Manager
```rust
pub struct DialogueManager {
    state: DialogueState,
    config: VoiceConfig,
    audio_buffer: Vec<f32>,
    vad: VadSystem,
}

impl DialogueManager {
    pub fn handle_event(&mut self, event: DialogueEvent) -> Action;
}
```

#### 4.2 Interruption Handling
```rust
impl DialogueManager {
    pub fn interrupt(&mut self) {
        // Stop current playback
        // Clear audio buffer
        // Transition to Listening or Idle based on config
        self.state = DialogueState::Interrupted;
    }
}
```

#### 4.3 Pipeline Configuration
```rust
pub struct VoiceConfig {
    pub trigger: TriggerMode,
    pub vad: VadConfig,
    pub stt: SttConfig,
    pub tts: TtsConfig,
}

pub enum TriggerMode {
    WakeWord(Vec<String>),  // Multiple wake words
    Vad,                    // Voice activation
    Manual,                 // Push to talk
}
```

### Week 2: Wake Word Detection

#### 4.4 WakeWordBackend Trait
```rust
pub trait WakeWordBackend: Send {
    fn detect(&mut self, audio: &[f32], sample_rate: u32) -> Result<WakeWordResult>;
    fn set_keywords(&mut self, keywords: Vec<String>);
}

pub struct WakeWordResult {
    pub detected: bool,
    pub keyword: Option<String>,
    pub confidence: f32,
}
```

#### 4.5 Multiple Wake Words
- Default: "Hey Thaleia", "Thaleia"
- Custom: User can add their own
- Uses Silero wake word (already have ONNX runtime loaded)

#### 4.6 Connect to Idle → Listening
```rust
match self.state {
    DialogueState::Idle => {
        if let WakeWordResult { detected: true, .. } = self.wake_word.detect(audio)? {
            self.state = DialogueState::Listening;
        }
    }
    // ...
}
```

### Week 3: Integration + Latency

#### 4.7 Full Pipeline Integration
```
Capture Loop → VAD → Buffer Audio → STT → LLM → TTS → Playback
```

#### 4.8 Streaming STT (for lower latency)
- Stream partial transcripts to LLM
- Start TTS earlier

#### 4.9 Latency Profiling
- Measure each stage
- Target: <500ms per stage, 1-2s total

### Week 4: Polish + Testing

#### 4.10 Mode Testing
- [ ] Wake word mode: detection accuracy, false triggers
- [ ] VAD mode: voice activation sensitivity
- [ ] Push-to-talk: responsiveness
- [ ] STT-only: file processing

#### 4.11 Interruption Testing
- [ ] Interrupt while listening
- [ ] Interrupt while processing
- [ ] Interrupt while speaking
- [ ] Rapid interruption scenarios

#### 4.12 Performance Testing
- [ ] End-to-end latency measurement
- [ ] Memory usage profiling
- [ ] CPU usage profiling

---

## Deliverables (Updated)

### Week 1
- [ ] Dialogue Manager with state machine
- [ ] Interruption handling
- [ ] Pipeline configuration

### Week 2
- [ ] WakeWordBackend trait
- [ ] Multiple wake word support
- [ ] Wake word detection integration

### Week 3
- [ ] Full pipeline integration
- [ ] Streaming STT integration
- [ ] Latency profiling

### Week 4
- [ ] Mode testing
- [ ] Interruption testing
- [ ] Performance testing

### Overall Deliverables
- [ ] Dialogue Manager with state machine
- [ ] Interruption handling (user can stop Thaleia anytime)
- [ ] Pipeline modes: wake-word, vad-only, push-to-talk
- [ ] Wake word detection (multiple wake words)
- [ ] VAD integration into pipeline
- [ ] Target 1-2s end-to-end latency
- [ ] Full integration testing

---

## Phase 5: Polish + Streaming

**Goal**: Optimize latency and add advanced features

**Duration**: 2 weeks

### Tasks

#### 5.1 Streaming TTS ⭐ PRIORITY
**Current**: Buffer entire audio before playback
**Target**: Play chunks as they're generated (TTFA < 100ms)

**Implementation approach**:
```rust
// StreamingAudioPlayer using rodio Mixer + Player
pub struct StreamingPlayer {
    sink: MixerDeviceSink,
    mixer: Mixer,
}

impl StreamingPlayer {
    pub fn play_chunk(&self, samples: &[f32], sample_rate: u32) {
        // Convert samples to SamplesBuffer
        // Add to mixer immediately
        // Audio plays while next chunk synthesizes
    }
}
```

**Key changes needed**:
1. `AudioPlayer` → `StreamingPlayer` with concurrent chunk handling
2. Kokoro streaming API integration (yields chunks, not full audio)
3. Concurrent synthesis + playback (spawn synthesis task)
4. Chunk queue with backpressure

**Dependencies**: rodio Mixer/Player API (already available in rodio 0.22)

**Trade-offs**:
| Approach | Pros | Cons |
|----------|------|------|
| Current (buffered) | Simple, reliable | Latency = synthesis time |
| Streaming | Low TTFA, responsive | More complex, needs tuning |

#### 5.2 Barge-in Support
```rust
// Interrupt Thaleia while speaking
pub struct AudioController {
    is_speaking: AtomicBool,
    current_stream: Option<AbortHandle>,
}

impl AudioController {
    pub fn interrupt(&self) {
        // Stop current playback
        // Start new request
        self.is_speaking.store(false);
        if let Some(handle) = self.current_stream.take() {
            handle.abort();
        }
    }
}
```

#### 5.3 Performance Profiling
```rust
// Tracing for latency analysis
#[tracing::instrument]
pub async fn process_audio(&self, audio: Audio) -> Timing {
    let start = Instant::now();
    
    let vad = self.vad.detect(&audio).await;
    let stt = self.stt.transcribe(&vad.audio).await;
    let llm = self.llm.generate(&stt.text).await;
    let tts = self.tts.speak(&llm.response).await;
    
    Timing {
        vad_ms: vad.elapsed,
        stt_ms: stt.elapsed,
        llm_ms: llm.elapsed,
        tts_ms: tts.elapsed,
        total_ms: start.elapsed(),
    }
}
```

#### 5.4 Memory Optimization
```rust
// Memory pooling for audio buffers
pub struct AudioPool {
    pool: Vec<Box<[i16]>>,
}

impl AudioPool {
    pub fn acquire(&self, size: usize) -> PooledAudio {
        // Reuse buffers
        // Reduce allocations
    }
}
```

#### 5.5 Cross-Platform Builds
```yaml
# .github/workflows/build.yml
jobs:
  build:
    strategy:
      matrix:
        include:
          - os: ubuntu-latest
            target: x86_64-unknown-linux-gnu
          - os: macos-latest
            target: aarch64-apple-darwin
          - os: windows-latest
            target: x86_64-pc-windows-msvc
```

#### Deliverables
- [ ] Streaming TTS working
- [ ] Barge-in support
- [ ] Latency profiling
- [ ] Cross-platform binaries

---

## Phase 6: Community + Growth

**Goal**: Build a sustainable open-source project

**Duration**: Ongoing

### Tasks

#### 6.1 Documentation
```
docs/
├── README.md
├── ARCHITECTURE.md
├── RESEARCH.md
├── COMPARISON.md
├── PLAN.md
├── CONTRIBUTING.md
├── INSTALLATION.md
├── MCP_INTEGRATION.md
└── API.md
```

#### 6.2 Plugin System
```rust
// thaleia-plugin/src/lib.rs
#[derive Plugin)]
pub trait SttPlugin {
    fn name(&self) -> &str;
    async fn transcribe(&self, audio: &[i16]) -> Result<String>;
}

// Example: faster-whisper plugin
pub struct FasterWhisperPlugin {
    model: whisper::Model,
}

#[thaleia_plugin]
impl SttPlugin for FasterWhisperPlugin {
    fn name(&self) -> &str { "faster-whisper" }
    
    async fn transcribe(&self, audio: &[i16]) -> Result<String> {
        self.model.transcribe(audio).await
    }
}
```

#### 6.3 Community Guidelines
```markdown
# CONTRIBUTING.md
- Code style (Rustfmt)
- Testing requirements
- PR process
- Code of conduct
```

#### 6.4 Release Process
```bash
# Version bumps
cargo release patch  # 0.1.0 -> 0.1.1
cargo release minor  # 0.1.0 -> 0.2.0
cargo release major # 0.1.0 -> 1.0.0
```

#### Deliverables
- [ ] Complete documentation
- [ ] Plugin system documented
- [ ] Contributing guide
- [ ] Release automation

---

## MCP Server Implementation

### Design Philosophy: Green & Efficient

Thaleia's MCP server prioritizes **minimal resource usage** to reduce environmental impact. Every design decision considers token consumption, storage, and compute efficiency.

### Phase 1: Core MCP Tools

| Tool | Description | Token Cost |
|------|-------------|------------|
| `speak(text, voice?)` | TTS synthesis | N/A (local) |
| `listen(timeout?)` | STT transcription | N/A (local) |
| `list_voices` | Show available voices | ~100 tokens |
| `get_status` | Current state | ~50 tokens |

### Phase 2: Session Management (Green History)

**Core Principle: Sparse, forgetful memory by default**

#### Storage Tiers
| Tier | Content | Size |
|------|---------|------|
| Hot | Last 3 exchanges, full text | ~1KB |
| Warm | Semantic only (topic, sentiment, outcome) | ~500 bytes |
| Cold | Explicit user saves only | varies |
| Skip | Casual chat below threshold | 0 |

#### Salience Scoring
Simple heuristics (no ML needed):
- Emotional intensity (`!`, `???`, CAPS) → high score
- Explicit markers (`"remember this"`, `"don't forget"`) → high score
- New topics detected → medium score
- Casual chat (`"hey"`, `"ok"`) → skip

#### User Presets
- `ephemeral` - No persistence (most green)
- `standard` - Compressed history (default)
- `longterm` - More retention for those who want it

#### Semantic Memory Structure
```rust
struct SemanticNote {
    timestamp: u32,        // relative, not absolute
    topic: String,        // compressed topic ID
    sentiment: i8,         // -1 to 1
    key_outcome: Option<String>,  // "decided X", "learned Y"
}
```
**Result**: 50 exchanges = ~500 bytes vs 50KB raw text (100x reduction)

### Phase 3: Full Duplex (Optional)

- `converse(text)` - Combined listen→think→speak loop
- Interrupt/barge-in support
- Emotional context tracking

### Storage Backend Options

1. **File-based (Simple)** - JSON in `~/.config/thaleia/`
2. **SQLite (Structured)** - Lightweight database
3. **MEM8-ready (Future)** - Wave-based consciousness memory

### Phase 2b: MEM8 Integration (Future)

**Reference**: [MEM|8: Wave-Based Cognitive Architecture](https://doi.org/10.5281/zenodo.16436298)

**Overview**: Replace token-based session memory with wave-based consciousness system

| Feature | Current (session.rs) | MEM8 (Future) |
|--------|---------------------|---------------|
| Storage | Text-based semantic notes | Wave interference patterns |
| Format | Token strings | 32-byte wave patterns |
| Speed | Fast | 973x faster (claimed) |
| Emotional | VAD sentiment | Native emotional encoding |
| Compression | ~50% | 99% (with .m8 format) |

**Key Benefits**:
- Cross-sensory binding (audio → visual → emotional)
- Natural memory consolidation via interference
- Sub-microsecond recall times
- 16x smaller memory footprint
- Emotional context natively encoded (VAD)

**Architecture Layers**:
- Layer 0 (0-10ms): Hardware reflexes
- Layer 1 (10-50ms): Pattern-matched responses
- Layer 2 (50-200ms): Emotional responses
- Layer 3 (>200ms): Conscious deliberation

**Implementation Path**:
1. Create `thaleia-mem8` crate for wave memory
2. Implement .m8 format parser
3. Add SIMD-optimized wave operations (AVX2/AVX-512)
4. Integrate with MCP session layer

**Note**: See RESEARCH.md for full paper details and citations.

### Token Budget Target

Target: <50K tokens/session
```
System prompt + tools:  ~2K tokens
Per-exchange overhead:  ~500 tokens
History (20 exchanges): ~1K tokens
Response buffer:       ~5K tokens
```

---

## Task Breakdown

### Week-by-Week Plan

| Week | Phase | Tasks | Deliverable |
|------|-------|-------|-------------|
| 1 | 0 | Workspace, audio engine | Project structure |
| 2 | 0 | Error types, config | Foundation complete |
| 3 | 1 | Kokoro integration | TTS works locally |
| 4 | 1 | Voice system, CLI | `thaleia speak` works |
| 5 | 2 | Whisper integration | STT works locally |
| 6 | 2 | Streaming, preprocessing | `thaleia listen` works |
| 7 | 3 | MCP protocol | Server running |
| 8 | 3 | Tools, Claude integration | MCP tools work |
| 9 | 4 | Silero VAD | Voice detection |
| 10 | 4 | Wake word, conversation | Natural flow |
| 11 | 5 | Streaming TTS | Low latency |
| 12 | 5 | Barge-in, profiling | Polish complete |
| 13+ | 6 | Docs, plugins, community | Launch! |

---

## Milestones

### v0.1.0 (Alpha)
- [x] Basic TTS with Kokoro ✅
- [x] Basic STT with Whisper ✅
- [x] **Pluggable STT architecture** ✅ (NEW)
- [x] CLI interface ✅
- [x] Linux binary ✅
- [ ] **Streaming TTS** (pending - see Phase 5.1)

### v0.2.0 (Beta)
- [x] MCP server ✅
- [x] Pluggable STT ✅ (NEW)
- [ ] Claude Desktop integration
- [ ] VAD support
- [ ] macOS binary
- [ ] **Streaming TTS** (target)

### v0.3.0 (RC)
- [ ] Wake word (optional)
- [ ] **Streaming TTS**
- [ ] Barge-in
- [ ] Windows binary

### v1.0.0 (Stable)
- [ ] Plugin system
- [ ] Documentation complete
- [ ] Performance optimized
- [ ] Cross-platform builds

---

## Resource Requirements

### Development Time
- **Estimated**: 12-16 weeks (part-time)
- **Full-time equivalent**: 6-8 weeks

### Dependencies
| Component | Time to Integrate | Complexity |
|-----------|-----------------|------------|
| Audio Engine | 1 week | Medium |
| Kokoro TTS | 1 week | Low |
| Whisper STT | 1 week | Medium |
| MCP Server | 2 weeks | High |
| Silero VAD | 1 week | Low |
| Wake Word | 1 week | Medium |

### External Resources
| Resource | Purpose | Cost |
|----------|---------|------|
| Whisper models | STT | Free |
| Kokoro models | TTS | Free |
| Silero models | VAD | Free |
| Claude Desktop | Testing | Free |

---

## Success Criteria

### Technical
- [ ] End-to-end latency < 500ms (P95)
- [ ] TTS latency < 100ms TTFA
- [ ] STT accuracy > 93%
- [ ] Binary size < 100MB

### Usability
- [ ] Installation in < 5 minutes
- [ ] CLI works out of the box
- [ ] MCP integration in < 10 minutes

### Community
- [ ] 100 Codeberg stars
- [ ] 10 pull requests
- [ ] 5 community plugins

---

## Community Contribution: Polish Voice Pack

Thaleia aims to support multiple languages. The first community contribution opportunity is creating a **Polish voice pack** for Kokoro TTS.

### Current State
| Component | Status |
|-----------|--------|
| Whisper STT | ✅ Supports Polish (multilingual model) |
| espeak-ng G2P | ✅ Supports Polish (`pl`) |
| Kokoro TTS | ❌ English voices only |

### What's Needed
A Polish voice pack (similar to English voice pack `0.bin`) containing:
- 256-dimensional style embeddings for Polish voice(s)
- Polish-specific voice characteristics

### Requirements for Contributors

#### 1. Training Data
- **20-50 hours** of permissively-licensed Polish audio
- Public domain, Apache, MIT, or similar license
- Clear audio, native Polish speakers

#### 2. Technical Requirements
- GPU access for training (A100 recommended)
- Experience with ML training
- Knowledge of Polish phonology

#### 3. Process
1. **Open discussion first**: Create issue on Codeberg before starting
2. **Coordinate with maintainers**: Ensure no duplicate work
3. **Follow our workflow**: Use our training code pattern
4. **Test locally**: Verify quality before submission
5. **Submit PR**: With documentation and tests

### Cost Estimate
| Item | Estimate |
|------|----------|
| GPU training | ~$200-400 |
| Data preparation | 10-20 hours |
| Voice pack creation | 10-20 hours |
| **Total** | **$200-400 + effort** |

### Resources
- Training code: `jonirajala/kokoro_training` (English pattern)
- Polish G2P: espeak-ng (`pl` language)
- Alignment: Montreal Forced Aligner (MFA) with Polish dictionary

### How to Contribute
1. Check existing issues for planned languages
2. Create new issue: "PolishVoice: Interested in contributing"
3. Wait for maintainer acknowledgment
4. Follow approved process
5. Submit PR with voice pack + tests

### Community Guidelines
- **Be polite and collaborative**: Welcome all contributions
- **Open discussion first**: Don't start work without coordintion
- **Quality over speed**: Take time to do it right
- **Follow our way**: Use our templates and processes

---

*"The best way to eat an elephant is one bite at a time. Thaleia will be built one feature at a time."*

**Next step**: Let's start Phase 0 - Project Foundation!
