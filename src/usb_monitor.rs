use rusb::{Context, Device, DeviceDescriptor, DeviceHandle, UsbContext};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use anyhow::{Result, Context as AnyhowContext};
use log::{debug, info, error, warn};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsbDeviceInfo {
    pub bus_number: u8,
    pub device_address: u8,
    pub vendor_id: u16,
    pub product_id: u16,
    pub device_version: u16,
    pub manufacturer: Option<String>,
    pub product: Option<String>,
    pub serial_number: Option<String>,
    pub device_class: u8,
    pub device_subclass: u8,
    pub device_protocol: u8,
    pub max_packet_size: u8,
    pub num_configurations: u8,
    pub timestamp: DateTime<Utc>,
    pub connection_status: ConnectionStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ConnectionStatus {
    Connected,
    Disconnected,
    Reconnected,
    Blocked,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceStatistics {
    pub total_connections: u32,
    pub total_disconnections: u32,
    pub total_blocked: u32,
    pub first_seen: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
    pub connection_duration: Duration,
    pub connection_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceAnalytics {
    pub device_class_distribution: HashMap<u8, u32>,
    pub vendor_distribution: HashMap<u16, u32>,
    pub connection_frequency: Vec<(DateTime<Utc>, u32)>,
    pub total_devices_seen: u32,
    pub unique_devices: u32,
    pub blocked_devices: u32,
    pub security_violations: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityEvent {
    pub timestamp: DateTime<Utc>,
    pub event_type: SecurityEventType,
    pub device_info: UsbDeviceInfo,
    pub reason: String,
    pub action_taken: SecurityAction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecurityEventType {
    DeviceBlocked,
    DeviceAllowed,
    RuleViolation,
    SuspiciousActivity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecurityAction {
    Blocked,
    Allowed,
    Warned,
    Logged,
}

pub struct UsbMonitor {
    context: Context,
    previous_devices: HashMap<String, UsbDeviceInfo>,
    device_filter: Option<String>,
    device_statistics: HashMap<String, DeviceStatistics>,
    connection_history: Vec<(DateTime<Utc>, String, ConnectionStatus)>,
    security_events: Vec<SecurityEvent>,
    config_manager: Option<std::sync::Arc<tokio::sync::RwLock<crate::config::ConfigManager>>>,
}

impl UsbMonitor {
    /// Create a new USB monitor instance
    pub fn new() -> Result<Self> {
        let context = Context::new()
            .context("Failed to create USB context")?;
        
        Ok(Self {
            context,
            previous_devices: HashMap::new(),
            device_filter: None,
            device_statistics: HashMap::new(),
            connection_history: Vec::new(),
            security_events: Vec::new(),
            config_manager: None,
        })
    }

    /// Set the configuration manager for device rules
    pub fn set_config_manager(&mut self, config_manager: std::sync::Arc<tokio::sync::RwLock<crate::config::ConfigManager>>) {
        self.config_manager = Some(config_manager);
        info!("Configuration manager set for device rules");
    }

    /// Set a device filter pattern
    pub fn set_filter(&mut self, filter: Option<String>) {
        self.device_filter = filter;
    }

    /// Get device statistics for a specific device
    pub fn get_device_statistics(&self, device_key: &str) -> Option<&DeviceStatistics> {
        self.device_statistics.get(device_key)
    }

    /// Get security events
    pub fn get_security_events(&self) -> &[SecurityEvent] {
        &self.security_events
    }

    /// Get overall device analytics
    pub fn get_device_analytics(&self) -> DeviceAnalytics {
        let mut class_distribution = HashMap::new();
        let mut vendor_distribution = HashMap::new();
        let mut unique_devices = std::collections::HashSet::new();
        let mut blocked_count = 0;
        let mut security_violations = 0;
        
        // Analyze all devices we've seen
        for (key, stats) in &self.device_statistics {
            unique_devices.insert(key.clone());
            blocked_count += stats.total_blocked;
            
            // Get device info from connection history
            if let Some(device_info) = self.get_device_info_from_key(key) {
                *class_distribution.entry(device_info.device_class).or_insert(0) += 1;
                *vendor_distribution.entry(device_info.vendor_id).or_insert(0) += 1;
            }
        }
        
        // Count security violations
        security_violations = self.security_events.len() as u32;
        
        // Generate connection frequency data (last 24 hours, hourly buckets)
        let mut connection_frequency = Vec::new();
        let now = Utc::now();
        let one_day_ago = now - chrono::Duration::hours(24);
        
        for hour in 0..24 {
            let hour_start = one_day_ago + chrono::Duration::hours(hour);
            let hour_end = hour_start + chrono::Duration::hours(1);
            
            let connections_in_hour = self.connection_history
                .iter()
                .filter(|(timestamp, _, status)| {
                    *timestamp >= hour_start && *timestamp < hour_end && 
                    matches!(status, ConnectionStatus::Connected)
                })
                .count();
            
            connection_frequency.push((hour_start, connections_in_hour as u32));
        }
        
        DeviceAnalytics {
            device_class_distribution: class_distribution,
            vendor_distribution: vendor_distribution,
            connection_frequency,
            total_devices_seen: self.connection_history.len() as u32,
            unique_devices: unique_devices.len() as u32,
            blocked_devices: blocked_count,
            security_violations,
        }
    }

    /// Get connection history for a specific device
    pub fn get_device_connection_history(&self, device_key: &str) -> Vec<(DateTime<Utc>, ConnectionStatus)> {
        self.connection_history
            .iter()
            .filter(|(_, key, _)| key == device_key)
            .map(|(timestamp, _, status)| (*timestamp, status.clone()))
            .collect()
    }

    /// Check if a device should be blocked based on current rules
    async fn check_device_security(&mut self, device: &UsbDeviceInfo) -> (bool, Option<String>, SecurityAction) {
        if let Some(config_manager) = &self.config_manager {
            let config = config_manager.read().await;
            let (should_block, reason) = config.should_block_device(device);
            
            if should_block {
                let action = SecurityAction::Blocked;
                let event = SecurityEvent {
                    timestamp: Utc::now(),
                    event_type: SecurityEventType::DeviceBlocked,
                    device_info: device.clone(),
                    reason: reason.clone().unwrap_or_else(|| "Unknown reason".to_string()),
                    action_taken: action.clone(),
                };
                
                self.security_events.push(event);
                
                // Keep only last 1000 security events
                if self.security_events.len() > 1000 {
                    self.security_events.remove(0);
                }
                
                return (true, reason, action);
            } else {
                let action = SecurityAction::Allowed;
                let event = SecurityEvent {
                    timestamp: Utc::now(),
                    event_type: SecurityEventType::DeviceAllowed,
                    device_info: device.clone(),
                    reason: "Device passed security checks".to_string(),
                    action_taken: action.clone(),
                };
                
                self.security_events.push(event);
                
                // Keep only last 1000 security events
                if self.security_events.len() > 1000 {
                    self.security_events.remove(0);
                }
                
                return (false, None, action);
            }
        }
        
        (false, None, SecurityAction::Allowed)
    }

    /// Get all currently connected USB devices
    pub fn get_connected_devices(&self) -> Result<Vec<UsbDeviceInfo>> {
        let devices = self.context.devices()
            .context("Failed to get device list")?;
        
        let mut device_info_list = Vec::new();
        
        for device in devices.iter() {
            match self.get_device_info(&device) {
                Ok(mut info) => {
                    // Check device security before adding to list (synchronous for now)
                    // TODO: Implement async security checking in a separate method
                    
                    // Apply filter if set
                    if let Some(ref filter) = self.device_filter {
                        if let Some(ref product) = info.product {
                            if !product.to_lowercase().contains(&filter.to_lowercase()) {
                                continue;
                            }
                        } else if let Some(ref manufacturer) = info.manufacturer {
                            if !manufacturer.to_lowercase().contains(&manufacturer.to_lowercase()) {
                                continue;
                            }
                        } else {
                            continue;
                        }
                    }
                    
                    device_info_list.push(info);
                }
                Err(e) => {
                    debug!("Failed to get device info: {}", e);
                }
            }
        }
        
        Ok(device_info_list)
    }
    
    /// Check device security asynchronously (separate from device enumeration)
    pub async fn check_device_security_async(&self, device: &UsbDeviceInfo) -> (bool, Option<String>, SecurityAction) {
        if let Some(config_manager) = &self.config_manager {
            let config = config_manager.read().await;
            let (should_block, reason) = config.should_block_device(device);
            
            if should_block {
                let action = SecurityAction::Blocked;
                return (true, reason, action);
            } else {
                let action = SecurityAction::Allowed;
                return (false, None, action);
            }
        }
        
        (false, None, SecurityAction::Allowed)
    }

    /// Monitor for device changes (connect/disconnect events)
    pub async fn monitor_changes(&mut self) -> Result<Vec<UsbDeviceChange>> {
        let current_devices = self.get_connected_devices()?;
        let mut changes = Vec::new();
        
        // Create a map of current devices by their unique identifier
        let mut current_device_map = HashMap::new();
        for device in &current_devices {
            let device_key = self.create_device_key(device);
            current_device_map.insert(device_key, device.clone());
        }
        
        // Check for disconnected devices
        let mut disconnected_keys = Vec::new();
        for (key, prev_device) in &self.previous_devices {
            if !current_device_map.contains_key(key) {
                let mut disconnected_device = prev_device.clone();
                disconnected_device.connection_status = ConnectionStatus::Disconnected;
                disconnected_device.timestamp = Utc::now();
                
                disconnected_keys.push((key.clone(), disconnected_device.clone()));
                changes.push(UsbDeviceChange::Disconnected(disconnected_device));
            }
        }
        
        // Check for new/reconnected devices
        let mut new_devices = Vec::new();
        let mut reconnected_devices = Vec::new();
        
        for (key, current_device) in &current_device_map {
            match self.previous_devices.get(key) {
                None => {
                    // New device - check security
                    let (is_blocked, reason, action) = self.check_device_security(current_device).await;
                    
                    let mut new_device = current_device.clone();
                    if is_blocked {
                        new_device.connection_status = ConnectionStatus::Blocked;
                        warn!("New device blocked: {} (VID:{:04X}, PID:{:04X}) - {}", 
                              new_device.product.as_deref().unwrap_or("Unknown"),
                              new_device.vendor_id, new_device.product_id, 
                              reason.unwrap_or_else(|| "Unknown reason".to_string()));
                    } else {
                        new_device.connection_status = ConnectionStatus::Connected;
                    }
                    
                    new_devices.push((key.clone(), new_device.clone()));
                    
                    if is_blocked {
                        changes.push(UsbDeviceChange::Blocked(new_device));
                    } else {
                        changes.push(UsbDeviceChange::Connected(new_device));
                    }
                }
                Some(prev_device) => {
                    // Check if device was previously disconnected
                    if matches!(prev_device.connection_status, ConnectionStatus::Disconnected) {
                        let mut reconnected_device = current_device.clone();
                        reconnected_device.connection_status = ConnectionStatus::Reconnected;
                        
                        reconnected_devices.push((key.clone(), reconnected_device.clone()));
                        changes.push(UsbDeviceChange::Reconnected(reconnected_device));
                    }
                }
            }
        }
        
        // Update statistics after collecting all changes
        for (key, device) in disconnected_keys {
            self.update_device_statistics(&key, &device, ConnectionStatus::Disconnected);
        }
        
        for (key, device) in new_devices {
            let status = if device.connection_status == ConnectionStatus::Blocked {
                ConnectionStatus::Blocked
            } else {
                ConnectionStatus::Connected
            };
            self.update_device_statistics(&key, &device, status);
        }
        
        for (key, device) in reconnected_devices {
            self.update_device_statistics(&key, &device, ConnectionStatus::Reconnected);
        }
        
        // Update previous devices state
        self.previous_devices = current_device_map;
        
        Ok(changes)
    }

    /// Update device statistics when a change occurs
    fn update_device_statistics(&mut self, device_key: &str, device: &UsbDeviceInfo, status: ConnectionStatus) {
        let now = Utc::now();
        
        // Record connection history
        self.connection_history.push((now, device_key.to_string(), status.clone()));
        
        // Keep only last 1000 entries to prevent memory bloat
        if self.connection_history.len() > 1000 {
            self.connection_history.remove(0);
        }
        
        // Update device statistics
        let stats = self.device_statistics.entry(device_key.to_string()).or_insert(DeviceStatistics {
            total_connections: 0,
            total_disconnections: 0,
            total_blocked: 0,
            first_seen: now,
            last_seen: now,
            connection_duration: Duration::ZERO,
            connection_count: 0,
        });
        
        stats.last_seen = now;
        
        match status {
            ConnectionStatus::Connected => {
                stats.total_connections += 1;
                stats.connection_count += 1;
            }
            ConnectionStatus::Disconnected => {
                stats.total_disconnections += 1;
                if stats.connection_count > 0 {
                    stats.connection_count -= 1;
                }
            }
            ConnectionStatus::Reconnected => {
                stats.total_connections += 1;
                stats.connection_count += 1;
            }
            ConnectionStatus::Blocked => {
                stats.total_blocked += 1;
                // Don't increment connection count for blocked devices
            }
        }
        
        // Calculate total connection duration
        if let Some(first_connection) = self.connection_history
            .iter()
            .find(|(_, key, status)| key == device_key && matches!(status, ConnectionStatus::Connected))
        {
            stats.connection_duration = now.signed_duration_since(first_connection.0)
                .to_std()
                .unwrap_or(Duration::ZERO);
        }
    }

    /// Start continuous monitoring
    pub async fn start_monitoring<F>(&mut self, mut callback: F) -> Result<()>
    where
        F: FnMut(Vec<UsbDeviceChange>) -> Result<()>,
    {
        info!("Starting USB device monitoring with security enforcement...");
        
        // Initial device scan
        let initial_devices = self.get_connected_devices()?;
        info!("Found {} initial USB devices", initial_devices.len());
        
        // Populate initial state
        for device in initial_devices {
            let key = self.create_device_key(&device);
            self.previous_devices.insert(key.clone(), device.clone());
            
            // Initialize statistics for initial devices
            let status = if device.connection_status == ConnectionStatus::Blocked {
                ConnectionStatus::Blocked
            } else {
                ConnectionStatus::Connected
            };
            self.update_device_statistics(&key, &device, status);
        }
        
        loop {
            match self.monitor_changes().await {
                Ok(changes) => {
                    if !changes.is_empty() {
                        debug!("Detected {} USB device changes", changes.len());
                        if let Err(e) = callback(changes) {
                            error!("Callback error: {}", e);
                        }
                    }
                }
                Err(e) => {
                    error!("Monitoring error: {}", e);
                }
            }
            
            // Poll interval
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    }

    /// Get detailed information about a USB device
    fn get_device_info(&self, device: &Device<Context>) -> Result<UsbDeviceInfo> {
        let descriptor = device.device_descriptor()
            .context("Failed to get device descriptor")?;
        
        let bus_number = device.bus_number();
        let device_address = device.address();
        
        // Try to open device to get string descriptors
        let (manufacturer, product, serial_number) = match device.open() {
            Ok(handle) => self.get_string_descriptors(&handle, &descriptor),
            Err(_) => {
                debug!("Could not open device {}:{} for string descriptors", 
                       bus_number, device_address);
                (None, None, None)
            }
        };
        
        Ok(UsbDeviceInfo {
            bus_number,
            device_address,
            vendor_id: descriptor.vendor_id(),
            product_id: descriptor.product_id(),
            device_version: {
                let version = descriptor.device_version();
                (version.major() as u16) << 8 | (version.minor() as u16)
            },
            manufacturer,
            product,
            serial_number,
            device_class: descriptor.class_code(),
            device_subclass: descriptor.sub_class_code(),
            device_protocol: descriptor.protocol_code(),
            max_packet_size: descriptor.max_packet_size(),
            num_configurations: descriptor.num_configurations(),
            timestamp: Utc::now(),
            connection_status: ConnectionStatus::Connected,
        })
    }

    /// Extract string descriptors from device
    fn get_string_descriptors(
        &self,
        handle: &DeviceHandle<Context>,
        descriptor: &DeviceDescriptor,
    ) -> (Option<String>, Option<String>, Option<String>) {
        let manufacturer = if let Some(index) = descriptor.manufacturer_string_index() {
            if index > 0 {
                handle.read_manufacturer_string_ascii(descriptor).ok()
            } else {
                None
            }
        } else {
            None
        };
        
        let product = if let Some(index) = descriptor.product_string_index() {
            if index > 0 {
                handle.read_product_string_ascii(descriptor).ok()
            } else {
                None
            }
        } else {
            None
        };
        
        let serial_number = if let Some(index) = descriptor.serial_number_string_index() {
            if index > 0 {
                handle.read_serial_number_string_ascii(descriptor).ok()
            } else {
                None
            }
        } else {
            None
        };
        
        (manufacturer, product, serial_number)
    }

    /// Create a unique key for device identification
    fn create_device_key(&self, device: &UsbDeviceInfo) -> String {
        format!("{}:{}:{}:{}", 
                device.vendor_id, 
                device.product_id, 
                device.bus_number, 
                device.device_address)
    }

    /// Helper method to get device info from a device key
    fn get_device_info_from_key(&self, device_key: &str) -> Option<&UsbDeviceInfo> {
        // Try to find in previous devices first
        if let Some(device) = self.previous_devices.get(device_key) {
            return Some(device);
        }
        
        // If not found, try to find in current devices
        for device in self.previous_devices.values() {
            if self.create_device_key(device) == device_key {
                return Some(device);
            }
        }
        
        None
    }
}

#[derive(Debug, Clone, Serialize)]
pub enum UsbDeviceChange {
    Connected(UsbDeviceInfo),
    Disconnected(UsbDeviceInfo),
    Reconnected(UsbDeviceInfo),
    Blocked(UsbDeviceInfo),
}

impl UsbDeviceChange {
    pub fn get_device_info(&self) -> &UsbDeviceInfo {
        match self {
            UsbDeviceChange::Connected(info) => info,
            UsbDeviceChange::Disconnected(info) => info,
            UsbDeviceChange::Reconnected(info) => info,
            UsbDeviceChange::Blocked(info) => info,
        }
    }
    
    pub fn get_change_type(&self) -> &str {
        match self {
            UsbDeviceChange::Connected(_) => "CONNECTED",
            UsbDeviceChange::Disconnected(_) => "DISCONNECTED",
            UsbDeviceChange::Reconnected(_) => "RECONNECTED",
            UsbDeviceChange::Blocked(_) => "BLOCKED",
        }
    }
}