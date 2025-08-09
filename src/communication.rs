use crate::usb_monitor::{UsbDeviceInfo, UsbDeviceChange};
use crate::error::{Result, IronWatchError, GuiError};
use std::sync::{Arc, Mutex};
use tokio::sync::{mpsc, oneshot, broadcast};
use std::time::Duration;

/// Messages sent from GUI to background monitoring thread
#[derive(Debug, Clone)]
pub enum MonitorCommand {
    /// Start USB monitoring
    StartMonitoring,
    /// Stop USB monitoring
    StopMonitoring,
    /// Refresh device list once
    RefreshDevices,
    /// Set device filter
    SetFilter(Option<String>),
    /// Update polling interval
    SetPollingInterval(Duration),
    /// Shutdown the monitoring thread
    Shutdown,
}

/// Messages sent from background monitoring thread to GUI
#[derive(Debug, Clone)]
pub enum MonitorEvent {
    /// Initial device list loaded
    DevicesLoaded(Vec<UsbDeviceInfo>),
    /// Device list updated
    DevicesUpdated(Vec<UsbDeviceInfo>),
    /// USB device change detected
    DeviceChanged(UsbDeviceChange),
    /// Multiple device changes
    DevicesChanged(Vec<UsbDeviceChange>),
    /// Monitoring started successfully
    MonitoringStarted,
    /// Monitoring stopped
    MonitoringStopped,
    /// Error occurred in monitoring
    MonitoringError(String),
    /// Permission error - needs user attention
    PermissionError(String),
    /// USB subsystem unavailable
    UsbUnavailable(String),
}

/// Status of the monitoring system
#[derive(Debug, Clone, PartialEq)]
pub enum MonitoringStatus {
    Stopped,
    Starting,
    Running,
    Stopping,
    Error(String),
}

/// Communication hub for GUI-background thread coordination
#[derive(Clone)]
pub struct CommunicationHub {
    /// Channel for sending commands to monitoring thread
    command_sender: mpsc::UnboundedSender<MonitorCommand>,
    /// Channel for receiving events from monitoring thread
    event_receiver: Arc<Mutex<mpsc::UnboundedReceiver<MonitorEvent>>>,
    /// Broadcast channel for status updates
    status_sender: broadcast::Sender<MonitoringStatus>,
    /// Current monitoring status
    current_status: Arc<Mutex<MonitoringStatus>>,
    /// Current device list
    current_devices: Arc<Mutex<Vec<UsbDeviceInfo>>>,
}

impl CommunicationHub {
    /// Create a new communication hub
    pub fn new() -> (Self, CommunicationReceiver) {
        let (command_sender, command_receiver) = mpsc::unbounded_channel();
        let (event_sender, event_receiver) = mpsc::unbounded_channel();
        let (status_sender, _status_receiver) = broadcast::channel(100);
        
        let current_status = Arc::new(Mutex::new(MonitoringStatus::Stopped));
        let current_devices = Arc::new(Mutex::new(Vec::new()));
        
        let hub = Self {
            command_sender,
            event_receiver: Arc::new(Mutex::new(event_receiver)),
            status_sender: status_sender.clone(),
            current_status: current_status.clone(),
            current_devices: current_devices.clone(),
        };
        
        let receiver = CommunicationReceiver {
            command_receiver,
            event_sender,
            status_sender,
            current_status,
            current_devices,
        };
        
        (hub, receiver)
    }
    
    /// Send a command to the monitoring thread
    pub fn send_command(&self, command: MonitorCommand) -> Result<()> {
        self.command_sender.send(command)
            .map_err(|_| GuiError::communication_error("Failed to send command to monitoring thread"))?;
        Ok(())
    }
    
    /// Try to receive an event from the monitoring thread (non-blocking)
    pub fn try_recv_event(&self) -> Option<MonitorEvent> {
        if let Ok(mut receiver) = self.event_receiver.try_lock() {
            receiver.try_recv().ok()
        } else {
            None
        }
    }
    
    /// Get current monitoring status
    pub fn get_status(&self) -> MonitoringStatus {
        self.current_status.lock().unwrap().clone()
    }
    
    /// Subscribe to status updates
    pub fn subscribe_status(&self) -> broadcast::Receiver<MonitoringStatus> {
        self.status_sender.subscribe()
    }
    
    /// Get current device list
    pub fn get_devices(&self) -> Vec<UsbDeviceInfo> {
        self.current_devices.lock().unwrap().clone()
    }
    
    /// Start monitoring with error handling
    pub fn start_monitoring(&self) -> Result<()> {
        self.send_command(MonitorCommand::StartMonitoring)
    }
    
    /// Stop monitoring
    pub fn stop_monitoring(&self) -> Result<()> {
        self.send_command(MonitorCommand::StopMonitoring)
    }
    
    /// Refresh device list
    pub fn refresh_devices(&self) -> Result<()> {
        self.send_command(MonitorCommand::RefreshDevices)
    }
    
    /// Set device filter
    pub fn set_filter(&self, filter: Option<String>) -> Result<()> {
        self.send_command(MonitorCommand::SetFilter(filter))
    }
    
    /// Shutdown the monitoring system
    pub fn shutdown(&self) -> Result<()> {
        self.send_command(MonitorCommand::Shutdown)
    }
}

/// Receiver side of the communication hub (runs in background thread)
pub struct CommunicationReceiver {
    command_receiver: mpsc::UnboundedReceiver<MonitorCommand>,
    event_sender: mpsc::UnboundedSender<MonitorEvent>,
    status_sender: broadcast::Sender<MonitoringStatus>,
    current_status: Arc<Mutex<MonitoringStatus>>,
    current_devices: Arc<Mutex<Vec<UsbDeviceInfo>>>,
}

impl CommunicationReceiver {
    /// Send an event to the GUI thread
    pub fn send_event(&self, event: MonitorEvent) -> Result<()> {
        // Update internal state based on event
        match &event {
            MonitorEvent::DevicesLoaded(devices) | MonitorEvent::DevicesUpdated(devices) => {
                *self.current_devices.lock().unwrap() = devices.clone();
            }
            MonitorEvent::MonitoringStarted => {
                self.update_status(MonitoringStatus::Running);
            }
            MonitorEvent::MonitoringStopped => {
                self.update_status(MonitoringStatus::Stopped);
            }
            MonitorEvent::MonitoringError(err) => {
                self.update_status(MonitoringStatus::Error(err.clone()));
            }
            MonitorEvent::PermissionError(err) => {
                self.update_status(MonitoringStatus::Error(format!("Permission: {}", err)));
            }
            MonitorEvent::UsbUnavailable(err) => {
                self.update_status(MonitoringStatus::Error(format!("USB Unavailable: {}", err)));
            }
            _ => {}
        }
        
        self.event_sender.send(event)
            .map_err(|_| GuiError::communication_error("Failed to send event to GUI thread"))?;
        Ok(())
    }
    
    /// Receive a command from the GUI thread (blocking)
    pub async fn recv_command(&mut self) -> Option<MonitorCommand> {
        self.command_receiver.recv().await
    }
    
    /// Try to receive a command (non-blocking)
    pub fn try_recv_command(&mut self) -> Option<MonitorCommand> {
        self.command_receiver.try_recv().ok()
    }
    
    /// Update the monitoring status
    fn update_status(&self, status: MonitoringStatus) {
        *self.current_status.lock().unwrap() = status.clone();
        let _ = self.status_sender.send(status);
    }
    
    /// Send monitoring started event
    pub fn send_monitoring_started(&self) -> Result<()> {
        self.send_event(MonitorEvent::MonitoringStarted)
    }
    
    /// Send monitoring stopped event
    pub fn send_monitoring_stopped(&self) -> Result<()> {
        self.send_event(MonitorEvent::MonitoringStopped)
    }
    
    /// Send error event
    pub fn send_error(&self, error: &str) -> Result<()> {
        self.send_event(MonitorEvent::MonitoringError(error.to_string()))
    }
    
    /// Send permission error event
    pub fn send_permission_error(&self, error: &str) -> Result<()> {
        self.send_event(MonitorEvent::PermissionError(error.to_string()))
    }
    
    /// Send devices loaded event
    pub fn send_devices_loaded(&self, devices: Vec<UsbDeviceInfo>) -> Result<()> {
        self.send_event(MonitorEvent::DevicesLoaded(devices))
    }
    
    /// Send devices updated event
    pub fn send_devices_updated(&self, devices: Vec<UsbDeviceInfo>) -> Result<()> {
        self.send_event(MonitorEvent::DevicesUpdated(devices))
    }
    
    /// Send device change event
    pub fn send_device_change(&self, change: UsbDeviceChange) -> Result<()> {
        self.send_event(MonitorEvent::DeviceChanged(change))
    }
}

/// Helper for graceful shutdown coordination
pub struct ShutdownCoordinator {
    shutdown_flag: Arc<std::sync::atomic::AtomicBool>,
}

impl ShutdownCoordinator {
    pub fn new() -> Self {
        Self {
            shutdown_flag: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }
    
    /// Signal shutdown
    pub fn signal_shutdown(&mut self) {
        self.shutdown_flag.store(true, std::sync::atomic::Ordering::Relaxed);
    }
    
    /// Wait for shutdown signal
    pub async fn wait_for_shutdown(&mut self) {
        loop {
            if self.shutdown_flag.load(std::sync::atomic::Ordering::Relaxed) {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
    }
    
    /// Check if shutdown was signaled (non-blocking)
    pub fn is_shutdown_signaled(&mut self) -> bool {
        self.shutdown_flag.load(std::sync::atomic::Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_communication_hub() {
        let (hub, mut receiver) = CommunicationHub::new();
        
        // Test command sending
        hub.start_monitoring().unwrap();
        let command = receiver.recv_command().await;
        assert!(matches!(command, Some(MonitorCommand::StartMonitoring)));
        
        // Test event sending
        let devices = vec![];
        receiver.send_devices_loaded(devices).unwrap();
        let event = hub.try_recv_event();
        assert!(matches!(event, Some(MonitorEvent::DevicesLoaded(_))));
    }
    
    #[test]
    fn test_shutdown_coordinator() {
        let mut coordinator = ShutdownCoordinator::new();
        assert!(!coordinator.is_shutdown_signaled());
        
        coordinator.signal_shutdown();
        assert!(coordinator.is_shutdown_signaled());
    }
}