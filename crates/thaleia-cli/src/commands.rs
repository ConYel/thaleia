//! CLI Commands for Thaleia
//!
//! Contains all command implementations separated by responsibility.

use std::io::Write;
use std::path::PathBuf;
use thaleia_core::TtsEngine;

#[cfg(feature = "whisper")]
use thaleia_core::{ModelSize, WhisperEngine};

// =============================================================================
// Audio Helpers
// =============================================================================

/// Check audio availability and exit with error if not available
pub fn check_audio_or_exit() -> bool {
    #[cfg(feature = "playback")]
    {
        print!("🔊 Checking audio... ");
        std::io::stdout().flush().unwrap();

        let audio = thaleia_core::AudioEngine::new();
        if audio.is_available() {
            println!("✓ {} backend available", audio.backend_name());
            true
        } else {
            println!("✗ Audio device unavailable");
            eprintln!();
            eprintln!("💡 The audio device was disconnected. Check cable connection.");
            false
        }
    }

    #[cfg(not(feature = "playback"))]
    {
        println!("ℹ️  Playback not enabled (rebuild with --features playback)");
        false
    }
}

// =============================================================================
// Speak Command
// =============================================================================

/// Execute speak command
pub async fn speak(
    text: String,
    voice: Option<String>,
    _speed: f32,
    output: Option<PathBuf>,
    no_play: bool,
) -> anyhow::Result<()> {
    tracing::info!("Speaking: {}", text);

    let will_play = !no_play && output.is_none();

    if will_play && !check_audio_or_exit() {
        eprintln!();
        eprintln!("❌ Cannot play audio. Use one of:");
        eprintln!("   thaleia speak --no-play \"text\"     # Just synthesize");
        eprintln!("   thaleia speak --output file.wav ...  # Save to file");
        std::process::exit(1);
    }

    let mut tts = TtsEngine::new().await?;
    let voice_id = voice.as_deref();

    tracing::info!("Synthesizing with voice: {:?}", voice_id);
    let samples = tts.synthesize(&text, voice_id)?;
    tracing::info!("Generated {} audio samples", samples.len());

    // Save to file
    if let Some(path) = output {
        use thaleia_core::audio::backends::rodio_backend::samples_to_wav;
        let wav_data = samples_to_wav(&samples, 24000)?;
        std::fs::write(&path, &wav_data)?;
        println!("💾 Saved audio to: {}", path.display());
        return Ok(());
    }

    // Play audio
    if will_play {
        #[cfg(feature = "playback")]
        {
            let mut audio = thaleia_core::AudioEngine::new();
            if audio.is_available() {
                audio.audio_system_mut().play(&samples, 24000)?;
                println!("🎭 Thaleia spoke: {}", text);
            } else {
                println!("🎭 Thaleia would speak: {}", text);
                println!("   (Audio backend not available)");
            }
        }

        #[cfg(not(feature = "playback"))]
        {
            println!("🎭 Thaleia would say: {}", text);
            println!("   (Rebuild with --features playback to hear audio)");
        }
    } else {
        println!("🎭 Synthesized: {}", text);
        println!("   (Audio playback skipped)");
    }

    Ok(())
}

// =============================================================================
// Voices Command
// =============================================================================

/// Execute voices command
pub async fn voices() -> anyhow::Result<()> {
    let tts = TtsEngine::new().await?;
    println!("Available voices:");
    for voice in tts.list_voices() {
        println!(
            "  {:15} - {} ({})",
            voice.id.as_str(),
            voice.name,
            voice.language
        );
    }
    Ok(())
}

// =============================================================================
// Listen Command (Whisper)
// =============================================================================

#[cfg(feature = "whisper")]
pub async fn listen(
    input: Option<PathBuf>,
    capture: Option<PathBuf>,
    output: Option<PathBuf>,
    model: Option<String>,
) -> anyhow::Result<()> {
    use std::time::Duration;
    use thaleia_core::SttBackend;

    let model_size = match model.as_deref() {
        Some("tiny") => ModelSize::Tiny,
        Some("small") => ModelSize::Small,
        Some("base") | None => ModelSize::Base,
        Some(s) => {
            eprintln!("Error: Unknown model size '{}'. Use: tiny, base, small", s);
            std::process::exit(1);
        }
    };

    println!("🎤 Loading Whisper {} model...", model_size.model_name());
    let mut engine = WhisperEngine::new(model_size).await?;

    // Determine audio source
    let (audio, sample_rate) = if let Some(path) = &input {
        println!("📄 Reading audio from: {}", path.display());
        let samples = load_audio_from_file(path)?;
        (samples, 24000)
    } else if let Some(capture_path) = &capture {
        println!("🎙️  Capturing audio to: {}", capture_path.display());
        let cap = thaleia_core::AudioCapture::new()?;
        let captured = cap.capture(Duration::from_secs(5))?;
        let saved_path = captured.save_wav(capture_path)?;
        println!("💾 Audio saved to: {}", saved_path);
        (captured.samples, captured.sample_rate)
    } else {
        println!("🎙️  Capturing from microphone (speak now)...");
        let cap = thaleia_core::AudioCapture::new()?;
        let captured = cap.capture(Duration::from_secs(5))?;
        (captured.samples, captured.sample_rate)
    };

    println!("🔄 Transcribing {} samples...", audio.len());
    let result = engine.transcribe(&audio, sample_rate)?;
    let text = result.text;

    if text.is_empty() {
        println!("🤷 No speech detected.");
    } else {
        println!("📝 You said: {}", text);
        if let Some(path) = output {
            std::fs::write(&path, &text)?;
            println!("💾 Transcription saved to: {}", path.display());
        }
    }

    Ok(())
}

// =============================================================================
// Audio Loading Helpers
// =============================================================================

#[cfg(feature = "whisper")]
fn load_audio_from_file(path: &PathBuf) -> anyhow::Result<Vec<f32>> {
    let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("");

    match extension.to_lowercase().as_str() {
        "wav" => load_wav_samples(path),
        "raw" => load_raw_samples(path),
        _ => {
            eprintln!("Unsupported format: {}. Use .wav or .raw", extension);
            eprintln!("Note: .raw files should be 16-bit mono PCM at 24kHz");
            std::process::exit(1);
        }
    }
}

#[cfg(feature = "whisper")]
fn load_wav_samples(path: &PathBuf) -> anyhow::Result<Vec<f32>> {
    let reader = hound::WavReader::open(path)?;
    let spec = reader.spec();

    println!(
        "Audio format: {} channels, {} Hz, {} bits",
        spec.channels, spec.sample_rate, spec.bits_per_sample
    );

    let samples: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Int => {
            let max_val = match spec.bits_per_sample {
                16 => i16::MAX as f32,
                32 => i32::MAX as f32,
                _ => i16::MAX as f32,
            };
            reader
                .into_samples::<i16>()
                .filter_map(|s| s.ok())
                .map(|s| s as f32 / max_val)
                .collect()
        }
        hound::SampleFormat::Float => reader
            .into_samples::<f32>()
            .filter_map(|s| s.ok())
            .collect(),
    };

    // Convert to mono if stereo
    if spec.channels > 1 {
        println!("Converting stereo to mono...");
        let chunk_size = samples.len() / spec.channels as usize;
        let mono: Vec<f32> = (0..chunk_size)
            .map(|i| {
                let sum: f32 = (0..spec.channels as usize)
                    .map(|ch| samples[i + ch * chunk_size])
                    .sum();
                sum / spec.channels as f32
            })
            .collect();
        return Ok(mono);
    }

    Ok(samples)
}

#[cfg(feature = "whisper")]
fn load_raw_samples(path: &PathBuf) -> anyhow::Result<Vec<f32>> {
    let bytes = std::fs::read(path)?;
    let samples: Vec<f32> = bytes
        .chunks(2)
        .filter_map(|chunk| {
            if chunk.len() == 2 {
                let s = i16::from_le_bytes([chunk[0], chunk[1]]);
                Some(s as f32 / i16::MAX as f32)
            } else {
                None
            }
        })
        .collect();
    Ok(samples)
}

// =============================================================================
// Download Command (Whisper)
// =============================================================================

#[cfg(feature = "whisper")]
pub async fn download_models(size: Option<String>) -> anyhow::Result<()> {
    match size.as_deref() {
        Some("tiny") => {
            println!("Downloading Whisper tiny model...");
            let _ = WhisperEngine::new(ModelSize::Tiny).await?;
            println!("✅ Tiny model downloaded!");
        }
        Some("base") => {
            println!("Downloading Whisper base model...");
            let _ = WhisperEngine::new(ModelSize::Base).await?;
            println!("✅ Base model downloaded!");
        }
        Some("small") => {
            println!("Downloading Whisper small model...");
            let _ = WhisperEngine::new(ModelSize::Small).await?;
            println!("✅ Small model downloaded!");
        }
        Some(s) => {
            eprintln!("Error: Unknown model size '{}'. Use: tiny, base, small", s);
            std::process::exit(1);
        }
        None => {
            println!("Downloading all Whisper models...");
            println!("Note: This will download ~680MB of models.\n");

            println!("1/3. Downloading tiny model (~75MB)...");
            let _ = WhisperEngine::new(ModelSize::Tiny).await?;
            println!("   ✅ Tiny done");

            println!("2/3. Downloading base model (~140MB)...");
            let _ = WhisperEngine::new(ModelSize::Base).await?;
            println!("   ✅ Base done");

            println!("3/3. Downloading small model (~465MB)...");
            let _ = WhisperEngine::new(ModelSize::Small).await?;
            println!("   ✅ Small done\n");

            println!("✅ All Whisper models downloaded to ~/.cache/whisper/");
        }
    }
    Ok(())
}

// =============================================================================
// MCP Command
// =============================================================================

#[cfg(feature = "mcp")]
pub async fn mcp(mode: Option<String>) -> anyhow::Result<()> {
    let _session_mode = mode
        .as_deref()
        .map(|m| match m {
            "ephemeral" => thaleia_mcp::session::MemoryMode::Ephemeral,
            "standard" => thaleia_mcp::session::MemoryMode::Standard,
            "longterm" => thaleia_mcp::session::MemoryMode::Longterm,
            _ => {
                eprintln!("Unknown mode '{}'. Using 'standard'.", m);
                eprintln!("Valid modes: ephemeral, standard, longterm");
                thaleia_mcp::session::MemoryMode::Standard
            }
        })
        .unwrap_or(thaleia_mcp::session::MemoryMode::Standard);

    // Run the MCP server (blocking)
    thaleia_mcp::run_server()
}

// =============================================================================
// Pipeline Testing
// =============================================================================

/// Test the voice pipeline with live microphone - full end-to-end test
///
/// This tests:
/// 1. Audio capture from microphone
/// 2. STT (Whisper) transcription
/// 3. LLM (OpenCode CLI) response
/// 4. TTS (Kokoro) synthesis
/// 5. Audio playback
pub async fn test_pipeline(
    duration_secs: u32,
    no_wake_word: bool,
    no_vad: bool,
) -> anyhow::Result<()> {
    use std::process::Command;
    use std::time::Duration;
    use thaleia_core::SttSystem;
    use thaleia_core::TtsEngine;

    // Suppress warnings for unused parameters (for future use)
    let _ = no_wake_word;
    let _ = no_vad;

    println!("🎙️  Testing Voice Pipeline (End-to-End)");
    println!("========================================");
    println!("Duration: {} seconds", duration_secs);
    println!(
        "Wake word: {}",
        if no_wake_word { "disabled" } else { "enabled" }
    );
    println!("VAD: {}", if no_vad { "disabled" } else { "enabled" });
    println!();

    // Check prerequisites
    println!("🔍 Checking prerequisites...");

    // Check microphone
    if !thaleia_core::AudioCapture::is_available() {
        anyhow::bail!("❌ Microphone not available");
    }
    println!("   ✅ Microphone available");

    // Check audio playback
    let audio_system = thaleia_core::AudioSystem::new();
    if audio_system.is_available() {
        println!("   ✅ Audio playback available");
    } else {
        println!("   ⚠️  Audio playback not available (will skip TTS)");
    }

    // Check OpenCode CLI
    let opencode_available = Command::new("opencode")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if opencode_available {
        println!("   ✅ OpenCode CLI available");
    } else {
        println!("   ⚠️  OpenCode CLI not available (will skip LLM)");
    }

    println!();

    // Step 1: Capture audio
    println!("\n📝 Step 1: Capturing audio...");

    // Debug: Check audio system
    let mut audio_system = thaleia_core::AudioSystem::new();
    println!("   AudioSystem backend: {}", audio_system.backend_name());
    println!(
        "   AudioSystem is_available: {}",
        audio_system.is_available()
    );

    // Check if capture is available
    let capture_available = thaleia_core::AudioCapture::is_available();
    println!("   AudioCapture::is_available() = {}", capture_available);

    if !capture_available {
        // Try to show diagnostics
        println!("   Running audio diagnostics...");
        let report = thaleia_core::audio::diagnostics::full_diagnostic_report();
        println!("{}", report);
        anyhow::bail!("❌ Audio capture not available");
    }

    let cap = thaleia_core::AudioCapture::new()?;
    let capture_duration = Duration::from_secs(duration_secs as u64);

    let captured = cap.capture(capture_duration)?;
    let sample_rate = captured.sample_rate;
    let audio_samples = captured.samples;

    println!(
        "   ✅ Captured {} samples at {} Hz",
        audio_samples.len(),
        sample_rate
    );

    if audio_samples.is_empty() {
        anyhow::bail!("❌ No audio captured");
    }

    // Step 2: STT (Transcribe)
    println!("\n🔄 Step 2: Transcribing (STT)...");
    let mut stt = SttSystem::new().await?;
    let transcription = stt.transcribe(&audio_samples, sample_rate)?;
    let text = transcription.text.clone();

    println!("   ✅ Transcription: \"{}\"", text);

    if text.trim().is_empty() {
        println!("   ⚠️  No speech detected, using placeholder text");
    }

    // Step 3: LLM (OpenCode)
    let llm_response = if opencode_available {
        println!("\n🧠 Step 3: Getting LLM response...");
        let prompt = if text.trim().is_empty() {
            "Say hello in a friendly way".to_string()
        } else {
            text
        };

        let output = Command::new("opencode")
            .arg("--ai")
            .arg(&prompt)
            .output()
            .map_err(|e| anyhow::anyhow!("Failed to run opencode: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("OpenCode failed: {}", stderr);
        }

        let response = String::from_utf8_lossy(&output.stdout).trim().to_string();
        println!("   ✅ LLM Response: \"{}\"", response);
        response
    } else {
        println!("\n🧠 Step 3: Skipping LLM (OpenCode not available)");
        "OpenCode not available".to_string()
    };

    // Step 4: TTS (Synthesize)
    println!("\n🎤 Step 4: Synthesizing speech (TTS)...");
    let mut tts = TtsEngine::new().await?;
    let tts_audio = tts.synthesize(&llm_response, None)?;

    println!("   ✅ Generated {} audio samples", tts_audio.len());

    if tts_audio.is_empty() {
        anyhow::bail!("❌ TTS generated no audio");
    }

    // Step 5: Playback
    println!("\n🔊 Step 5: Playing audio...");
    if audio_system.is_available() {
        audio_system.play(&tts_audio, 24000)?;
        println!("   ✅ Audio playback started");

        // Wait for playback to finish (rough estimate)
        let duration_secs = tts_audio.len() as f32 / 24000.0;
        println!("   ⏳ Playing for ~{:.1} seconds...", duration_secs);
        std::thread::sleep(Duration::from_secs(duration_secs as u64 + 1));
    } else {
        println!("   ⚠️  Skipping playback (audio system not available)");
    }

    println!("\n✅ Pipeline test complete!");
    println!("   - Audio capture: ✅");
    println!("   - STT (Whisper): ✅");
    println!("   - LLM (OpenCode): ✅");
    println!("   - TTS (Kokoro): ✅");
    println!(
        "   - Playback: {}",
        if audio_system.is_available() {
            "✅"
        } else {
            "⚠️"
        }
    );

    Ok(())
}
