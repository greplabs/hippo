//! Qdrant process manager
//!
//! Handles automatic download, installation, and lifecycle management of Qdrant.

use crate::error::{HippoError, Result};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

/// Qdrant version to download
#[allow(dead_code)]
const QDRANT_VERSION: &str = "v1.12.4";

/// Get the download URL for the current platform
fn get_download_url() -> Option<&'static str> {
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    return Some(concat!(
        "https://github.com/qdrant/qdrant/releases/download/v1.12.4/",
        "qdrant-x86_64-apple-darwin.tar.gz"
    ));

    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    return Some(concat!(
        "https://github.com/qdrant/qdrant/releases/download/v1.12.4/",
        "qdrant-aarch64-apple-darwin.tar.gz"
    ));

    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    return Some(concat!(
        "https://github.com/qdrant/qdrant/releases/download/v1.12.4/",
        "qdrant-x86_64-unknown-linux-musl.tar.gz"
    ));

    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    return Some(concat!(
        "https://github.com/qdrant/qdrant/releases/download/v1.12.4/",
        "qdrant-x86_64-pc-windows-msvc.zip"
    ));

    #[cfg(not(any(
        all(target_os = "macos", target_arch = "x86_64"),
        all(target_os = "macos", target_arch = "aarch64"),
        all(target_os = "linux", target_arch = "x86_64"),
        all(target_os = "windows", target_arch = "x86_64"),
    )))]
    return None;
}

/// Get the binary name for the current platform
fn get_binary_name() -> &'static str {
    #[cfg(target_os = "windows")]
    return "qdrant.exe";

    #[cfg(not(target_os = "windows"))]
    return "qdrant";
}

/// Status of Qdrant
#[derive(Debug, Clone, serde::Serialize)]
pub struct QdrantStatus {
    /// Whether Qdrant is available and responding
    pub available: bool,
    /// Whether Qdrant was started by us (managed)
    pub managed: bool,
    /// Whether the Qdrant binary is installed locally
    pub installed: bool,
    /// The URL Qdrant is running on
    pub url: String,
    /// Version string if available
    pub version: Option<String>,
    /// PID if managed process
    pub pid: Option<u32>,
    /// Path to the binary
    pub binary_path: Option<String>,
    /// Current download progress (0-100) if downloading
    pub download_progress: Option<u8>,
    /// Any status message
    pub message: String,
}

/// Manages Qdrant lifecycle
pub struct QdrantManager {
    /// Directory where Qdrant binary and data are stored
    data_dir: PathBuf,
    /// URL to connect to Qdrant
    url: String,
    /// Managed Qdrant process (if started by us)
    process: Arc<RwLock<Option<Child>>>,
    /// Current status
    status: Arc<RwLock<QdrantStatus>>,
}

impl QdrantManager {
    /// Create a new Qdrant manager
    pub fn new(data_dir: PathBuf, url: &str) -> Self {
        let qdrant_dir = data_dir.join("qdrant");
        let binary_path = qdrant_dir.join(get_binary_name());

        Self {
            data_dir: qdrant_dir,
            url: url.to_string(),
            process: Arc::new(RwLock::new(None)),
            status: Arc::new(RwLock::new(QdrantStatus {
                available: false,
                managed: false,
                installed: binary_path.exists(),
                url: url.to_string(),
                version: None,
                pid: None,
                binary_path: if binary_path.exists() {
                    Some(binary_path.to_string_lossy().to_string())
                } else {
                    None
                },
                download_progress: None,
                message: "Not started".to_string(),
            })),
        }
    }

    /// Get the current status
    pub async fn status(&self) -> QdrantStatus {
        self.status.read().await.clone()
    }

    /// Check if Qdrant is available (running and responding)
    pub async fn check_health(&self) -> bool {
        let client = reqwest::Client::new();
        let health_url = format!("{}/", self.url.replace("6334", "6333"));

        match client.get(&health_url).timeout(std::time::Duration::from_secs(2)).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    let mut status = self.status.write().await;
                    status.available = true;
                    status.message = "Running".to_string();
                    true
                } else {
                    false
                }
            }
            Err(_) => false,
        }
    }

    /// Get the path to the Qdrant binary
    fn binary_path(&self) -> PathBuf {
        self.data_dir.join(get_binary_name())
    }

    /// Check if Qdrant binary is installed
    pub fn is_installed(&self) -> bool {
        self.binary_path().exists()
    }

    /// Download and install Qdrant
    pub async fn install(&self) -> Result<()> {
        let download_url = get_download_url().ok_or_else(|| {
            HippoError::Other("Qdrant is not available for this platform".to_string())
        })?;

        info!("Downloading Qdrant from {}", download_url);

        {
            let mut status = self.status.write().await;
            status.download_progress = Some(0);
            status.message = "Downloading Qdrant...".to_string();
        }

        // Create the qdrant directory
        std::fs::create_dir_all(&self.data_dir)?;

        // Download the archive
        let client = reqwest::Client::new();
        let response = client
            .get(download_url)
            .send()
            .await
            .map_err(|e| HippoError::Other(format!("Failed to download Qdrant: {}", e)))?;

        if !response.status().is_success() {
            return Err(HippoError::Other(format!(
                "Failed to download Qdrant: HTTP {}",
                response.status()
            )));
        }

        let _content_length = response.content_length().unwrap_or(0);
        let bytes = response
            .bytes()
            .await
            .map_err(|e| HippoError::Other(format!("Failed to download Qdrant: {}", e)))?;

        {
            let mut status = self.status.write().await;
            status.download_progress = Some(50);
            status.message = "Extracting Qdrant...".to_string();
        }

        // Extract based on file type
        let archive_path = self.data_dir.join("qdrant_download");
        std::fs::write(&archive_path, &bytes)?;

        #[cfg(target_os = "windows")]
        {
            // Extract ZIP on Windows
            let file = std::fs::File::open(&archive_path)?;
            let mut archive = zip::ZipArchive::new(file)
                .map_err(|e| HippoError::Other(format!("Failed to open ZIP: {}", e)))?;

            for i in 0..archive.len() {
                let mut file = archive.by_index(i)
                    .map_err(|e| HippoError::Other(format!("Failed to read ZIP entry: {}", e)))?;

                if file.name().ends_with("qdrant.exe") {
                    let out_path = self.binary_path();
                    let mut outfile = std::fs::File::create(&out_path)?;
                    std::io::copy(&mut file, &mut outfile)?;
                    break;
                }
            }
        }

        #[cfg(not(target_os = "windows"))]
        {
            // Extract tar.gz on Unix
            use flate2::read::GzDecoder;
            use tar::Archive;

            let file = std::fs::File::open(&archive_path)?;
            let decoder = GzDecoder::new(file);
            let mut archive = Archive::new(decoder);

            for entry in archive.entries()? {
                let mut entry = entry?;
                let path = entry.path()?;

                if path.file_name().map(|n| n == "qdrant").unwrap_or(false) {
                    let out_path = self.binary_path();
                    entry.unpack(&out_path)?;

                    // Make executable
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::PermissionsExt;
                        let mut perms = std::fs::metadata(&out_path)?.permissions();
                        perms.set_mode(0o755);
                        std::fs::set_permissions(&out_path, perms)?;
                    }
                    break;
                }
            }
        }

        // Clean up
        let _ = std::fs::remove_file(&archive_path);

        {
            let mut status = self.status.write().await;
            status.download_progress = Some(100);
            status.installed = true;
            status.binary_path = Some(self.binary_path().to_string_lossy().to_string());
            status.message = "Qdrant installed successfully".to_string();
        }

        info!("Qdrant installed successfully at {:?}", self.binary_path());
        Ok(())
    }

    /// Start Qdrant process
    pub async fn start(&self) -> Result<()> {
        // First check if already running externally
        if self.check_health().await {
            info!("Qdrant is already running externally");
            let mut status = self.status.write().await;
            status.managed = false;
            return Ok(());
        }

        // Check if installed
        if !self.is_installed() {
            self.install().await?;
        }

        let binary_path = self.binary_path();
        let storage_path = self.data_dir.join("storage");
        std::fs::create_dir_all(&storage_path)?;

        info!("Starting Qdrant from {:?}", binary_path);

        {
            let mut status = self.status.write().await;
            status.message = "Starting Qdrant...".to_string();
        }

        // Start Qdrant process
        let child = Command::new(&binary_path)
            .arg("--storage-path")
            .arg(&storage_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| HippoError::Other(format!("Failed to start Qdrant: {}", e)))?;

        let pid = child.id();

        {
            let mut process = self.process.write().await;
            *process = Some(child);
        }

        // Wait for Qdrant to be ready (up to 30 seconds)
        let start_time = std::time::Instant::now();
        let timeout = std::time::Duration::from_secs(30);

        while start_time.elapsed() < timeout {
            if self.check_health().await {
                let mut status = self.status.write().await;
                status.managed = true;
                status.pid = Some(pid);
                status.message = "Running (managed)".to_string();
                info!("Qdrant started successfully (PID: {})", pid);
                return Ok(());
            }
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }

        // Failed to start
        {
            let mut process = self.process.write().await;
            if let Some(mut child) = process.take() {
                let _ = child.kill();
            }
        }

        {
            let mut status = self.status.write().await;
            status.message = "Failed to start Qdrant".to_string();
        }

        Err(HippoError::Other("Qdrant failed to start within 30 seconds".to_string()))
    }

    /// Stop Qdrant process (if managed)
    pub async fn stop(&self) -> Result<()> {
        let mut process = self.process.write().await;

        if let Some(mut child) = process.take() {
            info!("Stopping Qdrant process");
            let _ = child.kill();
            let _ = child.wait();
        }

        let mut status = self.status.write().await;
        status.available = false;
        status.managed = false;
        status.pid = None;
        status.message = "Stopped".to_string();

        Ok(())
    }

    /// Ensure Qdrant is running (start if needed)
    pub async fn ensure_running(&self) -> Result<()> {
        if self.check_health().await {
            return Ok(());
        }
        self.start().await
    }
}

impl Drop for QdrantManager {
    fn drop(&mut self) {
        // Try to stop the process synchronously on drop
        if let Ok(mut guard) = self.process.try_write() {
            if let Some(mut child) = guard.take() {
                debug!("Stopping Qdrant on drop");
                let _ = child.kill();
                let _ = child.wait();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_binary_name() {
        let name = get_binary_name();
        #[cfg(target_os = "windows")]
        assert_eq!(name, "qdrant.exe");
        #[cfg(not(target_os = "windows"))]
        assert_eq!(name, "qdrant");
    }

    #[test]
    fn test_download_url() {
        let url = get_download_url();
        // Should have a URL for supported platforms
        #[cfg(any(
            all(target_os = "macos", target_arch = "x86_64"),
            all(target_os = "macos", target_arch = "aarch64"),
            all(target_os = "linux", target_arch = "x86_64"),
            all(target_os = "windows", target_arch = "x86_64"),
        ))]
        assert!(url.is_some());
    }
}
