use std::path::PathBuf;

use clap::{Parser, ValueEnum};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, ValueEnum, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
#[clap(rename_all = "lower")]
pub enum Network {
    #[default]
    Esmeralda,
    #[serde(alias = "local-net")]
    LocalNet,
}

impl Network {
    pub fn default_indexer_url(self) -> &'static str {
        match self {
            Network::Esmeralda => "https://ootle-indexer-a.tari.com/",
            Network::LocalNet => "http://localhost:12500",
        }
    }
}

#[derive(Parser, Debug)]
#[command(
    name = "ootle-community-templates",
    about = "Ootle Community Templates Server"
)]
pub struct Cli {
    /// Path to the TOML configuration file
    #[arg(short, long, default_value = "config.toml")]
    pub config: PathBuf,

    /// Create a default config file at the config path if it doesn't exist, then exit
    #[arg(long)]
    pub create_config: bool,

    /// Override the server port
    #[arg(long)]
    pub port: Option<u16>,

    /// Override the database URL
    #[arg(long)]
    pub database_url: Option<String>,

    /// Override the indexer URL
    #[arg(long)]
    pub indexer_url: Option<String>,

    /// Network to use for default indexer URL
    #[arg(long, value_enum)]
    pub network: Option<Network>,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct Config {
    #[serde(default)]
    pub network: Network,
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub indexer: IndexerConfig,
    #[serde(default)]
    pub admin: AdminConfig,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ServerConfig {
    #[serde(default = "default_bind_address")]
    pub bind_address: String,
    #[serde(default = "default_port")]
    pub port: u16,
    /// JWT secret for admin auth. If not set, a random secret is generated on each startup
    /// (all existing JWTs are invalidated on restart).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub jwt_secret: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DatabaseConfig {
    #[serde(default = "default_database_url")]
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct IndexerConfig {
    /// Indexer REST API URL. If not set, uses the default for the configured network.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(default = "default_sync_interval")]
    pub sync_interval_secs: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AdminConfig {
    #[serde(default = "default_admin_username")]
    pub initial_username: String,
    #[serde(default = "default_admin_password")]
    pub initial_password: String,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind_address: default_bind_address(),
            port: default_port(),
            jwt_secret: None,
        }
    }
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: default_database_url(),
        }
    }
}

impl Default for IndexerConfig {
    fn default() -> Self {
        Self {
            url: None,
            sync_interval_secs: default_sync_interval(),
        }
    }
}

impl Default for AdminConfig {
    fn default() -> Self {
        Self {
            initial_username: default_admin_username(),
            initial_password: default_admin_password(),
        }
    }
}

impl Config {
    pub fn load(cli: &Cli) -> Result<Self, ConfigError> {
        let mut config: Config = if cli.config.exists() {
            let contents =
                std::fs::read_to_string(&cli.config).map_err(|e| ConfigError::ReadFile {
                    path: cli.config.clone(),
                    source: e,
                })?;
            toml::from_str(&contents).map_err(ConfigError::Parse)?
        } else {
            tracing::info!(
                "No config file found at {}, using defaults",
                cli.config.display()
            );
            Config::default()
        };

        // Apply CLI overrides
        if let Some(network) = cli.network {
            config.network = network;
        }
        if let Some(port) = cli.port {
            config.server.port = port;
        }
        if let Some(ref url) = cli.database_url {
            config.database.url = url.clone();
        }
        if let Some(ref url) = cli.indexer_url {
            config.indexer.url = Some(url.clone());
        }

        Ok(config)
    }

    /// Returns the effective indexer URL, falling back to the network default.
    pub fn indexer_url(&self) -> &str {
        self.indexer
            .url
            .as_deref()
            .unwrap_or_else(|| self.network.default_indexer_url())
    }

    /// Returns the effective JWT secret as bytes. If none is configured, generates random bytes.
    pub fn jwt_secret(&self) -> Vec<u8> {
        match &self.server.jwt_secret {
            Some(s) => s.as_bytes().to_vec(),
            None => generate_random_secret(),
        }
    }

    /// Write the default config to the given path. Returns an error if the file already exists.
    pub fn write_default(path: &std::path::Path) -> Result<(), ConfigError> {
        if path.exists() {
            return Err(ConfigError::AlreadyExists(path.to_path_buf()));
        }

        let config = Config::default();
        let contents = toml::to_string_pretty(&config).map_err(ConfigError::Serialize)?;

        // Prepend a comment header
        let output = format!(
            "# Ootle Community Templates Server Configuration\n\
             # Generated with --create-config\n\n\
             # Network determines the default indexer URL.\n\
             # Options: \"esmeralda\" (testnet, default), \"localnet\"\n\n\
             {contents}\n\
             # [server]\n\
             # jwt_secret = \"set-for-persistent-sessions\"  # omit to generate random on each startup\n\n\
             # [indexer]\n\
             # url = \"https://custom-indexer.example.com/\"  # omit to use the network default\n"
        );

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| ConfigError::WriteFile {
                path: path.to_path_buf(),
                source: e,
            })?;
        }

        std::fs::write(path, output).map_err(|e| ConfigError::WriteFile {
            path: path.to_path_buf(),
            source: e,
        })?;

        Ok(())
    }
}

fn generate_random_secret() -> Vec<u8> {
    use rand::Rng;
    let mut bytes = vec![0u8; 32];
    rand::rng().fill(&mut bytes[..]);
    bytes
}

fn default_bind_address() -> String {
    "0.0.0.0".to_string()
}

fn default_port() -> u16 {
    3000
}

fn default_database_url() -> String {
    "postgres://ootle:ootle@localhost:5432/community_templates".to_string()
}

fn default_sync_interval() -> u64 {
    60
}

fn default_admin_username() -> String {
    "admin".to_string()
}

fn default_admin_password() -> String {
    "change-me".to_string()
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Failed to read config file {path}: {source}")]
    ReadFile {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("Config file already exists: {0}")]
    AlreadyExists(PathBuf),
    #[error("Failed to write config file {path}: {source}")]
    WriteFile {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("Failed to parse config: {0}")]
    Parse(toml::de::Error),
    #[error("Failed to serialize config: {0}")]
    Serialize(toml::ser::Error),
}
