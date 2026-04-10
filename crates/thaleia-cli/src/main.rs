//! Thaleia CLI - The Joyful Voice AI
//!
//! Command-line interface for Thaleia.

mod commands;

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use thaleia_core::{VERSION, init_logging, set_debug};

#[derive(Parser)]
#[command(name = "thaleia")]
#[command(about = "Thaleia - The Joyful Voice AI", long_about = None)]
#[command(version = VERSION)]
struct Cli {
    /// Enable verbose debug output
    #[arg(short, long)]
    debug: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Speak text aloud
    Speak {
        /// Text to speak
        text: String,

        /// Voice to use
        #[arg(short, long)]
        voice: Option<String>,

        /// Speech speed (0.5 - 2.0)
        #[arg(short, long, default_value_t = 1.0)]
        speed: f32,

        /// Output audio to file instead of playing
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Skip audio playback (headless mode)
        #[arg(long)]
        no_play: bool,
    },

    /// List available voices
    Voices,

    /// Listen and transcribe speech (requires microphone)
    Listen {
        /// Input audio file (for testing)
        #[arg(short, long)]
        input: Option<PathBuf>,

        /// Capture microphone and save to WAV file
        #[arg(long)]
        capture: Option<PathBuf>,

        /// Output transcription to file
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Whisper model size (tiny, base, small)
        #[arg(short, long, default_value = "tiny")]
        model: Option<String>,
    },

    /// Download Whisper models
    DownloadModels {
        /// Model size to download (if not specified, downloads all three)
        #[arg(short, long)]
        size: Option<String>,
    },

    /// Test voice pipeline with live microphone
    Pipeline {
        /// Duration to listen in seconds
        #[arg(short, long, default_value_t = 30)]
        duration: u32,

        /// Disable wake word (push-to-talk mode)
        #[arg(long)]
        no_wake_word: bool,

        /// Disable VAD (manual speech detection)
        #[arg(long)]
        no_vad: bool,
    },

    /// Run MCP server for Claude Desktop integration
    Mcp {
        /// Session mode: ephemeral, standard, or longterm
        #[arg(long, default_value = "standard")]
        mode: Option<String>,
    },

    /// Show version
    Version,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging (uses RUST_LOG env var, defaults to "info")
    init_logging();

    let cli = Cli::parse();

    // Enable debug mode if flag provided
    if cli.debug {
        set_debug(true);
        eprintln!("Thaleia CLI: debug mode enabled");
    }

    match cli.command {
        Commands::Speak {
            text,
            voice,
            speed,
            output,
            no_play,
        } => {
            commands::speak(text, voice, speed, output, no_play).await?;
        }
        Commands::Voices => {
            commands::voices().await?;
        }
        #[cfg(feature = "whisper")]
        Commands::Listen {
            input,
            capture,
            output,
            model,
        } => {
            commands::listen(input, capture, output, model).await?;
        }
        #[cfg(feature = "whisper")]
        Commands::DownloadModels { size } => {
            commands::download_models(size).await?;
        }
        Commands::Pipeline {
            duration,
            no_wake_word,
            no_vad,
        } => {
            commands::test_pipeline(duration, no_wake_word, no_vad).await?;
        }
        #[cfg(feature = "mcp")]
        Commands::Mcp { mode } => {
            commands::mcp(mode).await?;
        }
        #[cfg(not(feature = "mcp"))]
        Commands::Mcp { .. } => {
            eprintln!("Error: MCP support not enabled. Rebuild with --features mcp");
            std::process::exit(1);
        }
        Commands::Version => {
            println!("Thaleia v{}", VERSION);
        }
        #[cfg(not(feature = "whisper"))]
        Commands::Listen { .. } => {
            eprintln!("Error: Whisper support not enabled. Rebuild with --features whisper");
            std::process::exit(1);
        }
        #[cfg(not(feature = "whisper"))]
        Commands::DownloadModels { .. } => {
            eprintln!("Error: Whisper support not enabled. Rebuild with --features whisper");
            std::process::exit(1);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_parse_speak() {
        let cli = Cli::try_parse_from(["thaleia", "speak", "Hello!"]).unwrap();
        match cli.command {
            Commands::Speak { text, .. } => assert_eq!(text, "Hello!"),
            _ => panic!("Expected speak command"),
        }
    }

    #[test]
    fn test_cli_parse_voices() {
        let cli = Cli::try_parse_from(["thaleia", "voices"]).unwrap();
        match cli.command {
            Commands::Voices => {}
            _ => panic!("Expected voices command"),
        }
    }

    #[test]
    fn test_cli_parse_speak_with_voice() {
        let cli =
            Cli::try_parse_from(["thaleia", "speak", "--voice", "af_bella", "Hello!"]).unwrap();
        match cli.command {
            Commands::Speak {
                text,
                voice: Some(v),
                ..
            } => {
                assert_eq!(text, "Hello!");
                assert_eq!(v, "af_bella");
            }
            _ => panic!("Expected speak command with voice"),
        }
    }

    #[test]
    fn test_cli_parse_speak_with_speed() {
        let cli = Cli::try_parse_from(["thaleia", "speak", "--speed", "1.5", "Hello!"]).unwrap();
        match cli.command {
            Commands::Speak { text, speed, .. } => {
                assert_eq!(text, "Hello!");
                assert!((speed - 1.5).abs() < 0.001);
            }
            _ => panic!("Expected speak command with speed"),
        }
    }

    #[cfg(feature = "whisper")]
    #[test]
    fn test_cli_parse_listen() {
        let cli = Cli::try_parse_from(["thaleia", "listen"]).unwrap();
        match cli.command {
            Commands::Listen {
                input,
                capture,
                output,
                model,
            } => {
                assert!(input.is_none());
                assert!(capture.is_none());
                assert!(output.is_none());
                assert_eq!(model, Some("tiny".to_string()));
            }
            _ => panic!("Expected listen command"),
        }
    }

    #[cfg(feature = "whisper")]
    #[test]
    fn test_cli_parse_listen_with_input() {
        let cli = Cli::try_parse_from(["thaleia", "listen", "--input", "test.wav"]).unwrap();
        match cli.command {
            Commands::Listen { input, .. } => {
                assert_eq!(input, Some(PathBuf::from("test.wav")));
            }
            _ => panic!("Expected listen command with input"),
        }
    }

    #[cfg(feature = "whisper")]
    #[test]
    fn test_cli_parse_listen_with_capture() {
        let cli = Cli::try_parse_from(["thaleia", "listen", "--capture", "test.wav"]).unwrap();
        match cli.command {
            Commands::Listen { capture, .. } => {
                assert_eq!(capture, Some(PathBuf::from("test.wav")));
            }
            _ => panic!("Expected listen command with capture"),
        }
    }

    #[cfg(feature = "whisper")]
    #[test]
    fn test_cli_parse_listen_with_model() {
        let cli = Cli::try_parse_from(["thaleia", "listen", "--model", "tiny"]).unwrap();
        match cli.command {
            Commands::Listen { model, .. } => {
                assert_eq!(model, Some("tiny".to_string()));
            }
            _ => panic!("Expected listen command with model"),
        }
    }

    #[cfg(feature = "whisper")]
    #[test]
    fn test_cli_parse_download_models() {
        let cli = Cli::try_parse_from(["thaleia", "download-models"]).unwrap();
        match cli.command {
            Commands::DownloadModels { size } => {
                assert!(size.is_none());
            }
            _ => panic!("Expected download-models command"),
        }
    }

    #[cfg(feature = "whisper")]
    #[test]
    fn test_cli_parse_download_models_with_size() {
        let cli = Cli::try_parse_from(["thaleia", "download-models", "--size", "small"]).unwrap();
        match cli.command {
            Commands::DownloadModels { size } => {
                assert_eq!(size, Some("small".to_string()));
            }
            _ => panic!("Expected download-models command with size"),
        }
    }
}
