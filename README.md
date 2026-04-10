# Thaleia: The Joyful Voice AI Companion

> *"Thaleia, the Muse of Comedy and Idyllic Poetry"*

![Thaleia Banner](https://img.shields.io/badge/Thaleia-Joyful%20Voice%20AI-FF6B6B?style=for-the-badge)
![Status](https://img.shields.io/badge/Status-Pre%20Alpha-FFA500?style=for-the-badge)
![Version](https://img.shields.io/badge/Version-0.1.0--pre--alpha.1-FFA500?style=for-the-badge)
![License](https://img.shields.io/badge/License-AGPL%20+%20Commercial-00D9FF?style=for-the-badge)
![Rust](https://img.shields.io/badge/Rust-100%25-DEA584?style=for-the-badge)
![Built With](https://img.shields.io/badge/Built%20With-LLM%20Agents-9CF?style=for-the-badge)

---

## The Vision

Thaleia is an **open-source voice AI system** that brings joy to human-AI interaction. Named after the Greek Muse of comedy and idyllic poetry, Thaleia represents our belief that technology should make people **smile**.

Built with cutting-edge algorithms optimized for **lowest latency**, **highest quality**, and **maximum privacy**, Thaleia is the voice layer that any AI can use to speak and listen.

---

## ⚠️ Development Status: Pre-Alpha

*"You step into the code... It's dark. Very dark. There may be bugs here. Not the kind that crawl, but the kind that make your audio do unexpected things. Don't say I didn't warn you when things go *unexpectedly joyful*."*

**Thaleia is currently in pre-alpha testing.**

This means:
- ✅ Core audio pipeline works (capture → STT → TTS → playback)
- ✅ CLI interface functional
- ✅ MCP server compiles and connects
- ❌ Not fully battle-tested in production
- ❌ HTTP transport not yet implemented
- ❌ VAD not connected to pipeline

**Use at your own risk** - API may change between versions.

---

## 🤝 About the Creator

**Thaleia was built with LLM agent assistance, not by a Rust expert.**

The creator of Thaleia is **not a Rust developer** - they learned along the way, building this project with help from AI agents (like me!).

This means:
- The code might not be "idiomatic Rust" in places
- There may be patterns that could be improved
- Contributions from experienced Rust developers are especially welcome!

**If you're a Rust expert and see ways to improve the code - PRs are warmly welcomed!**

---

## The Character: Meet Thaleia

### Who She Is

Thaleia is the **eighth-born** of the nine Muses, daughter of Zeus and Mnemosyne. She presides over:
- **Comedy** and humorous poetry
- **Festivity** and joyful gatherings  
- **Idyllic poetry** - pastoral, celebratory verses

### How She Looks (Visual Identity)

Thaleia's personality should evoke:
- **Joy and lightness** - bright, warm colors (coral, gold, ivory)
- **Natural elements** - ivy crowns, bugles, theatrical masks
- **Playful wisdom** - she's the witty one, not the serious scholar

### How She Speaks

Thaleia's voice responses should:
- ✅ Have a warm, slightly playful tone
- ✅ Include appropriate humor when context fits
- ✅ Be clear and helpful, never robotic
- ✅ Match the joyful energy of her name

### ASCII Art Representation

```
                    ╭─────────────────╮
                    │   ✨ THALEIA ✨  │
                    │   The Joyful Muse │
                    ╰────────┬────────╯
                             │
            ┌────────────────┼────────────────┐
            │    🎭        │        🎺      │
            │  (Comedy)    │    (Celebration)│
            │                │                 │
            │   "Bringing joy to       │
            │    every conversation!" │
            │                         │
            └─────────────────────────┘
```

---

## Core Values (From AGENTS.md)

| Value | Description | Thaleia's Application |
|-------|-------------|----------------------|
| **Security First** | Never compromise security | All processing local, no cloud dependency |
| **Local First** | Process on user device | Whisper + Kokoro run entirely offline |
| **Privacy Preserving** | User data belongs to user | No telemetry, no tracking, no data collection |
| **Open Source** | Free for all | AGPL license for freedom |
| **We Build to Save Humanity** | Solve real problems | Help people interact with AI naturally |

---

## The Double License

Thaleia uses the **Double License Model**:

| License | Who Can Use | Cost |
|---------|-------------|------|
| **AGPL** | Anyone | Free |
| **Commercial** | Companies building on Thaleia | Contact for pricing |

This ensures:
- Individual developers and hobbyists can use it freely
- Companies can build products without open-sourcing their code
- Sustainable development funding

---

## Quick Start

```bash
# Install from source (Rust required)
cargo install thaleia

# Or use pre-built binary
wget https://codeberg.org/ConYel/thaleia/releases/latest/thaleia-linux-x86_64.tar.gz
tar -xzf thaleia-linux-x86_64.tar.gz
chmod +x thaleia
./thaleia speak "Hello! I'm Thaleia, ready to bring some joy to your day!"

# Run MCP server
thaleia mcp --stdio

# Interactive mode
thaleia chat
```

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                        THALEIA                                   │
│                   The Joyful Voice AI                           │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│   ┌──────────┐     ┌──────────┐     ┌──────────┐              │
│   │   CLI    │────▶│   MCP    │────▶│  TUI    │              │
│   └──────────┘     └──────────┘     └──────────┘              │
│                        │                                      │
│   ┌───────────────────┴────────────────────┐                   │
│   │              CORE ENGINE                │                   │
│   │                                        │                   │
│   │  ┌────────┐  ┌────────┐  ┌────────┐  │                   │
│   │  │   VAD   │─▶│   STT   │─▶│   TTS   │─▶│                │
│   │  │ Silero  │  │Whisper │  │ Kokoro  │  │                   │
│   │  └────────┘  └────────┘  └────────┘  │                   │
│   │                                        │                   │
│   │  ┌─────────────────────────────────┐   │                   │
│   │  │     Plugin System (Hot-reload)  │   │                   │
│   │  └─────────────────────────────────┘   │                   │
│   │                                        │                   │
│   └────────────────────────────────────────┘                   │
│                        │                                      │
│   ┌───────────────────┴────────────────────┐                 │
│   │           Audio I/O (rodio/cpal)        │                 │
│   │  🎤 Mic ────────────────────── 🔊 Out   │                 │
│   └─────────────────────────────────────────┘                 │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## Features

### Implemented ✅
- [x] **TTS Engine**: Kokoro-82M for natural, fast speech synthesis
- [x] **STT Engine**: Whisper for accurate speech recognition
- [x] **MCP Server**: Connect to Claude, Cursor, any MCP client (stdio transport)
- [x] **CLI Interface**: Simple command-line interaction
- [x] **Voice Selection**: 50+ voices with emotion support
- [x] **Audio Backends**: SDL2 + Rodio for cross-platform support
- [x] **Debug System**: Unified logging with tracing + debug macros

### In Progress 🔄
- [~] **MCP E2E**: Audio capture/playback verification pending
- [~] **Debug System**: Full end-to-end testing pending

### Planned ❌
- [ ] **VAD Integration**: Module exists, not connected to pipeline
- [ ] **Wake Word**: "Hey Thaleia" activation
- [ ] **Barge-in**: Interrupt while speaking
- [ ] **HTTP Transport**: For container deployment
- [ ] **Streaming TTS**: Chunked audio for real-time feel
- [ ] **Context Memory**: Remember conversation history
- [ ] **Voice Commands**: Execute actions by voice

### Future (If Resources Allow)
- [ ] **Unified S2S**: LLaMA-Omni integration
- [ ] **Voice Cloning**: Custom voice creation
- [ ] **Multi-language**: [Polish voice pack](./PLAN.md#community-contribution-polish-voice-pack) seeking contributors
- [ ] **Emotion Detection**: Respond to user sentiment

---

## Performance Targets

| Metric | Current Best | Thaleia Target |
|--------|-------------|----------------|
| **TTS Latency** | 200-500ms | <100ms |
| **STT Latency** | 300-500ms | <200ms |
| **E2E Latency** | 800-1500ms | <500ms |
| **Memory (CPU)** | ~2GB | <500MB |
| **Model Size** | Varies | Minimal |
| **Quality (MOS)** | 4.0-4.5 | >4.5 |

---

## Contributing

Thaleia welcomes contributions! Please see:
- [CONTRIBUTING.md](./CONTRIBUTING.md) for guidelines
- [PLAN.md](./PLAN.md) for current roadmap and progress
- [ARCHITECTURE.md](./ARCHITECTURE.md) for target architecture (aspirational)
- [RESEARCH.md](./RESEARCH.md) for scientific foundations

---

## 🤖 Built with LLM Agents

Thaleia was designed and implemented with significant assistance from LLM agents (OpenCode, Claude).

Key contributions:
- Architecture design following SOLID principles
- Implementation using Rust best practices (2026)
- Test-driven development where applicable
- Code quality enforcement (clippy, fmt)

Human oversight: All critical decisions made by humans.

---

## Acknowledgments

Thaleia stands on the shoulders of giants:
- **Whisper** (OpenAI) - Speech recognition
- **Kokoro** (AI Foundation) - Text-to-speech
- **Silero** - Voice activity detection
- **rust-mcp** - MCP protocol implementation
- **All the Muses** - For inspiring us

---

## License

Copyright 2026 Thaleia Project

Dual License:
- **AGPL v3**: Open source, free for non-commercial use
- **Commercial License**: Required for commercial products

See [LICENSE](./LICENSE) and [LICENSE-COMMERCIAL](./LICENSE-COMMERCIAL) for details.

---

*"Thaleia reminds us that technology should bring joy, not frustration."*
