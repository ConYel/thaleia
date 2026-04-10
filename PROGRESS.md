# Thaleia Audio Progress

## Goal

Fix microphone capture in Thaleia (containerized voice AI with Kokoro TTS and Whisper STT) and implement audio playback that works in Qubes OS.

## Instructions

- All development happens inside containers using podman and makefile
- Use ring cryptography instead of aws-lc-rs (user philosophical preference)
- Minimize dependencies for environmental impact (green philosophy)
- Follow SOLID principles
- Thaleia should have audio capabilities embedded, not use external tools
- Thaleia must be lean (no media playback, no music libraries)

## Discoveries

1. **rodio 0.22 API changes** - Significant breaking changes from 0.20:
   - `Sink` → `Player` + `Mixer`
   - `OutputStream` → `DeviceSinkBuilder`
   - `Decoder::new()` → `Decoder::try_from()`
   - `cpal` no longer re-exported - needs separate dependency

2. **VLC works but rodio doesn't on Qubes** - Why?
   - VLC uses **SDL2** which has native PulseAudio support
   - rodio/cpal uses **ALSA** which is blocked on Qubes
   - Qubes uses `module-vchan-sink` - a Xen-based audio forwarding, not standard PulseAudio

3. **Qubes audio architecture** - Audio forwarded via vchan from VM to dom0/audioVM:
   - Port 4713: Audio playback (VM → dom0)
   - Port 4714: Audio capture + control (VM ↔ dom0)
   - Format: Raw 44.1kHz, S16LE, 2 channels, no compression
   - Requires `sys-audio` qube setup for proper audio forwarding

4. **SDL2 API** (from examining sdl2 crate source):
   - Use `open_playback()` and `open_capture()` methods
   - Audio callbacks receive `&mut [f32]` slices
   - `resume()` and `pause()` return `()` not `Result`

5. **SDL2 Thread Safety** - `AudioSubsystem` is `!Send + !Sync`:
   - Cannot store in global `OnceLock`
   - Each `SDL2Backend` instance owns its own `AudioSubsystem`
   - SDL_Init is idempotent - safe to call multiple times

## Accomplished

### ✅ Completed

1. **Rodio 0.20 → 0.22 Upgrade**
   - Updated all API calls in kokoro-tiny and thaleia-core
   - Added `cpal` as direct dependency
   - Fixed feature flags in Cargo.toml

2. **TLS → ring** (not aws-lc-rs)
   - Configured rustls with `ring` feature

3. **Created Unified Audio Backend Architecture**
   ```
   AudioSystem (auto-detects best backend)
   ├── RodioBackend (ALSA - standard Linux)
   ├── SDL2Backend (PulseAudio - Qubes)
   └── NullBackend (file-only fallback)
   ```

4. **New audio module structure created**:
   - `audio/mod.rs` - Public API
   - `audio/backends/mod.rs` - AudioBackend trait + factory
   - `audio/backends/rodio_backend.rs` - Primary backend (working)
   - `audio/backends/sdl2_backend.rs` - SDL2 fallback (FIXED)
   - `audio/backends/null_backend.rs` - No audio fallback
   - `audio/diagnostics.rs` - Backend detection + diagnostics
   - `audio/engine.rs` - AudioEngine wrapper for backwards compat

5. **Updated CLI and MCP to use new audio API**

6. **Fixed conditional compilation for audio features**
   - Backends are now conditionally compiled based on features
   - `rodio` feature enables RodioBackend
   - `sdl2-audio` feature enables SDL2Backend
   - Both can be enabled together

7. **SDL2 Backend Fix**
   - Fixed thread safety issue: `AudioSubsystem` cannot be in global static
   - Used atomic state tracking (`AtomicBool`) instead
   - Each `SDL2Backend` instance owns its own `AudioSubsystem`
   - SDL_Init is idempotent - safe for multiple instances

8. **Added WAV Audio Export**
   - `CapturedAudio::save_wav(path)` method saves to 16-bit PCM WAV
   - Saves at native sample rate (SDL2: 44100Hz)
   - Whisper handles resampling internally
   - Uses `hound` crate (already in deps)

9. **Added CLI Capture Flag**
   - `thaleia listen --capture <file.wav>` captures mic and saves WAV
   - Interactive capture (5 seconds) if no path provided

10. **Added MCP save_to Parameter**
    - `thaleia_listen` tool accepts optional `save_to` argument
    - Returns path in response: "Audio saved to: /tmp/test.wav"

11. **Fixed Makefile test-mic**
    - Added `SDL_AUDIODRIVER=pulse` environment variable
    - Added `sdl2-audio` feature to `test-full` target

12. **Fixed Makefile BUILD_FEATURES issue**
    - Makefile was dropping features due to comma handling bug
    - Added `BUILD_FEATURES` variable to properly pass features
    - Fixed `build-full` target to include `rodio,sdl2-audio,vad`

13. **Fixed AudioSystem fallback bug**
    - When SDL2 backend failed to initialize, it was returning `AudioSystem::None`
    - Changed to continue trying other backends instead of giving up
    - Now properly falls back to Rodio if SDL2 fails

14. **Fixed Rodio availability check**
    - Original check required actually configuring microphone which fails in containers
    - Changed to just check if devices can be enumerated (not configured)
    - Moved actual device selection to capture time with fallback loop

15. **Added device fallback in capture**
    - Now iterates through all available audio devices until one works
    - Works in Qubes where some PulseAudio devices are proxies that fail
    - Device 7 "PulseAudio Sound Server" works in containerized environments

### ✅ Verified Working

- TTS synthesis: `make speak` works (Thaleia speaks!)
- Audio playback: SDL2 backend connects to PulseAudio
- Microphone capture: `make test-mic` works, captures 44.1kHz audio
- Transcription: Whisper tiny model transcribes captured audio
- MCP server: `thaleia_listen` tool works with `save_to` parameter
- WAV export: Audio saved to file successfully

## MCP Microphone Fix (April 5, 2026) ✅

### Problem
- CLI `thaleia listen` worked correctly (captured 217k samples with SDL2 backend)
- MCP `thaleia_listen` returned no audio / [BLANK_AUDIO]

### Root Cause Analysis
1. MCP audio thread created `AudioSystem` at startup (using SDL2) ✅
2. BUT each `thaleia_listen` call created NEW `AudioCapture` → new AudioSystem
3. New AudioSystem selected Rodio backend (wrong!) instead of SDL2
4. Also: Removed PULSE_SOURCE env var on auto-detect (bug!)

### Fix Applied
1. Changed MCP to use existing `audio_system` from audio thread (same as CLI)
2. Removed bug that removed PULSE_SOURCE env var on auto-detect
3. Added optional `source` parameter to `thaleia_listen` for explicit source selection
4. Added `thaleia_list_sources` tool to list available audio sources
5. Added debug output to trace audio backend selection

### Files Modified
```
crates/thaleia-mcp/src/
├── audio_manager.rs    # Use existing audio_system, fixed env var bug
└── rmcp_server.rs      # Added source parameter, list_sources tool

crates/thaleia-core/src/
└── capture.rs         # Added debug output

crates/thaleia-core/src/audio/backends/
└── sdl2_backend.rs    # Added capture debug output

Cargo.toml             # Fixed reqwest TLS: __rustls → rustls-tls-webpki-roots
kokoro-tiny/Cargo.toml # Fixed reqwest TLS
thaleia-core/Cargo.toml # Added blocking feature for VAD
Makefile               # Added espeak-ng env vars
```

### TLS Configuration Fix
- Issue: `__rustls` is internal reqwest feature (non-functional)
- Fix: Changed to `rustls-tls-webpki-roots` in 3 Cargo.toml files
- Result: Pure Rust TLS with ring, no OpenSSL dependency

### Unified Debug System (April 6, 2026) 🔄 IN PROGRESS

**Problem**: Development debugging used ~40 raw `eprintln!` statements scattered throughout code. No unified way to enable/disable verbose output.

**Solution**: Created unified debug/trace system following Rust 2026 best practices:

**Implementation**:
1. Created `thaleia-core/src/debug.rs` with:
   - `is_debug()` - Check if debug mode enabled
   - `set_debug(bool)` - Programmatic debug control
   - `init_logging()` - Initialize tracing-subscriber
   - `thaleia_debug!` macro - Verbose debug output to stderr
   - Auto-initialization from `THALEIA_DEBUG` or `RUST_LOG` env vars

2. Updated Cargo.toml files:
   - Added `tracing-subscriber` with `env-filter` feature
   - Added `ctor` for auto-initialization

3. Updated entry points:
   - `thaleia-mcp/main.rs` - Calls `init_logging()`, uses `--debug` flag
   - `thaleia-cli/main.rs` - Added `--debug` flag, calls `init_logging()`

4. Replaced `eprintln!` statements:
   - Production logging → `tracing::info!`, `tracing::warn!`, `tracing::error!`
   - Verbose debug → `thaleia_debug!` (only when debug enabled)

**Files Modified**:
```
crates/thaleia-core/src/
├── debug.rs              # NEW - Unified debug module
├── lib.rs               # Export debug utilities
├── capture.rs           # Replace eprintln! with tracing/thaleia_debug
└── audio/backends/
    ├── sdl2_backend.rs  # Replace eprintln! with thaleia_debug
    └── rodio_backend.rs # Replace eprintln! with thaleia_debug

crates/thaleia-mcp/src/
├── lib.rs               # Re-export debug from thaleia-core
├── main.rs              # Add --debug flag, init logging
├── audio_manager.rs     # Replace eprintln! with thaleia_debug
└── rmcp_server.rs       # Replace eprintln! with thaleia_debug

crates/thaleia-cli/src/
└── main.rs              # Add --debug flag, init logging
```

**Remaining**:
- Full end-to-end testing of debug output

### Code Quality - Lint & Format (April 7, 2026) ✅ COMPLETED

**Problem**: No standardized way to check code quality. Needed to match Rust 2026 best practices.

**Solution**: Added Makefile targets and fixed clippy warnings.

**Implementation**:
1. Added `make fmt` target - runs `cargo fmt --all`
2. Added `make lint` target - runs `cargo clippy --all-targets --all-features -- -D warnings`

3. Fixed clippy errors in thaleia-core:
   - Collapsible if statements (3 places)
   - `std::io::Error::other()` shorthand
   - Derived Default trait for enums
   - `.is_multiple_of()` instead of `%` operator
   - Removed unnecessary type casts

4. Fixed clippy errors in thaleia-mcp:
   - Collapsible if statements
   - Derived Default for MemoryMode
   - Removed unused imports

5. Fixed clippy errors in thaleia-cli:
   - Removed unnecessary `mut`

6. Fixed test code:
   - MCP test: Used array instead of vec!
   - SDL2 test: Made conditional to avoid double-init

**Files Modified**:
```
Makefile                           # Added fmt and lint targets
crates/thaleia-core/src/audio/backends/rodio_backend.rs   # Clippy fixes
crates/thaleia-core/src/audio/backends/sdl2_backend.rs   # Clippy fixes
crates/thaleia-core/src/stt/engine.rs                    # Derived Default
crates/thaleia-core/src/stt/whisper.rs                   # Derived Default, is_multiple_of
crates/thaleia-core/src/vad/onnx.rs                     # Collapsible ifs
crates/thaleia-mcp/src/audio_manager.rs                 # Collapsible if
crates/thaleia-mcp/src/session.rs                       # Derived Default, collapsible if
crates/thaleia-mcp/src/rmcp_server.rs                  # Removed unused import
crates/thaleia-cli/src/commands.rs                      # Removed mut
crates/thaleia-mcp/tests/rmcp_server_test.rs          # Used array
```

**Result**:
- `make build` ✅ Passes
- `make fmt` ✅ Passes  
- `make lint` ✅ Passes (no errors)
- Tests ✅ 97 pass
- 🔄 E2E testing: Not yet verified

**Remaining Warnings** (external dependencies, not our code):
- kokoro-tiny: 66 warnings (cfg condition `as-lib`)
- vad-rs: 2 warnings (unused import/variable)

**Usage**:
```bash
# Production (minimal output)
thaleia listen

# Enable verbose debug output (runtime)
RUST_LOG=debug thaleia listen
THALEIA_DEBUG=1 thaleia listen
thaleia --debug listen

# MCP debug
thaleia-mcp --debug
```

### Current State
- ✅ CLI: Works (captures with SDL2)
- ✅ MCP: Works (uses same SDL2 backend as CLI)
- ✅ OpenCode integration: Working
- ✅ Code quality: lint and fmt targets added
- 🔄 Debug system: Implemented - needs E2E verification

### Codeberg Templates (April 9, 2026) ✅ COMPLETED

**Goal**: Prepare for publishing to Codeberg (Forgejo) with proper templates.

**Implementation**:
1. Created `.forgejo/ISSUE_TEMPLATE/` directory for Codeberg:
   - `bug.yaml` - Bug report template
   - `feature.yaml` - Feature request template
   - `config.yaml` - Configuration issue template

2. Created `.forgejo/pull_request_template.md`:
   - Summary section
   - Testing checklist
   - Code quality checklist

3. Updated README.md:
   - Pre-alpha badges
   - Version: 0.1.0-pre-alpha.1
   - "Built With LLM Agents" badge

4. Added "About the Creator" section:
   - Disclosure: creator is NOT a Rust expert
   - Thaleia created with LLM agent assistance

5. Updated CONTRIBUTING.md:
   - Codeberg URL instead of GitHub

**Files Modified**:
```
.forgejo/
├── ISSUE_TEMPLATE/
│   ├── bug.yaml
│   ├── feature.yaml
│   └── config.yaml
└── pull_request_template.md

README.md     # Added badges, disclosure
CONTRIBUTING.md # Updated URL
.gitignore    # Comprehensive exclusions
```

### ⏳ Remaining Issues

#### Phase 1: Production-Ready Foundation

1. **MCP Server with rmcp**
   - Implemented using rmcp v1.3 with stdio transport
   - Tools: listen, speak, list_voices, get_status, list_sources
   - 🔄 Debug system: Implemented - needs E2E verification
   - 🔄 Audio source selection: Implemented - needs verification
   - HTTP transport: NOT implemented yet

2. **HTTP Transport Support**
   - NOT implemented yet
   - Needed for container deployment

3. **Container Image**
   - Working in development mode

#### Phase 2: Natural Conversation

4. **VAD Integration**
   - ONNX-based VAD module exists but not connected to pipeline
   - Need to integrate with audio capture loop
   - Enables: automatic silence detection, natural conversation

5. **Wake Word Detection**
   - Not yet implemented
   - Enables: "Hey Thaleia" hands-free activation
   - Use Silero wake word (already have ONNX loaded)

6. **Interruption Handling**
   - User cannot stop Thaleia mid-speech
   - Need: Stop TTS, cancel STT, cancel LLM requests
   - Critical for natural conversation flow

#### Phase 3: Polish & Performance

7. **Streaming TTS**
   - Currently buffered (wait for full audio before playback)
   - Need: Stream chunks as generated
   - Target: TTFA < 100ms

8. **Whisper model accuracy**
   - Currently using tiny model
   - Can upgrade to base/small for better accuracy
   - Test image bundles small model (~465MB)

#### Future (Phase 4)

9. **Multiple Languages** - i18n support
10. **Voice Customization** - More voices, voice mixing
11. **Enterprise Features** - Auth, clustering

---

## MCP Benchmark Reference

From TM Dev Lab benchmark (39.9M requests, 15 implementations):

| Implementation | RPS | Avg Latency | Memory |
|---------------|-----|-------------|--------|
| Rust (rmcp) | 4,845 | 5.09ms | 10.9 MB |
| Quarkus | 4,739 | 4.04ms | 194 MB |
| Go | 3,616 | 6.87ms | 23.9 MB |
| Java MVC | 3,540 | 6.13ms | 368 MB |

**Key insight:** Use rmcp v0.17.0+ with `json_response: true`

## Relevant Files

```
/home/user/pickle_files/thaleia/
├── Cargo.toml                              # Workspace - ring for TLS
├── crates/
│   ├── thaleia-core/
│   │   ├── Cargo.toml                     # Audio features
│   │   └── src/
│   │       ├── lib.rs                     # Updated exports
│   │       ├── capture.rs                 # Added save_wav() method
│   │       └── audio/
│   │           ├── mod.rs                  # Public API
│   │           ├── backends/
│   │           │   ├── mod.rs             # AudioBackend trait + AudioSystem
│   │           │   ├── rodio_backend.rs   # Rodio/ALSA backend
│   │           │   ├── sdl2_backend.rs    # SDL2/PulseAudio backend (FIXED)
│   │           │   └── null_backend.rs    # No audio fallback
│   │           ├── diagnostics.rs          # Backend detection
│   │           └── engine.rs              # AudioEngine wrapper
│   ├── thaleia-cli/
│   │   └── src/main.rs                   # Added --capture flag
│   └── thaleia-mcp/
│       └── src/tools.rs                  # Added save_to parameter
├── Containerfile.dev                      # Audio/PulseAudio config
└── Makefile                              # Test targets
```

## Build Commands

```bash
# Build with full features (kokoro + playback + sdl2-audio)
make build-full

# Test audio system
make test-audio

# Test microphone capture (MCP)
make test-mic DURATION=3

# Test microphone and save to WAV file
make test-mic-save OUTPUT=capture.wav DURATION=3

# Test TTS synthesis + playback
make speak TEXT="Hello world"

# MCP with debug output
make run-mcp-test-debug
```

## MCP Tool Usage

```bash
# Listen and transcribe
{"name":"thaleia_listen","arguments":{"timeout":5}}

# Listen and save to WAV
{"name":"thaleia_listen","arguments":{"timeout":5,"save_to":"/tmp/test.wav"}}

# Speak text
{"name":"thaleia_speak","arguments":{"text":"Hello!"}}
```

## Architecture Notes

### Backend Selection Logic

```
AudioSystem::new():
1. Try SDL2 (PulseAudio) - if sdl2-audio feature enabled (Qubes first)
2. Try Rodio (ALSA) - if rodio feature enabled
3. Fall back to None (file-only mode)
```

### Feature Flags

- `rodio` - Rodio backend with playback + recording
- `sdl2-audio` - SDL2 backend for PulseAudio systems
- `playback` - Audio playback only
- `playback-capture` - Both playback and capture
- `kokoro` - TTS synthesis
- `whisper` - Speech-to-text

### Current Container Configuration

The container includes:
- PulseAudio client libraries
- SDL2 development libraries
- ALSA development libraries
- Sound device passthrough (`--device /dev/snd:/dev/snd:rw`)
- PulseAudio socket mount (`-v /run/user/1000/pulse:/run/user/1000/pulse:rw`)
- Environment variables: `SDL_AUDIODRIVER=pulse`, `PULSE_SERVER=unix://...`

## Next Steps

1. **Test on actual Qubes system** - The SDL2 backend is implemented but needs Qubes to verify
2. **Upgrade Whisper model** - Switch from tiny to base model for better accuracy
3. **Document audio architecture** - Update PLAN.md with backend selection logic

## Code Review Summary (2026-03-23)

### Cleanup Performed
- Removed dead code file: `crates/thaleia-core/src/tts_kokoro.rs` (redundant re-export)
- Removed dead code file: `crates/thaleia-core/src/audio/playback.rs` (unused module with `AudioPlayer`, `samples_to_wav`, `check_audio`)

### Refactoring Performed
- **CLI Split**: Split `main.rs` (602 lines) into:
  - `main.rs` (~200 lines) - CLI parsing, dispatch, tests
  - `commands.rs` (~300 lines) - All command implementations
- **Fixed Comment**: Updated `capture.rs` sample rate comment to be accurate

### Code Quality Observations
- ✅ Audio backends are well-structured with clear separation of concerns
- ✅ MCP tools are clean with proper async/await patterns
- ✅ Session management follows green computing principles
- ✅ Proper use of conditional compilation for features
- ✅ Tests exist for core functionality
- ✅ CLI now follows SRP (Single Responsibility Principle)

## Pluggable STT Architecture (2026-03-27)

### Goal
Fix and perfect Thaleia's STT (Speech-to-Text) architecture to be:
1. **Pluggable** - Support multiple backends (Whisper, Qwen3-ASR)
2. **Consistent** - Same pattern as existing Audio module (enum + factory)
3. **SOLID-compliant** - Clean trait design following developer guidelines
4. **Testable** - TDD approach with trait contract tests

### Implementation Completed

#### 1. Enhanced `SttBackend` Trait
New methods added to the trait:
- `info()` - Returns backend metadata (`SttInfo`)
- `supports_streaming()` - Query streaming capability
- `languages()` - List supported languages
- `transcribe()` - Now returns `Transcription` (text + metadata) instead of raw `String`

#### 2. Domain Types Created
- `BackendName` - Wrapped backend name (Whisper, Qwen3Asr, Custom)
- `LanguageCode` - Wrapped ISO 639-1 code with validation
- `SttInfo` - Metadata struct (name, languages, streaming, model_size)
- `Transcription` - Result struct (text, language, is_partial)

#### 3. `SttSystem` Factory
- `new()` - Auto-detect backend (Whisper default)
- `with_backend(SttBackendType)` - Select specific backend
- Matches Audio module pattern (enum + factory)

#### 4. Updated Exports
- `SttBackend`, `SttSystem`, `SttBackendType`
- `BackendName`, `LanguageCode`, `SttInfo`, `Transcription`
- Updated in both `lib.rs` and `mod.rs`

#### 5. Fixed MCP and CLI
- Updated to use new `SttBackend` trait
- Now use `.text` to get transcribed string from `Transcription` result

#### 6. Added Cargo Registry Cache
- Added to Makefile for faster builds:
  ```makefile
  -v $(HOME_DIR)/.cargo/registry:/home/devuser/.cargo/registry:Z
  ```

### Files Modified
```
crates/thaleia-core/src/
├── lib.rs                      # Updated exports
├── stt/
│   ├── mod.rs                  # Updated exports, added new types
│   ├── engine.rs               # NEW: SttBackend trait, domain types, SttSystem
│   └── whisper.rs              # Updated to implement SttBackend trait

crates/thaleia-cli/src/
└── commands.rs                 # Updated to use SttBackend trait

crates/thaleia-mcp/src/
└── tools.rs                    # Updated to use SttBackend trait

Makefile                        # Added Cargo registry cache
```

### Test Results
- Build: ✅ Passes (45.76s with cached deps)
- Tests: 46 passed, 1 failed (pre-existing SDL2 audio test)
- New tests for domain types: ✅ All pass

## VAD Implementation (2026-03-27)

### Goal
Add Voice Activity Detection (VAD) using Silero for natural conversations.

### Completed: ONNX-Based VAD ✅
- **Type**: Silero VAD using ONNX model
- **Implementation**: Patched vad-rs crate for ort 2.0.0-rc.12 compatibility
- **State Machine**: Idle → Speaking → Ending → Silence
- **Architecture**: Following STT/Audio pattern (trait + factory)
- **Tests**: 8 tests passing
- **Model**: Auto-downloads from GitHub on first use (~2MB cached in ~/.cache/thaleia/)

### Configuration (Sensible Defaults)
- Threshold: 0.5 (speech probability)
- Min speech duration: 250ms
- Min silence duration: 300ms
- Sample rate: 16000 Hz

### Patch Applied
- Forked vad-rs from GitHub
- Updated ort from rc.9 to rc.12
- Updated ndarray from 0.16.1 to 0.17.2
- Fixed API changes in tensor creation/extraction

### Files Created
```
crates/thaleia-core/src/vad/
├── engine.rs    # VadBackend trait, VadSystem factory
├── silero.rs    # Energy-based fallback
├── onnx.rs     # ONNX-based implementation (NEW)
└── mod.rs       # Module exports

patches/vad-rs/  # Patched vad-rs crate
├── src/vad.rs      # Fixed for ort 2.0.0-rc.12
├── src/session.rs  # Custom error handling
└── Cargo.toml      # Updated dependencies
```

## Pipeline Architecture Plan (2026-03-27) - UPDATED

### Architecture Decision: Dialogue Manager + State Machine

After researching best practices from Rhasspy, LiveKit, and production voice agents, we chose:

- **State machine** for dialogue flow (handles interruption cleanly)
- **Sequential within each state** (simpler for MVP)
- **Streaming** later to reduce latency

### Why This Architecture?

| Requirement | Solution |
|-------------|----------|
| Multiple modes | Configurable trigger (wake-word / VAD / manual) |
| Interruption | State machine handles transitions cleanly |
| 1-2s latency target | Sequential for MVP, streaming later |
| Multiple wake words | Silero wake word (reuse ONNX runtime) |

### Dialogue States
```
Idle → Listening → Processing → Speaking → (back to Idle)
         ↑                              │
         └──────── interruption ─────────┘
```

### Implementation Plan

#### Week 1: Core Pipeline + State Machine
- Dialogue Manager with state machine
- Interruption handling
- Pipeline configuration

#### Week 2: Wake Word Detection
- WakeWordBackend trait
- Multiple wake word support ("Hey Thaleia", custom)
- Connect to Idle → Listening transition

#### Week 3: Integration + Latency
- Full pipeline integration
- Streaming STT
- Latency profiling

#### Week 4: Polish + Testing
- Mode testing
- Interruption testing
- Performance testing

### Files to Create
```
crates/thaleia-core/src/pipeline/
├── mod.rs           # Pipeline exports
├── dialogue.rs      # DialogueManager + states
├── config.rs        # VoiceConfig, TriggerMode
└── events.rs       # DialogueEvent, Action

crates/thaleia-core/src/wake_word/
├── mod.rs           # WakeWordBackend trait
├── silero.rs        # Silero wake word implementation
└── config.rs       # WakeWordConfig
```

### After Pipeline
- Phase 4.2: Wake Word detection
- Phase 5.1: Streaming TTS

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
│  │  CLI (for testing)                                  │   │
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
              │  - Any MCP client      │
              └─────────────────────────┘
```

### What Thaleia Is NOT

- ❌ Not a standalone assistant with its own "brain"
- ❌ Does NOT call LLMs directly
- ❌ Does NOT have LLM backends

### Two Usage Modes

| Mode | How it works |
|------|--------------|
| **Production** | User runs LLM client with MCP - connects to Thaleia MCP server |
| **Development** | `make pipeline` - CLI tests end-to-end with OpenCode CLI |

### Pipeline End-to-End Flow

```
make pipeline
    │
    ▼
┌──────────────────────────────────────────────────────┐
│  Thaleia CLI Pipeline (Dev Mode)                     │
│  1. Spawn MCP server as subprocess                   │
│  2. Call thaleia_listen → capture + STT → text     │
│  3. Receive transcription                            │
│  4. Call OpenCode CLI → get LLM response            │
│  5. Call thaleia_speak → TTS + playback            │
│  6. Verify all steps succeeded                       │
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

### Changes Made (2026-03-29)

1. **Removed LLM module** - Deleted incorrect implementation
   - Thaleia doesn't need LLM - that's the "brain" (external)

2. **Removed LLM references** from:
   - `integration/pipeline.rs`
   - `integration/config.rs`
   - `lib.rs`
   - `Cargo.toml`

### Remaining Work

1. **Implement full pipeline** in CLI:
   - Spawn MCP server as subprocess
   - Call thaleia_listen via JSON-RPC
   - Call OpenCode CLI
   - Call thaleia_speak via JSON-RPC
   - Verify success

2. **Add MCP transport options**:
   - stdio (current, for CLI)
   - socket (future, for background server)
