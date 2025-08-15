use crate::usb_monitor::{UsbDeviceInfo, DeviceStatistics, DeviceAnalytics, SecurityEvent, SecurityEventType, SecurityAction};
use crate::communication::{CommunicationHub, MonitorEvent, MonitoringStatus};
use crate::error::{Result, get_user_friendly_message};
use crate::system_tray::{SystemTray, TrayMessage};

use eframe::egui::{self, *};
use std::time::Instant;

pub struct IronWatchGui {
    // Core state
    devices: Vec<UsbDeviceInfo>,
    communication_hub: CommunicationHub,
    monitoring_status: MonitoringStatus,
    
    // System tray
    system_tray: Option<SystemTray>,
    tray_sender: Option<std::sync::mpsc::Sender<TrayMessage>>,
    
    // UI state
    current_tab: Tab,
    
    // Animation state
    last_refresh: Instant,
    
    // Filtering
    search_filter: String,
    
    // Settings
    show_settings: bool,
    dark_mode: bool,
    show_animations: bool,
    
    // Error handling
    last_error: Option<String>,
    error_message: Option<String>,
    show_permission_dialog: bool,
    
    // Statistics
    device_analytics: Option<DeviceAnalytics>,
    selected_device_stats: Option<(String, DeviceStatistics)>,
    
    // Security
    security_events: Vec<SecurityEvent>,
    show_security_details: bool,
    selected_security_event: Option<usize>,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Tab {
    Dashboard,
    Devices,
    Monitoring,
    Statistics,
    Security,
    Settings,
}

impl Default for IronWatchGui {
    fn default() -> Self {
        panic!("Use IronWatchGui::new() instead of Default::default()")
    }
}

impl IronWatchGui {
    pub fn new(cc: &eframe::CreationContext<'_>, communication_hub: CommunicationHub) -> Self {
        let mut style = (*cc.egui_ctx.style()).clone();
        style.visuals.dark_mode = true;
        style.visuals.window_rounding = Rounding::same(10.0);
        cc.egui_ctx.set_style(style);
        
        let (system_tray, tray_sender) = match SystemTray::new() {
            Ok((tray, sender)) => {
                log::info!("System tray initialized successfully");
                (Some(tray), Some(sender))
            }
            Err(e) => {
                log::warn!("Failed to initialize system tray: {}", e);
                (None, None)
            }
        };
        
        let app = Self {
            devices: Vec::new(),
            communication_hub,
            monitoring_status: MonitoringStatus::Stopped,
            system_tray,
            tray_sender,
            current_tab: Tab::Dashboard,
            last_refresh: Instant::now(),
            search_filter: String::new(),
            show_settings: false,
            dark_mode: true,
            show_animations: true,
            last_error: None,
            error_message: None,
            show_permission_dialog: false,
            device_analytics: None,
            selected_device_stats: None,
            security_events: Vec::new(),
            show_security_details: false,
            selected_security_event: None,
        };
        
        let _ = app.communication_hub.refresh_devices();
        app
    }
    
    fn process_monitoring_events(&mut self) {
        while let Some(event) = self.communication_hub.try_recv_event() {
            match event {
                MonitorEvent::DevicesLoaded(devices) | MonitorEvent::DevicesUpdated(devices) => {
                    self.devices = devices;
                }
                MonitorEvent::DeviceChanged(change) => {
                    log::info!("Device change: {:?}", change);
                    // Show notification
                    let title = "USB Device Change";
                    let device_info = change.get_device_info();
                    let product_name = device_info.product.as_deref().unwrap_or("Unknown Device");
                    let message = format!("Device {} detected", product_name);
                    self.show_tray_notification(title, &message);
                    // Refresh device list after change
                    let _ = self.communication_hub.refresh_devices();
                }
                MonitorEvent::DevicesChanged(changes) => {
                    log::info!("Multiple device changes: {} devices", changes.len());
                    let title = "USB Devices Changed";
                    let message = format!("{} devices changed", changes.len());
                    self.show_tray_notification(title, &message);
                    // Refresh device list after changes
                    let _ = self.communication_hub.refresh_devices();
                }
                MonitorEvent::MonitoringStarted => {
                    self.monitoring_status = MonitoringStatus::Running;
                    self.update_tray_icon();
                }
                MonitorEvent::MonitoringStopped => {
                    self.monitoring_status = MonitoringStatus::Stopped;
                    self.update_tray_icon();
                }
                MonitorEvent::MonitoringError(error) => {
                    self.last_error = Some(error);
                }
                MonitorEvent::PermissionError(error) => {
                    self.last_error = Some(format!("Permission error: {}", error));
                    self.show_permission_dialog = true;
                }
                MonitorEvent::UsbUnavailable(error) => {
                    self.last_error = Some(format!("USB unavailable: {}", error));
                }
            }
        }
    }
    
    fn process_tray_messages(&mut self) {
        if let Some(tray) = &self.system_tray {
            let mut messages = Vec::new();
            
            while let Some(message) = tray.try_recv() {
                messages.push(message);
            }
            
            for message in messages {
                match message {
                    TrayMessage::Show => {
                        log::info!("Show window requested from system tray");
                    }
                    TrayMessage::Hide => {
                        log::info!("Hide window requested from system tray");
                    }
                    TrayMessage::ToggleMonitoring => {
                        self.toggle_monitoring();
                    }
                    TrayMessage::ShowSettings => {
                        self.show_settings = true;
                        self.current_tab = Tab::Settings;
                    }
                    TrayMessage::ShowAbout => {
                        log::info!("About requested from system tray");
                    }
                    TrayMessage::Quit => {
                        log::info!("Quit requested from system tray");
                        std::process::exit(0);
                    }
                }
            }
        }
    }
    
    fn is_monitoring_active(&self) -> bool {
        matches!(self.monitoring_status, MonitoringStatus::Running)
    }
    
    fn toggle_monitoring(&mut self) {
        if self.is_monitoring_active() {
            let _ = self.communication_hub.stop_monitoring();
        } else {
            let _ = self.communication_hub.start_monitoring();
        }
        
        // Update tray icon
        self.update_tray_icon();
    }
    
    fn update_tray_icon(&mut self) {
        if let Some(tray) = &self.system_tray {
            let is_monitoring = self.is_monitoring_active();
            if let Err(e) = tray.update_icon(is_monitoring) {
                log::warn!("Failed to update tray icon: {}", e);
            }
        }
    }
    
    fn show_tray_notification(&self, title: &str, message: &str) {
        if let Some(tray) = &self.system_tray {
            if let Err(e) = tray.show_notification(title, message) {
                log::warn!("Failed to show tray notification: {}", e);
            }
        }
    }
    
    fn render_top_panel(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.add_space(4.0);
            
            ui.horizontal(|ui| {
                ui.heading("🛡️ IronWatch");
                ui.label("v1.0.0 GUI");
                
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if self.is_monitoring_active() {
                        ui.colored_label(Color32::GREEN, "● MONITORING");
                    } else {
                        ui.colored_label(Color32::GRAY, "● IDLE");
                    }
                    
                    ui.separator();
                    ui.label(format!("📱 {} devices", self.devices.len()));
                });
            });
            
            ui.add_space(4.0);
            ui.separator();
            
            // Tab bar
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.current_tab, Tab::Dashboard, "📊 Dashboard");
                ui.selectable_value(&mut self.current_tab, Tab::Devices, "💾 Devices");
                ui.selectable_value(&mut self.current_tab, Tab::Monitoring, "👁 Monitoring");
                ui.selectable_value(&mut self.current_tab, Tab::Statistics, "📊 Statistics");
                ui.selectable_value(&mut self.current_tab, Tab::Security, "🛡️ Security");
                ui.selectable_value(&mut self.current_tab, Tab::Settings, "⚙️ Settings");
            });
        });
    }
    
    fn render_main_content(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            match self.current_tab {
                Tab::Dashboard => self.render_dashboard(ui),
                Tab::Devices => self.render_devices_tab(ui),
                Tab::Monitoring => self.render_monitoring_tab(ui),
                Tab::Statistics => self.render_statistics_tab(ui),
                Tab::Security => self.render_security_tab(ui),
                Tab::Settings => self.render_settings_tab(ui),
            }
        });
    }
    
    fn render_dashboard(&mut self, ui: &mut egui::Ui) {
        ui.heading("Dashboard");
        ui.add_space(20.0);
        
        // Stats
        ui.horizontal(|ui| {
            self.render_stat_card(ui, "Connected Devices", &self.devices.len().to_string(), Color32::BLUE);
            ui.add_space(20.0);
            self.render_stat_card(ui, "Monitoring Status", if self.is_monitoring_active() { "Active" } else { "Inactive" }, if self.is_monitoring_active() { Color32::GREEN } else { Color32::GRAY });
        });
        
        ui.add_space(30.0);
        
        // Quick actions
        ui.heading("Quick Actions");
        ui.add_space(10.0);
        
        ui.horizontal(|ui| {
            if ui.button("🔄 Refresh Devices").clicked() {
                let _ = self.communication_hub.refresh_devices();
            }
            
            ui.add_space(10.0);
            
            let monitor_text = if self.is_monitoring_active() {
                "⏸️ Stop Monitoring"
            } else {
                "▶️ Start Monitoring"
            };
            
            if ui.button(monitor_text).clicked() {
                self.toggle_monitoring();
            }
        });
        
        ui.add_space(30.0);
        
        // Recent devices
        ui.heading("Recent Devices");
        ui.add_space(10.0);
        
        egui::ScrollArea::vertical()
            .max_height(200.0)
            .show(ui, |ui| {
                for (i, device) in self.devices.iter().enumerate().take(5) {
                    ui.horizontal(|ui| {
                        ui.colored_label(Color32::BLUE, "●");
                        ui.label(device.product.as_deref().unwrap_or("Unknown Device"));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.small(format!("{:04X}:{:04X}", device.vendor_id, device.product_id));
                        });
                    });
                    
                    if i < 4 && i < self.devices.len() - 1 {
                        ui.separator();
                    }
                }
                
                if self.devices.is_empty() {
                    ui.label("No devices found. Click 'Refresh Devices' to scan.");
                }
            });
    }
    
    fn render_devices_tab(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading("USB Devices");
            
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("🔄 Refresh").clicked() {
                    let _ = self.communication_hub.refresh_devices();
                }
            });
        });
        
        ui.add_space(10.0);
        
        // Search filter
        ui.horizontal(|ui| {
            ui.label("Search:");
            ui.text_edit_singleline(&mut self.search_filter);
            
            if ui.button("Clear").clicked() {
                self.search_filter.clear();
            }
        });
        
        ui.add_space(10.0);
        
        // Device list
        egui::ScrollArea::vertical().show(ui, |ui| {
            egui::Grid::new("device_grid")
                .striped(true)
                .show(ui, |ui| {
                    // Header
                    ui.strong("Manufacturer");
                    ui.strong("Product");
                    ui.strong("VID:PID");
                    ui.strong("Bus");
                    ui.strong("Class");
                    ui.end_row();
                    
                    // Devices
                    for device in &self.devices {
                        // Apply search filter
                        if !self.search_filter.is_empty() {
                            let search_lower = self.search_filter.to_lowercase();
                            let matches = device.manufacturer.as_deref().unwrap_or("").to_lowercase().contains(&search_lower)
                                || device.product.as_deref().unwrap_or("").to_lowercase().contains(&search_lower);
                            
                            if !matches {
                                continue;
                            }
                        }
                        
                        ui.label(device.manufacturer.as_deref().unwrap_or("Unknown"));
                        ui.label(device.product.as_deref().unwrap_or("Unknown"));
                        ui.monospace(format!("{:04X}:{:04X}", device.vendor_id, device.product_id));
                        ui.label(device.bus_number.to_string());
                        ui.monospace(format!("{:02X}", device.device_class));
                        ui.end_row();
                    }
                });
        });
    }
    
    fn render_monitoring_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Real-time Monitoring");
        ui.add_space(20.0);
        
        // Controls
        ui.horizontal(|ui| {
            let button_text = if self.is_monitoring_active() {
                "⏸️ Stop Monitoring"
            } else {
                "▶️ Start Monitoring"
            };
            
            if ui.button(button_text).clicked() {
                self.toggle_monitoring();
            }
            
            ui.separator();
            
            ui.label("Status:");
            if self.is_monitoring_active() {
                ui.colored_label(Color32::GREEN, "● ACTIVE");
            } else {
                ui.colored_label(Color32::GRAY, "● INACTIVE");
            }
        });
        
        ui.add_space(30.0);
        
        if self.is_monitoring_active() {
            ui.label("🔍 Monitoring for USB device changes...");
            ui.add_space(10.0);
            ui.label("Connect or disconnect USB devices to see real-time updates.");
        } else {
            ui.label("Click 'Start Monitoring' to begin real-time USB device monitoring.");
        }
        
        ui.add_space(20.0);
        
        if self.is_monitoring_active() && self.last_refresh.elapsed().as_secs() >= 2 {
            let _ = self.communication_hub.refresh_devices();
            self.last_refresh = Instant::now();
        }
        
        // Current device count
        ui.separator();
        ui.add_space(10.0);
        ui.label(format!("Current device count: {}", self.devices.len()));
    }
    
    fn render_statistics_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Device Statistics");
        ui.add_space(20.0);

        // Analytics
        ui.heading("Overall Analytics");
        ui.add_space(10.0);
        if let Some(analytics) = &self.device_analytics {
            ui.label(format!("Total Devices Seen: {}", analytics.total_devices_seen));
            ui.label(format!("Unique Devices: {}", analytics.unique_devices));
            ui.label(format!("Blocked Devices: {}", analytics.blocked_devices));
            ui.label(format!("Security Violations: {}", analytics.security_violations));
            
            // Calculate total connections from history
            let total_connections = analytics.connection_frequency.iter().map(|(_, count)| count).sum::<u32>();
            ui.label(format!("Total Connections (24h): {}", total_connections));
        } else {
            ui.label("No analytics data available. Please enable monitoring.");
        }

        ui.add_space(30.0);

        // Device Class Distribution
        ui.heading("Device Class Distribution");
        ui.add_space(10.0);
        if let Some(analytics) = &self.device_analytics {
            if analytics.device_class_distribution.is_empty() {
                ui.label("No device class distribution data.");
            } else {
                egui::ScrollArea::vertical()
                    .max_height(200.0)
                    .show(ui, |ui| {
                        egui::Grid::new("class_distribution_grid")
                            .striped(true)
                            .show(ui, |ui| {
                                ui.strong("Class ID");
                                ui.strong("Count");
                                ui.end_row();

                                for (class_id, count) in &analytics.device_class_distribution {
                                    ui.label(format!("0x{:02x}", class_id));
                                    ui.label(count.to_string());
                                    ui.end_row();
                                }
                            });
                    });
            }
        }

        ui.add_space(30.0);

        // Vendor Distribution
        ui.heading("Vendor Distribution");
        ui.add_space(10.0);
        if let Some(analytics) = &self.device_analytics {
            if analytics.vendor_distribution.is_empty() {
                ui.label("No vendor distribution data.");
            } else {
                egui::ScrollArea::vertical()
                    .max_height(200.0)
                    .show(ui, |ui| {
                        egui::Grid::new("vendor_distribution_grid")
                            .striped(true)
                            .show(ui, |ui| {
                                ui.strong("Vendor ID");
                                ui.strong("Count");
                                ui.end_row();

                                for (vendor_name, stats) in &analytics.vendor_distribution {
                                    ui.label(format!("0x{:04x}", vendor_name));
                                    ui.label(stats.to_string());
                                    ui.end_row();
                                }
                            });
                    });
            }
        }

        ui.add_space(30.0);

        // Connection Frequency Chart
        ui.heading("Connection Frequency (Last 24 Hours)");
        ui.add_space(10.0);
        if let Some(analytics) = &self.device_analytics {
            if !analytics.connection_frequency.is_empty() {
                let max_connections = analytics.connection_frequency.iter().map(|(_, count)| count).max().unwrap_or(&0);
                
                egui::ScrollArea::horizontal()
                    .max_height(150.0)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            for (timestamp, count) in &analytics.connection_frequency {
                                let height = if *max_connections > 0 {
                                    (*count as f32 / *max_connections as f32) * 100.0
                                } else {
                                    0.0
                                };
                                
                                let hour = timestamp.format("%H").to_string();
                                ui.vertical(|ui| {
                                    ui.label(hour);
                                    ui.add_space(5.0);
                                    ui.allocate_ui(egui::vec2(20.0, 100.0), |ui| {
                                        let rect = ui.available_rect_before_wrap();
                                        ui.painter().rect_filled(
                                            rect,
                                            5.0,
                                            Color32::from_rgb(100, 150, 255)
                                        );
                                        ui.painter().rect_filled(
                                            rect.shrink(2.0),
                                            3.0,
                                            Color32::from_rgb(150, 200, 255)
                                        );
                                    });
                                    ui.label(count.to_string());
                                });
                            }
                        });
                    });
            }
        }

        ui.add_space(30.0);

        ui.horizontal(|ui| {
            if ui.button("🔄 Refresh Analytics").clicked() {
                // Request analytics refresh from communication hub
                self.refresh_analytics();
            }
            
            if ui.button("📤 Export Data").clicked() {
                self.export_analytics_data();
            }
        });
    }
    
    fn refresh_analytics(&mut self) {
        // For now, we'll simulate analytics data since we don't have direct access to USB monitor
        // In a real implementation, this would come from the communication hub
        use crate::usb_monitor::DeviceAnalytics;
        use std::collections::HashMap;
        
        let mut analytics = DeviceAnalytics {
            device_class_distribution: HashMap::new(),
            vendor_distribution: HashMap::new(),
            connection_frequency: Vec::new(),
            total_devices_seen: self.devices.len() as u32,
            unique_devices: self.devices.len() as u32,
            blocked_devices: 0,
            security_violations: 0,
        };
        
        // Generate sample analytics from current devices
        for device in &self.devices {
            *analytics.device_class_distribution.entry(device.device_class).or_insert(0) += 1;
            *analytics.vendor_distribution.entry(device.vendor_id).or_insert(0) += 1;
        }
        
        // Generate sample connection frequency (last 24 hours)
        let now = chrono::Utc::now();
        for hour in 0..24 {
            let hour_start = now - chrono::Duration::hours(24 - hour);
            let connections = if hour % 3 == 0 { 2 } else { 0 }; // Sample data
            analytics.connection_frequency.push((hour_start, connections));
        }
        
        self.device_analytics = Some(analytics);
        self.last_refresh = Instant::now();
    }
    
    fn export_analytics_data(&mut self) {
        use std::path::PathBuf;
        use chrono::Utc;
        
        // Generate export filename with timestamp
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        let export_path = PathBuf::from(format!("ironwatch_export_{}.json", timestamp));
        
        if let Some(analytics) = &self.device_analytics {
            // Create device stats from current devices
            let mut device_stats = Vec::new();
            for device in &self.devices {
                let key = format!("{}:{}:{}:{}", 
                    device.vendor_id, device.product_id, device.bus_number, device.device_address);
                let stats = crate::usb_monitor::DeviceStatistics {
                    total_connections: 1,
                    total_disconnections: 0,
                    total_blocked: 0,
                    first_seen: device.timestamp,
                    last_seen: device.timestamp,
                    connection_duration: std::time::Duration::ZERO,
                    connection_count: 1,
                };
                device_stats.push((key, stats));
            }
            
            // Export to JSON format
            let export_data = serde_json::json!({
                "export_timestamp": Utc::now(),
                "export_format": "json",
                "summary": {
                    "total_devices": self.devices.len(),
                    "total_connections": analytics.connection_frequency.iter().map(|(_, count)| count).sum::<u32>(),
                    "unique_devices": analytics.unique_devices,
                    "blocked_devices": analytics.blocked_devices,
                    "security_violations": analytics.security_violations,
                    "device_classes": analytics.device_class_distribution.len(),
                    "vendors": analytics.vendor_distribution.len(),
                },
                "current_devices": self.devices,
                "device_statistics": device_stats,
                "analytics": {
                    "device_class_distribution": analytics.device_class_distribution,
                    "vendor_distribution": analytics.vendor_distribution,
                    "connection_frequency": analytics.connection_frequency,
                },
                "security_events": self.security_events
            });
            
            match serde_json::to_string_pretty(&export_data) {
                Ok(json_string) => {
                    if let Err(e) = std::fs::write(&export_path, json_string) {
                        log::error!("Failed to export data: {}", e);
                    } else {
                        log::info!("Data exported successfully to: {}", export_path.display());
                    }
                }
                Err(e) => {
                    log::error!("Failed to serialize export data: {}", e);
                }
            }
        }
    }
    
    fn render_security_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("🛡️ Security Dashboard");
        ui.add_space(20.0);
        
        // Security Overview
        ui.heading("Security Overview");
        ui.add_space(10.0);
        
        let blocked_devices = self.devices.iter().filter(|d| matches!(d.connection_status, crate::usb_monitor::ConnectionStatus::Blocked)).count();
        let total_security_events = self.security_events.len();
        
        ui.horizontal(|ui| {
            self.render_security_card(ui, "Blocked Devices", &blocked_devices.to_string(), Color32::RED);
            ui.add_space(20.0);
            self.render_security_card(ui, "Security Events", &total_security_events.to_string(), Color32::from_rgb(255, 165, 0));
            ui.add_space(20.0);
            self.render_security_card(ui, "Active Rules", "0", Color32::BLUE); // Placeholder
        });
        
        ui.add_space(30.0);
        
        // Security Events
        ui.heading("Security Events");
        ui.add_space(10.0);
        
        if self.security_events.is_empty() {
            ui.label("No security events recorded. Start monitoring to see device security activity.");
        } else {
            egui::ScrollArea::vertical()
                .max_height(300.0)
                .show(ui, |ui| {
                    for (i, event) in self.security_events.iter().enumerate() {
                        let is_selected = self.selected_security_event == Some(i);
                        
                        ui.horizontal(|ui| {
                            // Event type icon and color
                            let (icon, color) = match event.event_type {
                                SecurityEventType::DeviceBlocked => ("🚫", Color32::RED),
                                SecurityEventType::DeviceAllowed => ("✅", Color32::GREEN),
                                SecurityEventType::RuleViolation => ("⚠️", Color32::from_rgb(255, 165, 0)),
                                SecurityEventType::SuspiciousActivity => ("🔍", Color32::YELLOW),
                            };
                            
                            ui.colored_label(color, icon);
                            
                            // Event details
                            ui.vertical(|ui| {
                                ui.horizontal(|ui| {
                                    ui.strong(format!("{} - {}", 
                                        event.timestamp.format("%H:%M:%S"),
                                        event.device_info.product.as_deref().unwrap_or("Unknown Device")
                                    ));
                                    
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        ui.small(format!("VID:{:04X} PID:{:04X}", 
                                            event.device_info.vendor_id, 
                                            event.device_info.product_id));
                                    });
                                });
                                
                                ui.label(format!("Action: {:?} - {}", event.action_taken, event.reason));
                            });
                            
                            // Select button
                            if ui.button(if is_selected { "Hide Details" } else { "Show Details" }).clicked() {
                                if is_selected {
                                    self.selected_security_event = None;
                                } else {
                                    self.selected_security_event = Some(i);
                                }
                            }
                        });
                        
                        // Show detailed information if selected
                        if is_selected {
                            ui.add_space(10.0);
                            ui.group(|ui| {
                                ui.label("Device Details:");
                                ui.label(format!("Manufacturer: {}", event.device_info.manufacturer.as_deref().unwrap_or("Unknown")));
                                ui.label(format!("Product: {}", event.device_info.product.as_deref().unwrap_or("Unknown")));
                                ui.label(format!("Serial: {}", event.device_info.serial_number.as_deref().unwrap_or("Unknown")));
                                ui.label(format!("Class: 0x{:02X}", event.device_info.device_class));
                                ui.label(format!("Bus: {} Address: {}", event.device_info.bus_number, event.device_info.device_address));
                                ui.label(format!("Timestamp: {}", event.timestamp.format("%Y-%m-%d %H:%M:%S UTC")));
                            });
                        }
                        
                        if i < self.security_events.len() - 1 {
                            ui.separator();
                        }
                    }
                });
        }
        
        ui.add_space(30.0);
        
        // Device Rules Management
        ui.heading("Device Rules Management");
        ui.add_space(10.0);
        
        ui.horizontal(|ui| {
            if ui.button("📋 View Rules").clicked() {
                // TODO: Show rules management dialog
                log::info!("Rules management requested");
            }
            
            ui.add_space(10.0);
            
            if ui.button("➕ Add Rule").clicked() {
                // TODO: Show add rule dialog
                log::info!("Add rule requested");
            }
            
            ui.add_space(10.0);
            
            if ui.button("🔄 Refresh Security").clicked() {
                // TODO: Refresh security data
                log::info!("Security refresh requested");
            }
        });
        
        ui.add_space(20.0);
        
        // Security Statistics
        ui.heading("Security Statistics");
        ui.add_space(10.0);
        
        if let Some(analytics) = &self.device_analytics {
            ui.label(format!("Total Blocked Devices: {}", analytics.blocked_devices));
            ui.label(format!("Security Violations: {}", analytics.security_violations));
        } else {
            ui.label("No security analytics available. Please enable monitoring.");
        }
    }
    
    fn render_security_card(&self, ui: &mut egui::Ui, title: &str, value: &str, color: Color32) {
        egui::Frame::none()
            .fill(color.gamma_multiply(0.1))
            .rounding(8.0)
            .inner_margin(egui::Margin::same(15.0))
            .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.colored_label(color, value);
                    ui.small(title);
                });
            });
    }
    
    fn render_settings_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Settings");
        ui.add_space(20.0);
        
        ui.checkbox(&mut self.dark_mode, "Dark Mode");
        ui.checkbox(&mut self.show_animations, "Enable Animations");
        
        ui.add_space(20.0);
        
        ui.heading("System Integration");
        ui.add_space(10.0);
        
        let mut tray_enabled = self.system_tray.is_some();
        if ui.checkbox(&mut tray_enabled, "Enable System Tray").clicked() {
            if tray_enabled && self.system_tray.is_none() {
                // Try to create system tray
                match SystemTray::new() {
                    Ok((tray, sender)) => {
                        self.system_tray = Some(tray);
                        self.tray_sender = Some(sender);
                        log::info!("System tray enabled");
                    }
                    Err(e) => {
                        log::warn!("Failed to enable system tray: {}", e);
                        self.last_error = Some(format!("Failed to enable system tray: {}", e));
                    }
                }
            } else if !tray_enabled && self.system_tray.is_some() {
                // Disable system tray
                self.system_tray = None;
                self.tray_sender = None;
                log::info!("System tray disabled");
            }
        }
        
        if tray_enabled {
            ui.label("System tray is active and will show notifications for USB changes");
        } else {
            ui.label("System tray is disabled");
        }
        
        ui.add_space(20.0);
        
        if ui.button("💾 Save Settings").clicked() {
            // TODO: Save settings to config
            log::info!("Settings saved (placeholder)");
        }
        
        if ui.button("🔄 Reset to Defaults").clicked() {
            self.dark_mode = true;
            self.show_animations = true;
        }
        
        ui.add_space(30.0);
        ui.separator();
        ui.add_space(10.0);
        
        ui.heading("About");
        ui.add_space(10.0);
        ui.label("IronWatch v1.0.0");
        ui.label("USB Device Input Monitor");
        ui.label("by KnivInstitute");
        ui.add_space(5.0);
        ui.small("Built with Rust + egui");
    }
    
    fn render_stat_card(&self, ui: &mut egui::Ui, title: &str, value: &str, color: Color32) {
        egui::Frame::none()
            .fill(color.gamma_multiply(0.1))
            .rounding(8.0)
            .inner_margin(egui::Margin::same(15.0))
            .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.heading(value);
                    ui.small(title);
                });
            });
    }
}

impl eframe::App for IronWatchGui {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Process monitoring events
        self.process_monitoring_events();
        
        // Process tray messages
        self.process_tray_messages();
        
        // Auto-refresh devices periodically
        if self.last_refresh.elapsed().as_secs() >= 5 {
            let _ = self.communication_hub.refresh_devices();
            self.last_refresh = Instant::now();
        }
        
        // Render UI
        self.render_top_panel(ctx);
        self.render_main_content(ctx);
        
        // Show error dialogs if needed
        if let Some(error) = &self.error_message.clone() {
            egui::Window::new("Error")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.label(error);
                    if ui.button("OK").clicked() {
                        self.error_message = None;
                    }
                });
        }
        
        // Show permission dialog if needed
        if self.show_permission_dialog {
            egui::Window::new("Permission Required")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.label("USB access requires elevated permissions.");
                    ui.add_space(10.0);
                    ui.label("Please run the application as administrator or check device permissions.");
                    ui.add_space(10.0);
                    if ui.button("OK").clicked() {
                        self.show_permission_dialog = false;
                    }
                });
        }
        
        // Request repaint for animations
        if self.show_animations {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
    }
    
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        // Clean up system tray on exit
        if self.system_tray.is_some() {
            log::info!("Cleaning up system tray on exit");
        }
    }
    
    fn auto_save_interval(&self) -> std::time::Duration {
        // Auto-save every 30 seconds
        std::time::Duration::from_secs(30)
    }
    
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        // Use dark theme background color
        [0.1, 0.1, 0.1, 1.0]
    }
}