use anyhow::{Context, Result};
use std::{net::SocketAddr, path::PathBuf};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServerConfig {
    pub data_dir: PathBuf,
    pub port: u16,
    pub bind_addr: SocketAddr,
}

impl ServerConfig {
    pub fn from_env() -> Result<Self> {
        Self::from_values(std::env::var("DATA_DIR").ok(), std::env::var("PORT").ok())
    }

    fn from_values(data_dir: Option<String>, port: Option<String>) -> Result<Self> {
        let data_dir = data_dir
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."));
        let port = port
            .unwrap_or_else(|| "8080".to_string())
            .parse::<u16>()
            .context("PORT must be a valid TCP port")?;
        let bind_addr = SocketAddr::from(([0, 0, 0, 0], port));

        Ok(Self {
            data_dir,
            port,
            bind_addr,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_to_current_directory_and_port_8080() {
        let config = ServerConfig::from_values(None, None).unwrap();

        assert_eq!(config.data_dir, PathBuf::from("."));
        assert_eq!(config.port, 8080);
        assert_eq!(config.bind_addr, SocketAddr::from(([0, 0, 0, 0], 8080)));
    }

    #[test]
    fn reads_custom_data_dir_and_port() {
        let config =
            ServerConfig::from_values(Some("fixtures".to_string()), Some("9090".to_string()))
                .unwrap();

        assert_eq!(config.data_dir, PathBuf::from("fixtures"));
        assert_eq!(config.port, 9090);
        assert_eq!(config.bind_addr, SocketAddr::from(([0, 0, 0, 0], 9090)));
    }

    #[test]
    fn rejects_invalid_port() {
        let err = ServerConfig::from_values(None, Some("not-a-port".to_string())).unwrap_err();

        assert!(err.to_string().contains("PORT must be a valid TCP port"));
    }
}
