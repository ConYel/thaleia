//! Download progress tracking for Thaleia
//!
//! Provides a clean progress bar for downloading AI models.

use std::io::Write;
use std::time::Duration;
use tokio::io::AsyncWriteExt;

/// Progress bar for terminal output
pub struct ProgressBar {
    total: u64,
    current: u64,
    width: usize,
    start_time: std::time::Instant,
}

impl ProgressBar {
    /// Create a new progress bar
    pub fn new(total: u64) -> Self {
        Self {
            total,
            current: 0,
            width: 50, // Width of progress bar in terminal
            start_time: std::time::Instant::now(),
        }
    }

    /// Update progress
    pub fn update(&mut self, current: u64) {
        self.current = current;
    }

    /// Increment by amount
    pub fn inc(&mut self, amount: u64) {
        self.current += amount;
    }

    /// Set to complete
    pub fn finish(&mut self) {
        self.current = self.total;
        self.draw();
        println!();
    }

    /// Draw the progress bar
    fn draw(&self) {
        let elapsed = self.start_time.elapsed();
        let rate = if elapsed > Duration::from_secs(0) {
            self.current as f64 / elapsed.as_secs_f64()
        } else {
            0.0
        };

        let percent = if self.total > 0 {
            (self.current as f64 / self.total as f64 * 100.0) as u32
        } else {
            100
        };

        let filled = if self.total > 0 {
            ((self.current as f64 / self.total as f64) * self.width as f64) as usize
        } else {
            self.width
        };

        let empty = self.width.saturating_sub(filled);

        // Format size
        let current_str = format_bytes(self.current);
        let total_str = format_bytes(self.total);

        // Build the line and overwrite
        let progress_str = format!(
            "\r🎭 Downloading model: [{}{}] {}% ({}/s) [{}/{}]",
            "█".repeat(filled),
            "░".repeat(empty),
            percent,
            format_bytes(rate as u64) + "/s",
            current_str,
            total_str
        );

        // Clear line and print
        print!("{:<90}", progress_str);
        std::io::stdout().flush().ok();
    }
}

impl Drop for ProgressBar {
    fn drop(&mut self) {
        if self.current > 0 && self.current < self.total {
            self.finish();
        }
    }
}

/// Format bytes as human-readable string
fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.1}GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1}MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1}KB", bytes as f64 / KB as f64)
    } else {
        format!("{}B", bytes)
    }
}

/// Download a file with progress tracking
pub async fn download_with_progress(
    url: &str,
    path: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use futures_util::StreamExt;
    use reqwest::Client;

    println!("🎭 Downloading model...");

    let client = Client::new();
    let response = client.get(url).send().await?;

    let total_size = response.content_length().unwrap_or(0);

    let mut pb = ProgressBar::new(total_size);
    let mut downloaded: u64 = 0;

    let mut file = tokio::fs::File::create(path).await?;
    let mut stream = response.bytes_stream();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        file.write_all(&chunk).await?;
        downloaded += chunk.len() as u64;
        pb.update(downloaded);
    }

    pb.finish();
    println!("✅ Model downloaded to {:?}", path);

    Ok(())
}

/// Check if model is cached and download if needed
pub async fn ensure_model_cached()
-> Result<std::path::PathBuf, Box<dyn std::error::Error + Send + Sync>> {
    use home::home_dir;

    let cache_dir = home_dir()
        .ok_or("Could not find home directory")?
        .join(".cache/kokoro");

    tokio::fs::create_dir_all(&cache_dir).await?;

    let model_path = cache_dir.join("kokoro-v1.0.onnx");

    // Check if already downloaded
    if model_path.exists() {
        let metadata = tokio::fs::metadata(&model_path).await?;
        println!("🎭 Model cached ({})", format_bytes(metadata.len()));
        return Ok(model_path);
    }

    // Download with progress
    let url = "https://github.com/thewh1teagle/kokoro-onnx/releases/download/model-files-v1.0/kokoro-v1.0.onnx";
    download_with_progress(url, &model_path).await?;

    Ok(model_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(500), "500B");
        assert_eq!(format_bytes(1024), "1.0KB");
        assert_eq!(format_bytes(1024 * 1024), "1.0MB");
        assert_eq!(format_bytes(1024 * 1024 * 1024), "1.0GB");
    }
}
