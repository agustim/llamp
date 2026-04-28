#[cfg(test)]
mod tests {
    use llamp::tunnel::CloudflareTunnel;

    #[test]
    fn test_tunnel_creation() {
        let tunnel = CloudflareTunnel::new("http://localhost:8080");
        assert_eq!(tunnel.url, "http://localhost:8080");
        assert!(!tunnel.is_running());
        assert!(tunnel.hostname.is_none());
    }

    #[test]
    fn test_tunnel_with_hostname() {
        let tunnel = CloudflareTunnel::new("http://localhost:8080")
            .with_hostname("exemple.example.com");
        assert_eq!(tunnel.hostname, Some("exemple.example.com".to_string()));
        assert_eq!(tunnel.url, "http://localhost:8080");
    }

    #[test]
    fn test_tunnel_multiple_options() {
        let tunnel = CloudflareTunnel::new("http://localhost:3000")
            .with_hostname("api.example.com");
        
        assert_eq!(tunnel.url, "http://localhost:3000");
        assert_eq!(tunnel.hostname, Some("api.example.com".to_string()));
    }

    #[test]
    fn test_tunnel_detect_arch() {
        let arch = CloudflareTunnel::detect_arch();
        // Should detect one of the supported architectures
        assert!(
            arch == "x86_64" || arch == "aarch64" || arch == "arm64" || 
            arch == "amd64" || arch == "arm" || arch == "riscv64" || arch == "s390x"
        );
    }

    #[test]
    fn test_tunnel_supports_current_arch() {
        assert!(CloudflareTunnel::supports_current_arch());
    }
}
