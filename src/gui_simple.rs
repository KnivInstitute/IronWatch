use crate::usb_monitor::UsbDeviceInfo;
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
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Tab {
    Dashboard,
    Devices,
    Monitoring,
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