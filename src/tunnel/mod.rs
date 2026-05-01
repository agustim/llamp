use anyhow::Result;
use std::env;
use std::process::{Command, Stdio};

pub struct CloudflareTunnel {
    pub hostname: Option<String>,
    pub url: String,
    pub token: Option<String>,
    process: Option<std::process::Child>,
}

impl CloudflareTunnel {
    pub fn new(url: &str) -> Self {
        CloudflareTunnel {
            hostname: None,
            url: url.to_string(),
            token: None,
            process: None,
        }
    }

    pub fn with_hostname(mut self, hostname: &str) -> Self {
        self.hostname = Some(hostname.to_string());
        self
    }

    pub fn with_token(mut self, token: &str) -> Self {
        self.token = Some(token.to_string());
        self
    }

    /// Check if cloudflared is installed and get its version
    pub fn check_installed() -> Result<String> {
        let output = Command::new("cloudflared").arg("--version").output()?;

        if !output.status.success() {
            anyhow::bail!("cloudflared is not installed or not in PATH");
        }

        let version_str = String::from_utf8_lossy(&output.stdout);
        Ok(version_str.trim().to_string())
    }

    /// Detect the current system architecture
    pub fn detect_arch() -> String {
        env::consts::ARCH.to_string()
    }

    /// Check if cloudflared supports the current architecture
    pub fn supports_current_arch() -> bool {
        let arch = Self::detect_arch();
        matches!(
            arch.as_str(),
            "x86_64" | "aarch64" | "arm64" | "amd64" | "arm"
        )
    }

    pub fn start(&mut self) -> Result<()> {
        // Check if cloudflared is installed before starting
        match Self::check_installed() {
            Ok(version) => {
                tracing::info!("cloudflared version: {}", version);
                // Check if architecture is supported
                if !Self::supports_current_arch() {
                    tracing::warn!(
                        "Architecture {} may not be fully supported by cloudflared",
                        Self::detect_arch()
                    );
                }
            }
            Err(e) => {
                tracing::warn!("cloudflared check failed: {}", e);
                tracing::info!("Attempting to start cloudflared anyway...");
            }
        }

        let mut cmd = Command::new("cloudflared");
        cmd.arg("tunnel")
            .arg("--url")
            .arg(&self.url)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        if let Some(hostname) = &self.hostname {
            cmd.arg("--hostname").arg(hostname);
        }

        if let Some(token) = &self.token {
            cmd.arg("--token").arg(token);
        }

        tracing::info!("Starting Cloudflare tunnel with URL: {}", self.url);
        if let Some(ref hostname) = self.hostname {
            tracing::info!("Hostname: {}", hostname);
        }
        if let Some(ref token) = self.token {
            tracing::info!("Token: {}...", &token[..8]);
        }
        tracing::info!("System architecture: {}", Self::detect_arch());

        self.process = Some(cmd.spawn()?);
        Ok(())
    }

    pub fn stop(&mut self) -> Result<()> {
        if let Some(mut child) = self.process.take() {
            tracing::info!("Stopping Cloudflare tunnel...");
            child.kill()?;
            child.wait()?;
            tracing::info!("Cloudflare tunnel stopped");
        }
        Ok(())
    }

    pub fn is_running(&self) -> bool {
        self.process.is_some()
    }
}

impl Drop for CloudflareTunnel {
    fn drop(&mut self) {
        if let Err(e) = self.stop() {
            tracing::warn!("Error stopping tunnel: {}", e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tunnel_creation() {
        let tunnel = CloudflareTunnel::new("http://localhost:8080");
        assert_eq!(tunnel.url, "http://localhost:8080");
        assert!(!tunnel.is_running());
    }

    #[test]
    fn test_tunnel_with_hostname() {
        let tunnel =
            CloudflareTunnel::new("http://localhost:8080").with_hostname("exemple.example.com");
        assert_eq!(tunnel.hostname, Some("exemple.example.com".to_string()));
    }
}
