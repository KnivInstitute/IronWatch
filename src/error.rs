use std::fmt;
use anyhow::Result as AnyhowResult;

/// Custom error types for IronWatch application
#[derive(Debug)]
pub enum IronWatchError {
    /// USB-related errors
    UsbError(UsbError),
    /// Configuration-related errors
    ConfigError(ConfigError),
    /// GUI-related errors
    GuiError(GuiError),
    /// System tray errors
    TrayError(TrayError),
    /// General I/O errors
    IoError(std::io::Error),
    /// Permission-related errors
    PermissionError(String),
}

#[derive(Debug)]
pub enum UsbError {
    /// Failed to initialize USB context
    InitializationFailed(String),
    /// No USB devices found
    NoDevicesFound,
    /// Failed to access device
    DeviceAccessDenied(u16, u16), // vendor_id, product_id
    /// Failed to read device descriptor
    DescriptorReadFailed(String),
    /// USB monitoring failed
    MonitoringFailed(String),
    /// Device enumeration failed
    EnumerationFailed(String),
}

#[derive(Debug)]
pub enum ConfigError {
    /// Configuration file not found
    FileNotFound(String),
    /// Invalid configuration format
    InvalidFormat(String),
    /// Configuration validation failed
    ValidationFailed(String),
    /// Failed to save configuration
    SaveFailed(String),
    /// Invalid configuration key
    InvalidKey(String),
    /// Invalid configuration value
    InvalidValue(String, String), // key, value
}

#[derive(Debug)]
pub enum GuiError {
    /// Failed to initialize GUI
    InitializationFailed(String),
    /// Failed to create window
    WindowCreationFailed(String),
    /// Failed to load theme/resources
    ResourceLoadFailed(String),
    /// Threading/async communication error
    CommunicationError(String),
}

#[derive(Debug)]
pub enum TrayError {
    /// Failed to create system tray
    CreationFailed(String),
    /// Failed to update tray icon
    IconUpdateFailed(String),
    /// Failed to show notification
    NotificationFailed(String),
}

impl fmt::Display for IronWatchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IronWatchError::UsbError(e) => write!(f, "USB Error: {}", e),
            IronWatchError::ConfigError(e) => write!(f, "Configuration Error: {}", e),
            IronWatchError::GuiError(e) => write!(f, "GUI Error: {}", e),
            IronWatchError::TrayError(e) => write!(f, "System Tray Error: {}", e),
            IronWatchError::IoError(e) => write!(f, "I/O Error: {}", e),
            IronWatchError::PermissionError(msg) => write!(f, "Permission Error: {}", msg),
        }
    }
}

impl fmt::Display for UsbError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UsbError::InitializationFailed(msg) => write!(f, "Failed to initialize USB context: {}", msg),
            UsbError::NoDevicesFound => write!(f, "No USB devices found"),
            UsbError::DeviceAccessDenied(vid, pid) => write!(f, "Access denied to device {:04x}:{:04x}", vid, pid),
            UsbError::DescriptorReadFailed(msg) => write!(f, "Failed to read device descriptor: {}", msg),
            UsbError::MonitoringFailed(msg) => write!(f, "USB monitoring failed: {}", msg),
            UsbError::EnumerationFailed(msg) => write!(f, "Device enumeration failed: {}", msg),
        }
    }
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::FileNotFound(path) => write!(f, "Configuration file not found: {}", path),
            ConfigError::InvalidFormat(msg) => write!(f, "Invalid configuration format: {}", msg),
            ConfigError::ValidationFailed(msg) => write!(f, "Configuration validation failed: {}", msg),
            ConfigError::SaveFailed(msg) => write!(f, "Failed to save configuration: {}", msg),
            ConfigError::InvalidKey(key) => write!(f, "Invalid configuration key: {}", key),
            ConfigError::InvalidValue(key, value) => write!(f, "Invalid value '{}' for key '{}'", value, key),
        }
    }
}

impl fmt::Display for GuiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GuiError::InitializationFailed(msg) => write!(f, "GUI initialization failed: {}", msg),
            GuiError::WindowCreationFailed(msg) => write!(f, "Window creation failed: {}", msg),
            GuiError::ResourceLoadFailed(msg) => write!(f, "Resource loading failed: {}", msg),
            GuiError::CommunicationError(msg) => write!(f, "GUI communication error: {}", msg),
        }
    }
}

impl fmt::Display for TrayError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TrayError::CreationFailed(msg) => write!(f, "System tray creation failed: {}", msg),
            TrayError::IconUpdateFailed(msg) => write!(f, "Tray icon update failed: {}", msg),
            TrayError::NotificationFailed(msg) => write!(f, "Notification failed: {}", msg),
        }
    }
}

impl std::error::Error for IronWatchError {}
impl std::error::Error for UsbError {}
impl std::error::Error for ConfigError {}
impl std::error::Error for GuiError {}
impl std::error::Error for TrayError {}

// Conversion implementations for easier error handling
impl From<std::io::Error> for IronWatchError {
    fn from(error: std::io::Error) -> Self {
        IronWatchError::IoError(error)
    }
}

impl From<rusb::Error> for IronWatchError {
    fn from(error: rusb::Error) -> Self {
        let usb_error = match error {
            rusb::Error::Access => UsbError::DeviceAccessDenied(0, 0),
            rusb::Error::NoDevice => UsbError::NoDevicesFound,
            rusb::Error::NotFound => UsbError::NoDevicesFound,
            _ => UsbError::EnumerationFailed(error.to_string()),
        };
        IronWatchError::UsbError(usb_error)
    }
}

impl From<serde_json::Error> for IronWatchError {
    fn from(error: serde_json::Error) -> Self {
        IronWatchError::ConfigError(ConfigError::InvalidFormat(error.to_string()))
    }
}

/// Result type alias for IronWatch operations
pub type Result<T> = std::result::Result<T, IronWatchError>;

/// Helper trait for better error messages
pub trait ErrorContext<T> {
    fn with_context(self, msg: &str) -> Result<T>;
}

impl<T> ErrorContext<T> for std::result::Result<T, IronWatchError> {
    fn with_context(self, msg: &str) -> Result<T> {
        self.map_err(|e| match e {
            IronWatchError::UsbError(usb_err) => IronWatchError::UsbError(
                UsbError::MonitoringFailed(format!("{}: {}", msg, usb_err))
            ),
            other => other,
        })
    }
}

/// Helper functions for creating specific errors
impl UsbError {
    pub fn initialization_failed(msg: impl Into<String>) -> IronWatchError {
        IronWatchError::UsbError(UsbError::InitializationFailed(msg.into()))
    }
    
    pub fn device_access_denied(vendor_id: u16, product_id: u16) -> IronWatchError {
        IronWatchError::UsbError(UsbError::DeviceAccessDenied(vendor_id, product_id))
    }
    
    pub fn monitoring_failed(msg: impl Into<String>) -> IronWatchError {
        IronWatchError::UsbError(UsbError::MonitoringFailed(msg.into()))
    }
}

impl ConfigError {
    pub fn file_not_found(path: impl Into<String>) -> IronWatchError {
        IronWatchError::ConfigError(ConfigError::FileNotFound(path.into()))
    }
    
    pub fn validation_failed(msg: impl Into<String>) -> IronWatchError {
        IronWatchError::ConfigError(ConfigError::ValidationFailed(msg.into()))
    }
}

impl GuiError {
    pub fn initialization_failed(msg: impl Into<String>) -> IronWatchError {
        IronWatchError::GuiError(GuiError::InitializationFailed(msg.into()))
    }
    
    pub fn communication_error(msg: impl Into<String>) -> IronWatchError {
        IronWatchError::GuiError(GuiError::CommunicationError(msg.into()))
    }
}

/// Check if the current user has sufficient permissions for USB access
pub fn check_usb_permissions() -> Result<()> {
    use rusb::UsbContext;
    
    // Try to create a USB context to check permissions
    match rusb::Context::new() {
        Ok(context) => {
            // Try to get device list to verify access
            match context.devices() {
                Ok(_) => Ok(()),
                Err(rusb::Error::Access) => Err(IronWatchError::PermissionError(
                    "Insufficient permissions to access USB devices. Try running as administrator or adding your user to the appropriate groups.".to_string()
                )),
                Err(e) => Err(IronWatchError::from(e)),
            }
        }
        Err(e) => Err(IronWatchError::from(e)),
    }
}

/// Provide user-friendly error messages with suggested solutions
pub fn get_user_friendly_message(error: &IronWatchError) -> (String, Option<String>) {
    match error {
        IronWatchError::UsbError(UsbError::DeviceAccessDenied(vid, pid)) => (
            format!("Cannot access USB device {:04x}:{:04x}", vid, pid),
            Some("Try running the application as administrator or check device permissions.".to_string())
        ),
        IronWatchError::UsbError(UsbError::InitializationFailed(_)) => (
            "Failed to initialize USB monitoring".to_string(),
            Some("Make sure you have proper USB access permissions and libusb is installed.".to_string())
        ),
        IronWatchError::PermissionError(_) => (
            "Insufficient permissions for USB access".to_string(),
            Some("Run as administrator or add your user to the 'plugdev' group on Linux.".to_string())
        ),
        IronWatchError::ConfigError(ConfigError::FileNotFound(_)) => (
            "Configuration file not found".to_string(),
            Some("A default configuration will be created automatically.".to_string())
        ),
        _ => (error.to_string(), None),
    }
}