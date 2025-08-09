use crate::usb_monitor::{UsbMonitor, UsbDeviceChange};
use crate::communication::{CommunicationReceiver, MonitorCommand, ShutdownCoordinator};
use crate::error::{Result, UsbError, IronWatchError, check_usb_permissions};
use std::time::Duration;
use tokio::time::{interval, sleep};
use log::{info, error, debug, warn};

/// Background service that handles USB monitoring
pub struct MonitoringService {
    usb_monitor: Option<UsbMonitor>,
    communication: CommunicationReceiver,
    shutdown_coordinator: ShutdownCoordinator,
    polling_interval: Duration,
    device_filter: Option<String>,
    is_monitoring: bool,
}

impl MonitoringService {
    /// Create a new monitoring service
    pub fn new(communication: CommunicationReceiver) -> Self {
        Self {
            usb_monitor: None,
            communication,
            shutdown_coordinator: ShutdownCoordinator::new(),
            polling_interval: Duration::from_millis(500),
            device_filter: None,
            is_monitoring: false,
        }
    }
    
    /// Initialize the USB monitor with graceful error handling
    async fn initialize_usb_monitor(&mut self) -> Result<()> {
        // Check permissions first
        if let Err(e) = check_usb_permissions() {
            match &e {
                IronWatchError::PermissionError(msg) => {
                    self.communication.send_permission_error(msg)?;
                    return Err(e);
                }
                _ => {
                    self.communication.send_error(&format!("USB initialization failed: {}", e))?;
                    return Err(e);
                }
            }
        }
        
        // Try to create USB monitor
        match UsbMonitor::new() {
            Ok(mut monitor) => {
                // Set filter if configured
                monitor.set_filter(self.device_filter.clone());
                self.usb_monitor = Some(monitor);
                info!("USB monitor initialized successfully");
                Ok(())
            }
            Err(e) => {
                let error_msg = format!("Failed to initialize USB monitor: {}", e);
                error!("{}", error_msg);
                self.communication.send_error(&error_msg)?;
                Err(UsbError::initialization_failed(error_msg))
            }
        }
    }
    
    /// Start the monitoring service
    pub async fn run(&mut self) -> Result<()> {
        info!("Starting monitoring service");
        
        // Try to initialize USB monitor
        if let Err(e) = self.initialize_usb_monitor().await {
            warn!("USB monitor initialization failed, running in degraded mode: {}", e);
            // Continue running to handle commands, but USB functionality will be limited
        }
        
        // Main service loop
        let mut poll_timer = interval(self.polling_interval);
        
        loop {
            tokio::select! {
                // Handle shutdown signal
                _ = self.shutdown_coordinator.wait_for_shutdown() => {
                    info!("Shutdown signal received, stopping monitoring service");
                    break;
                }
                
                // Handle commands from GUI
                command = self.communication.recv_command() => {
                    match command {
                        Some(cmd) => {
                            if let Err(e) = self.handle_command(cmd).await {
                                error!("Error handling command: {}", e);
                            }
                        }
                        None => {
                            debug!("Command channel closed, shutting down");
                            break;
                        }
                    }
                }
                
                // Periodic USB monitoring (only if monitoring is active)
                _ = poll_timer.tick(), if self.is_monitoring => {
                    if let Err(e) = self.perform_monitoring_cycle().await {
                        error!("Monitoring cycle error: {}", e);
                        // Don't break on monitoring errors, just log and continue
                    }
                }
            }
        }
        
        // Cleanup
        self.stop_monitoring().await?;
        info!("Monitoring service stopped");
        Ok(())
    }
    
    /// Handle commands from the GUI
    async fn handle_command(&mut self, command: MonitorCommand) -> Result<()> {
        debug!("Handling command: {:?}", command);
        
        match command {
            MonitorCommand::StartMonitoring => {
                self.start_monitoring().await?;
            }
            MonitorCommand::StopMonitoring => {
                self.stop_monitoring().await?;
            }
            MonitorCommand::RefreshDevices => {
                self.refresh_devices().await?;
            }
            MonitorCommand::SetFilter(filter) => {
                self.set_filter(filter).await?;
            }
            MonitorCommand::SetPollingInterval(interval) => {
                self.set_polling_interval(interval).await?;
            }
            MonitorCommand::Shutdown => {
                info!("Received shutdown command");
                self.shutdown_coordinator.signal_shutdown();
            }
        }
        
        Ok(())
    }
    
    /// Start USB monitoring
    async fn start_monitoring(&mut self) -> Result<()> {
        if self.is_monitoring {
            debug!("Monitoring already active");
            return Ok(());
        }
        
        // Ensure USB monitor is initialized
        if self.usb_monitor.is_none() {
            if let Err(e) = self.initialize_usb_monitor().await {
                return Err(e);
            }
        }
        
        self.is_monitoring = true;
        self.communication.send_monitoring_started()?;
        info!("USB monitoring started");
        
        // Send initial device list
        self.refresh_devices().await?;
        
        Ok(())
    }
    
    /// Stop USB monitoring
    async fn stop_monitoring(&mut self) -> Result<()> {
        if !self.is_monitoring {
            return Ok(());
        }
        
        self.is_monitoring = false;
        self.communication.send_monitoring_stopped()?;
        info!("USB monitoring stopped");
        
        Ok(())
    }
    
    /// Refresh the device list
    async fn refresh_devices(&mut self) -> Result<()> {
        if let Some(ref monitor) = self.usb_monitor {
            match monitor.get_connected_devices() {
                Ok(devices) => {
                    debug!("Found {} USB devices", devices.len());
                    self.communication.send_devices_updated(devices)?;
                }
                Err(e) => {
                    let error_msg = format!("Failed to get device list: {}", e);
                    error!("{}", error_msg);
                    self.communication.send_error(&error_msg)?;
                    return Err(UsbError::monitoring_failed(error_msg));
                }
            }
        } else {
            // USB monitor not available, send empty list
            warn!("USB monitor not available, sending empty device list");
            self.communication.send_devices_updated(vec![])?;
        }
        
        Ok(())
    }
    
    /// Set device filter
    async fn set_filter(&mut self, filter: Option<String>) -> Result<()> {
        self.device_filter = filter.clone();
        
        if let Some(ref mut monitor) = self.usb_monitor {
            monitor.set_filter(filter);
            // Refresh devices with new filter
            self.refresh_devices().await?;
        }
        
        Ok(())
    }
    
    /// Set polling interval
    async fn set_polling_interval(&mut self, interval: Duration) -> Result<()> {
        self.polling_interval = interval;
        info!("Polling interval updated to {:?}", interval);
        Ok(())
    }
    
    /// Perform one monitoring cycle
    async fn perform_monitoring_cycle(&mut self) -> Result<()> {
        if let Some(ref mut monitor) = self.usb_monitor {
            match monitor.monitor_changes().await {
                Ok(changes) => {
                    if !changes.is_empty() {
                        debug!("Detected {} device changes", changes.len());
                        for change in changes {
                            self.communication.send_device_change(change)?;
                        }
                    }
                }
                Err(e) => {
                    let error_msg = format!("Device monitoring error: {}", e);
                    error!("{}", error_msg);
                    
                    // Check if it's a permission error
                    if error_msg.contains("Access") || error_msg.contains("Permission") {
                        self.communication.send_permission_error(&error_msg)?;
                    } else {
                        self.communication.send_error(&error_msg)?;
                    }
                    
                    return Err(UsbError::monitoring_failed(error_msg));
                }
            }
        }
        
        Ok(())
    }
    
    /// Get the shutdown coordinator (for external shutdown signaling)
    pub fn shutdown_coordinator(&mut self) -> &mut ShutdownCoordinator {
        &mut self.shutdown_coordinator
    }
}

/// Spawn the monitoring service in a background task
pub fn spawn_monitoring_service(communication: CommunicationReceiver) -> tokio::task::JoinHandle<Result<()>> {
    tokio::spawn(async move {
        let mut service = MonitoringService::new(communication);
        service.run().await
    })
}

/// Helper function to create and start the monitoring service with error recovery
pub async fn start_monitoring_service_with_recovery(
    communication: CommunicationReceiver,
    max_retries: usize,
) -> Result<tokio::task::JoinHandle<Result<()>>> {
    let mut retries = 0;
    
    loop {
        // Try to check USB permissions before starting
        match check_usb_permissions() {
            Ok(()) => {
                info!("USB permissions verified, starting monitoring service");
                let handle = spawn_monitoring_service(communication);
                return Ok(handle);
            }
            Err(e) => {
                error!("USB permission check failed: {}", e);
                
                if retries >= max_retries {
                    error!("Max retries reached, starting service in degraded mode");
                    let handle = spawn_monitoring_service(communication);
                    return Ok(handle);
                }
                
                retries += 1;
                warn!("Retrying USB permission check in 2 seconds... ({}/{})", retries, max_retries);
                sleep(Duration::from_secs(2)).await;
            }
        }
    }
}