use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use anyhow::{Result, Context};
use log::{info, debug, warn};
use dirs::config_dir;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub monitoring: MonitoringConfig,
    pub output: OutputConfig,
    pub logging: LoggingConfig,
    pub filters: FilterConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    pub poll_interval_ms: u64,
    pub auto_start: bool,
    pub track_input_events: bool,
    pub detect_suspicious_activity: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputConfig {
    pub default_format: String,
    pub timestamp_format: String,
    pub include_metadata: bool,
    pub color_output: bool,
    pub max_log_entries: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
    pub file_logging: bool,
    pub log_file_path: Option<PathBuf>,
    pub max_log_file_size_mb: u64,
    pub rotate_logs: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterConfig {
    pub ignored_vendors: Vec<u16>,
    pub ignored_products: Vec<u16>,
    pub allowed_device_classes: Option<Vec<u8>>,
    pub name_patterns: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            monitoring: MonitoringConfig {
                poll_interval_ms: 500,
                auto_start: false,
                track_input_events: true,
                detect_suspicious_activity: true,
            },
            output: OutputConfig {
                default_format: "table".to_string(),
                timestamp_format: "%Y-%m-%d %H:%M:%S UTC".to_string(),
                include_metadata: true,
                color_output: true,
                max_log_entries: 1000,
            },
            logging: LoggingConfig {
                level: "info".to_string(),
                file_logging: false,
                log_file_path: None,
                max_log_file_size_mb: 10,
                rotate_logs: true,
            },
            filters: FilterConfig {
                ignored_vendors: vec![],
                ignored_products: vec![],
                allowed_device_classes: None,
                name_patterns: vec![],
            },
        }
    }
}

pub struct ConfigManager {
    config: Config,
    config_path: PathBuf,
}

impl ConfigManager {
    /// Create a new configuration manager
    pub fn new(custom_path: Option<PathBuf>) -> Result<Self> {
        let config_path = match custom_path {
            Some(path) => path,
            None => Self::get_default_config_path()?,
        };

        let config = if config_path.exists() {
            Self::load_from_file(&config_path)?
        } else {
            info!("No configuration file found, using defaults");
            Config::default()
        };

        Ok(Self {
            config,
            config_path,
        })
    }

    /// Get the default configuration file path
    fn get_default_config_path() -> Result<PathBuf> {
        let config_dir = config_dir()
            .context("Could not determine config directory")?;
        
        let app_config_dir = config_dir.join("ironwatch");
        if !app_config_dir.exists() {
            fs::create_dir_all(&app_config_dir)
                .context("Failed to create config directory")?;
        }

        Ok(app_config_dir.join("config.json"))
    }

    /// Load configuration from file
    fn load_from_file(path: &Path) -> Result<Config> {
        debug!("Loading configuration from: {}", path.display());
        
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;
        
        let config: Config = serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse config file: {}", path.display()))?;
        
        info!("Configuration loaded successfully");
        Ok(config)
    }

    /// Save current configuration to file
    pub fn save(&self) -> Result<()> {
        debug!("Saving configuration to: {}", self.config_path.display());
        
        // Ensure parent directory exists
        if let Some(parent) = self.config_path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)
                    .context("Failed to create config directory")?;
            }
        }

        let content = serde_json::to_string_pretty(&self.config)
            .context("Failed to serialize configuration")?;
        
        fs::write(&self.config_path, content)
            .with_context(|| format!("Failed to write config file: {}", self.config_path.display()))?;
        
        info!("Configuration saved successfully");
        Ok(())
    }

    /// Get the current configuration
    pub fn get_config(&self) -> &Config {
        &self.config
    }

    /// Get mutable reference to configuration
    pub fn get_config_mut(&mut self) -> &mut Config {
        &mut self.config
    }

    /// Update a configuration value by key path
    pub fn set_value(&mut self, key_path: &str, value: &str) -> Result<()> {
        debug!("Setting config value: {} = {}", key_path, value);
        
        match key_path {
            "monitoring.poll_interval_ms" => {
                self.config.monitoring.poll_interval_ms = value.parse()
                    .context("Invalid poll_interval_ms value")?;
            }
            "monitoring.auto_start" => {
                self.config.monitoring.auto_start = value.parse()
                    .context("Invalid auto_start value")?;
            }
            "monitoring.track_input_events" => {
                self.config.monitoring.track_input_events = value.parse()
                    .context("Invalid track_input_events value")?;
            }
            "monitoring.detect_suspicious_activity" => {
                self.config.monitoring.detect_suspicious_activity = value.parse()
                    .context("Invalid detect_suspicious_activity value")?;
            }
            "output.default_format" => {
                if !["json", "table", "csv"].contains(&value) {
                    anyhow::bail!("Invalid output format. Must be: json, table, or csv");
                }
                self.config.output.default_format = value.to_string();
            }
            "output.color_output" => {
                self.config.output.color_output = value.parse()
                    .context("Invalid color_output value")?;
            }
            "output.include_metadata" => {
                self.config.output.include_metadata = value.parse()
                    .context("Invalid include_metadata value")?;
            }
            "logging.level" => {
                if !["error", "warn", "info", "debug", "trace"].contains(&value) {
                    anyhow::bail!("Invalid log level. Must be: error, warn, info, debug, or trace");
                }
                self.config.logging.level = value.to_string();
            }
            "logging.file_logging" => {
                self.config.logging.file_logging = value.parse()
                    .context("Invalid file_logging value")?;
            }
            _ => {
                anyhow::bail!("Unknown configuration key: {}", key_path);
            }
        }
        
        info!("Configuration updated: {} = {}", key_path, value);
        Ok(())
    }

    /// Get a configuration value by key path
    pub fn get_value(&self, key_path: &str) -> Result<String> {
        let value = match key_path {
            "monitoring.poll_interval_ms" => self.config.monitoring.poll_interval_ms.to_string(),
            "monitoring.auto_start" => self.config.monitoring.auto_start.to_string(),
            "monitoring.track_input_events" => self.config.monitoring.track_input_events.to_string(),
            "monitoring.detect_suspicious_activity" => self.config.monitoring.detect_suspicious_activity.to_string(),
            "output.default_format" => self.config.output.default_format.clone(),
            "output.color_output" => self.config.output.color_output.to_string(),
            "output.include_metadata" => self.config.output.include_metadata.to_string(),
            "logging.level" => self.config.logging.level.clone(),
            "logging.file_logging" => self.config.logging.file_logging.to_string(),
            _ => anyhow::bail!("Unknown configuration key: {}", key_path),
        };
        
        Ok(value)
    }

    /// Reset configuration to defaults
    pub fn reset_to_defaults(&mut self) {
        warn!("Resetting configuration to defaults");
        self.config = Config::default();
    }

    /// Validate current configuration
    pub fn validate(&self) -> Result<()> {
        // Validate monitoring settings
        if self.config.monitoring.poll_interval_ms < 100 {
            anyhow::bail!("Poll interval must be at least 100ms");
        }

        // Validate output format
        if !["json", "table", "csv"].contains(&self.config.output.default_format.as_str()) {
            anyhow::bail!("Invalid default output format");
        }

        // Validate log level
        if !["error", "warn", "info", "debug", "trace"].contains(&self.config.logging.level.as_str()) {
            anyhow::bail!("Invalid log level");
        }

        // Validate log file size
        if self.config.logging.max_log_file_size_mb > 100 {
            warn!("Large log file size configured: {}MB", self.config.logging.max_log_file_size_mb);
        }

        debug!("Configuration validation passed");
        Ok(())
    }

    /// Get the configuration file path
    pub fn get_config_path(&self) -> &PathBuf {
        &self.config_path
    }
}