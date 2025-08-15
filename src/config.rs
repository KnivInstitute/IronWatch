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
    pub device_rules: DeviceRulesConfig,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceRulesConfig {
    pub blacklist_enabled: bool,
    pub whitelist_enabled: bool,
    pub blacklisted_devices: Vec<DeviceRule>,
    pub whitelisted_devices: Vec<DeviceRule>,
    pub auto_block_suspicious: bool,
    pub block_threshold: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DeviceRule {
    pub vendor_id: Option<u16>,
    pub product_id: Option<u16>,
    pub device_class: Option<u8>,
    pub manufacturer: Option<String>,
    pub product_name: Option<String>,
    pub serial_number: Option<String>,
    pub reason: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub enabled: bool,
}

impl DeviceRule {
    pub fn new() -> Self {
        Self {
            vendor_id: None,
            product_id: None,
            device_class: None,
            manufacturer: None,
            product_name: None,
            serial_number: None,
            reason: String::new(),
            created_at: chrono::Utc::now(),
            enabled: true,
        }
    }
    
    pub fn matches_device(&self, device: &crate::usb_monitor::UsbDeviceInfo) -> bool {
        // Check vendor ID
        if let Some(vid) = self.vendor_id {
            if device.vendor_id != vid {
                return false;
            }
        }
        
        // Check product ID
        if let Some(pid) = self.product_id {
            if device.product_id != pid {
                return false;
            }
        }
        
        // Check device class
        if let Some(class) = self.device_class {
            if device.device_class != class {
                return false;
            }
        }
        
        // Check manufacturer
        if let Some(ref manufacturer) = self.manufacturer {
            if let Some(ref device_manufacturer) = device.manufacturer {
                if !device_manufacturer.to_lowercase().contains(&manufacturer.to_lowercase()) {
                    return false;
                }
            } else {
                return false;
            }
        }
        
        // Check product name
        if let Some(ref product) = self.product_name {
            if let Some(ref device_product) = device.product {
                if !device_product.to_lowercase().contains(&product.to_lowercase()) {
                    return false;
                }
            } else {
                return false;
            }
        }
        
        // Check serial number
        if let Some(ref serial) = self.serial_number {
            if let Some(ref device_serial) = device.serial_number {
                if !device_serial.to_lowercase().contains(&serial.to_lowercase()) {
                    return false;
                }
            } else {
                return false;
            }
        }
        
        true
    }
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
            device_rules: DeviceRulesConfig {
                blacklist_enabled: true,
                whitelist_enabled: false,
                blacklisted_devices: vec![],
                whitelisted_devices: vec![],
                auto_block_suspicious: false,
                block_threshold: 5,
            },
        }
    }
}

#[derive(Debug)]
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
    
    // Device Rules Management
    
    /// Add a device to the blacklist
    pub fn add_blacklisted_device(&mut self, rule: DeviceRule) -> Result<()> {
        if !self.config.device_rules.blacklist_enabled {
            anyhow::bail!("Blacklist is not enabled");
        }
        
        // Check if device is already blacklisted
        if self.config.device_rules.blacklisted_devices.iter().any(|r| r == &rule) {
            anyhow::bail!("Device is already blacklisted");
        }
        
        self.config.device_rules.blacklisted_devices.push(rule);
        info!("Device added to blacklist");
        Ok(())
    }
    
    /// Remove a device from the blacklist
    pub fn remove_blacklisted_device(&mut self, index: usize) -> Result<()> {
        if index >= self.config.device_rules.blacklisted_devices.len() {
            anyhow::bail!("Invalid blacklist index");
        }
        
        let removed = self.config.device_rules.blacklisted_devices.remove(index);
        info!("Device removed from blacklist: {:?}", removed);
        Ok(())
    }
    
    /// Add a device to the whitelist
    pub fn add_whitelisted_device(&mut self, rule: DeviceRule) -> Result<()> {
        if !self.config.device_rules.whitelist_enabled {
            anyhow::bail!("Whitelist is not enabled");
        }
        
        // Check if device is already whitelisted
        if self.config.device_rules.whitelisted_devices.iter().any(|r| r == &rule) {
            anyhow::bail!("Device is already whitelisted");
        }
        
        self.config.device_rules.whitelisted_devices.push(rule);
        info!("Device added to whitelist");
        Ok(())
    }
    
    /// Remove a device from the whitelist
    pub fn remove_whitelisted_device(&mut self, index: usize) -> Result<()> {
        if index >= self.config.device_rules.whitelisted_devices.len() {
            anyhow::bail!("Invalid whitelist index");
        }
        
        let removed = self.config.device_rules.whitelisted_devices.remove(index);
        info!("Device removed from whitelist: {:?}", removed);
        Ok(())
    }
    
    /// Check if a device should be blocked based on current rules
    pub fn should_block_device(&self, device: &crate::usb_monitor::UsbDeviceInfo) -> (bool, Option<String>) {
        // If whitelist is enabled, only allow whitelisted devices
        if self.config.device_rules.whitelist_enabled {
            let is_whitelisted = self.config.device_rules.whitelisted_devices.iter()
                .any(|rule| rule.enabled && rule.matches_device(device));
            
            if !is_whitelisted {
                return (true, Some("Device not in whitelist".to_string()));
            }
        }
        
        // Check blacklist
        if self.config.device_rules.blacklist_enabled {
            for rule in &self.config.device_rules.blacklisted_devices {
                if rule.enabled && rule.matches_device(device) {
                    return (true, Some(rule.reason.clone()));
                }
            }
        }
        
        (false, None)
    }
    
    /// Enable or disable blacklist
    pub fn set_blacklist_enabled(&mut self, enabled: bool) {
        self.config.device_rules.blacklist_enabled = enabled;
        info!("Blacklist {}", if enabled { "enabled" } else { "disabled" });
    }
    
    /// Enable or disable whitelist
    pub fn set_whitelist_enabled(&mut self, enabled: bool) {
        self.config.device_rules.whitelist_enabled = enabled;
        info!("Whitelist {}", if enabled { "enabled" } else { "disabled" });
    }
    
    /// Get all blacklisted devices
    pub fn get_blacklisted_devices(&self) -> &[DeviceRule] {
        &self.config.device_rules.blacklisted_devices
    }
    
    /// Get all whitelisted devices
    pub fn get_whitelisted_devices(&self) -> &[DeviceRule] {
        &self.config.device_rules.whitelisted_devices
    }
}