use rusb::{Context, Device, DeviceDescriptor, DeviceHandle, UsbContext};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use anyhow::{Result, Context as AnyhowContext};
use log::{debug, info, error};
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConnectionStatus {
    Connected,
    Disconnected,
    Reconnected,
}

#[derive(Debug)]
pub struct UsbMonitor {
    context: Context,
    previous_devices: HashMap<String, UsbDeviceInfo>,
    device_filter: Option<String>,
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
        })
    }

    /// Set a device filter pattern
    pub fn set_filter(&mut self, filter: Option<String>) {
        self.device_filter = filter;
    }

    /// Get all currently connected USB devices
    pub fn get_connected_devices(&self) -> Result<Vec<UsbDeviceInfo>> {
        let devices = self.context.devices()
            .context("Failed to get device list")?;
        
        let mut device_info_list = Vec::new();
        
        for device in devices.iter() {
            match self.get_device_info(&device) {
                Ok(info) => {
                    // Apply filter if set
                    if let Some(ref filter) = self.device_filter {
                        if let Some(ref product) = info.product {
                            if !product.to_lowercase().contains(&filter.to_lowercase()) {
                                continue;
                            }
                        } else if let Some(ref manufacturer) = info.manufacturer {
                            if !manufacturer.to_lowercase().contains(&filter.to_lowercase()) {
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
        for (key, prev_device) in &self.previous_devices {
            if !current_device_map.contains_key(key) {
                let mut disconnected_device = prev_device.clone();
                disconnected_device.connection_status = ConnectionStatus::Disconnected;
                disconnected_device.timestamp = Utc::now();
                changes.push(UsbDeviceChange::Disconnected(disconnected_device));
            }
        }
        
        // Check for new/reconnected devices
        for (key, current_device) in &current_device_map {
            match self.previous_devices.get(key) {
                None => {
                    // New device
                    let mut new_device = current_device.clone();
                    new_device.connection_status = ConnectionStatus::Connected;
                    changes.push(UsbDeviceChange::Connected(new_device));
                }
                Some(prev_device) => {
                    // Check if device was previously disconnected
                    if matches!(prev_device.connection_status, ConnectionStatus::Disconnected) {
                        let mut reconnected_device = current_device.clone();
                        reconnected_device.connection_status = ConnectionStatus::Reconnected;
                        changes.push(UsbDeviceChange::Reconnected(reconnected_device));
                    }
                }
            }
        }
        
        // Update previous devices state
        self.previous_devices = current_device_map;
        
        Ok(changes)
    }

    /// Start continuous monitoring
    pub async fn start_monitoring<F>(&mut self, mut callback: F) -> Result<()>
    where
        F: FnMut(Vec<UsbDeviceChange>) -> Result<()>,
    {
        info!("Starting USB device monitoring...");
        
        // Initial device scan
        let initial_devices = self.get_connected_devices()?;
        info!("Found {} initial USB devices", initial_devices.len());
        
        // Populate initial state
        for device in initial_devices {
            let key = self.create_device_key(&device);
            self.previous_devices.insert(key, device);
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
}

#[derive(Debug, Clone, Serialize)]
pub enum UsbDeviceChange {
    Connected(UsbDeviceInfo),
    Disconnected(UsbDeviceInfo),
    Reconnected(UsbDeviceInfo),
}

impl UsbDeviceChange {
    pub fn get_device_info(&self) -> &UsbDeviceInfo {
        match self {
            UsbDeviceChange::Connected(info) => info,
            UsbDeviceChange::Disconnected(info) => info,
            UsbDeviceChange::Reconnected(info) => info,
        }
    }
    
    pub fn get_change_type(&self) -> &str {
        match self {
            UsbDeviceChange::Connected(_) => "CONNECTED",
            UsbDeviceChange::Disconnected(_) => "DISCONNECTED",
            UsbDeviceChange::Reconnected(_) => "RECONNECTED",
        }
    }
}