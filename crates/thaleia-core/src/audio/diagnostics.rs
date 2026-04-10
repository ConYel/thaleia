//! Audio diagnostics module
//!
//! Provides tools for debugging audio issues and checking backend availability.
//!
//! ## Usage
//!
//! ```rust
//! use thaleia_core::audio::diagnostics;
//!
//! // Get detailed diagnostic report
//! let report = diagnostics::full_diagnostic_report();
//! println!("{}", report);
//!
//! // Check specific backends
//! #[cfg(feature = "rodio")]
//! println!("Rodio: {}", diagnostics::is_rodio_available());
//! #[cfg(feature = "sdl2-audio")]
//! println!("SDL2: {}", diagnostics::is_sdl2_available());
//!
//! // List audio devices
//! diagnostics::list_devices();
//! ```

use crate::audio::backends::AudioBackend;
use crate::audio::backends::AudioSystem;

/// Check if Rodio/cpal backend is available
#[cfg(feature = "rodio")]
pub fn is_rodio_available() -> bool {
    use crate::audio::backends::rodio_backend::RodioBackend;
    RodioBackend::new()
        .map(|b| AudioBackend::is_available(&b))
        .unwrap_or(false)
}

/// Check if SDL2 backend is available
#[cfg(feature = "sdl2-audio")]
pub fn is_sdl2_available() -> bool {
    use crate::audio::backends::sdl2_backend::SDL2Backend;
    SDL2Backend::new()
        .map(|b| AudioBackend::is_available(&b))
        .unwrap_or(false)
}

/// Get a diagnostic report for audio systems
///
/// Returns a human-readable string describing:
/// - Which backends are available
/// - Which backend would be auto-selected
/// - Audio device information
pub fn full_diagnostic_report() -> String {
    let mut report = String::new();

    report.push_str("=== Audio Diagnostics ===\n\n");

    // Backend availability
    report.push_str("Backend Availability:\n");

    #[cfg(feature = "rodio")]
    report.push_str(&format!(
        "  Rodio (ALSA):     {}\n",
        if is_rodio_available() {
            "Available"
        } else {
            "Not available"
        }
    ));

    #[cfg(feature = "sdl2-audio")]
    report.push_str(&format!(
        "  SDL2 (PulseAudio): {}\n",
        if is_sdl2_available() {
            "Available"
        } else {
            "Not available"
        }
    ));

    // Auto-selected backend
    report.push_str("\nAuto-selected Backend:\n");
    let system = AudioSystem::new();
    match system {
        #[cfg(feature = "rodio")]
        AudioSystem::Rodio(_) => {
            report.push_str("  rodio (ALSA) - Standard Linux audio\n");
        }
        #[cfg(feature = "sdl2-audio")]
        AudioSystem::SDL2(_) => {
            report.push_str("  sdl2 (PulseAudio) - Qubes/custom systems\n");
        }
        AudioSystem::None => {
            report.push_str("  none - No audio backend available\n");
            report.push_str("  Use: thaleia speak --output file.wav\n");
        }
    }

    // Backend details
    report.push_str(&format!("\nCurrent Backend: {}\n", system.backend_name()));

    // Hints
    report.push_str("\nHints:\n");

    #[cfg(all(feature = "rodio", feature = "sdl2-audio"))]
    if !is_rodio_available() && !is_sdl2_available() {
        report.push_str("  - No audio backends available\n");
        report.push_str("  - Check audio device configuration\n");
        report.push_str("  - On Qubes: Set up sys-audio qube\n");
        report.push_str("  - Use file-based output: thaleia speak --output audio.wav\n");
    } else if !is_rodio_available() && is_sdl2_available() {
        report.push_str("  - Only SDL2 available (Qubes/PulseAudio mode)\n");
        report.push_str("  - Audio will use PulseAudio\n");
    }

    #[cfg(all(feature = "rodio", not(feature = "sdl2-audio")))]
    if !is_rodio_available() {
        report.push_str("  - No audio backends available\n");
        report.push_str("  - Check audio device configuration\n");
        report.push_str("  - Use file-based output: thaleia speak --output audio.wav\n");
    }

    #[cfg(all(not(feature = "rodio"), feature = "sdl2-audio"))]
    if !is_sdl2_available() {
        report.push_str("  - No audio backends available\n");
        report.push_str("  - Check audio device configuration\n");
        report.push_str("  - On Qubes: Set up sys-audio qube\n");
        report.push_str("  - Use file-based output: thaleia speak --output audio.wav\n");
    }

    #[cfg(all(not(feature = "rodio"), not(feature = "sdl2-audio")))]
    {
        report.push_str("  - No audio backends compiled in\n");
        report.push_str("  - Enable audio features for playback/capture\n");
        report.push_str("  - Use file-based output: thaleia speak --output audio.wav\n");
    }

    report
}

/// List audio devices (using available backends)
pub fn list_devices() {
    println!("{}", full_diagnostic_report());
}

/// Print audio status for CLI
pub fn print_status() {
    let system = AudioSystem::new();

    if system.is_available() {
        println!("🔊 Audio: {} backend", system.backend_name());
    } else {
        println!("🔇 Audio: Not available (file-only mode)");
    }
}

/// Categorize audio error for better user messages
pub fn categorize_error(error: &str) -> (&'static str, &'static str) {
    let error_lower = error.to_lowercase();

    if error_lower.contains("no such file") || error_lower.contains("unknown pcm") {
        (
            "No audio device found",
            "Check that speakers/headphones are connected. \
             On Qubes: Set up sys-audio qube.",
        )
    } else if error_lower.contains("permission") || error_lower.contains("denied") {
        (
            "Audio permission denied",
            "Your user may not have access to audio. \
             Try adding yourself to the 'audio' group.",
        )
    } else if error_lower.contains("device") || error_lower.contains("unplugged") {
        (
            "Audio device unavailable",
            "The audio device was disconnected. Check cable connection.",
        )
    } else if error_lower.contains("exclusive") || error_lower.contains("busy") {
        (
            "Audio device in use",
            "Another application may be using the audio device. \
             Close other audio apps and try again.",
        )
    } else {
        (
            "Audio error",
            "Check audio configuration. Use --output for file-based output.",
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diagnostic_report() {
        let report = full_diagnostic_report();
        assert!(!report.is_empty());
        assert!(report.contains("Backend Availability"));
    }

    #[test]
    fn test_categorize_error() {
        let (title, _) = categorize_error("No such file or directory");
        assert_eq!(title, "No audio device found");
    }
}
