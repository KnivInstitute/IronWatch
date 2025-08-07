use crate::usb_monitor::{UsbMonitor, UsbDeviceInfo};

use eframe::egui::{self, *};
use std::sync::{Arc, Mutex};
use std::time::Instant;

pub struct IronWatchGui {
    // Core state
    devices: Vec<UsbDeviceInfo>,
    usb_monitor: Arc<Mutex<Option<UsbMonitor>>>,
    
    // UI state
    current_tab: Tab,
    monitoring_active: bool,
    
    // Animation state
    last_refresh: Instant,
    
    // Filtering
    search_filter: String,
    
    // Settings
    show_settings: bool,
    dark_mode: bool,
    show_animations: bool,
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
        Self {
            devices: Vec::new(),
            usb_monitor: Arc::new(Mutex::new(None)),
            current_tab: Tab::Dashboard,
            monitoring_active: false,
            last_refresh: Instant::now(),
            search_filter: String::new(),
            show_settings: false,
            dark_mode: true,
            show_animations: true,
        }
    }
}

impl IronWatchGui {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Configure style
        let mut style = (*cc.egui_ctx.style()).clone();
        style.visuals.dark_mode = true;
        style.visuals.window_rounding = Rounding::same(10.0);
        cc.egui_ctx.set_style(style);
        
        let mut app = Self::default();
        app.initialize_usb_monitoring();
        app
    }
    
    fn initialize_usb_monitoring(&mut self) {
        match UsbMonitor::new() {
            Ok(monitor) => {
                *self.usb_monitor.lock().unwrap() = Some(monitor);
                log::info!("USB monitoring initialized");
            }
            Err(e) => {
                log::error!("Failed to initialize USB monitoring: {}", e);
            }
        }
    }
    
    fn refresh_devices(&mut self) {
        if let Some(ref mut monitor) = *self.usb_monitor.lock().unwrap() {
            match monitor.get_connected_devices() {
                Ok(devices) => {
                    self.devices = devices;
                }
                Err(e) => {
                    log::error!("Error getting devices: {}", e);
                }
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
                    if self.monitoring_active {
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
            self.render_stat_card(ui, "Monitoring Status", if self.monitoring_active { "Active" } else { "Inactive" }, if self.monitoring_active { Color32::GREEN } else { Color32::GRAY });
        });
        
        ui.add_space(30.0);
        
        // Quick actions
        ui.heading("Quick Actions");
        ui.add_space(10.0);
        
        ui.horizontal(|ui| {
            if ui.button("🔄 Refresh Devices").clicked() {
                self.refresh_devices();
            }
            
            ui.add_space(10.0);
            
            let monitor_text = if self.monitoring_active {
                "⏸️ Stop Monitoring"
            } else {
                "▶️ Start Monitoring"
            };
            
            if ui.button(monitor_text).clicked() {
                self.monitoring_active = !self.monitoring_active;
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
                    self.refresh_devices();
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
            let button_text = if self.monitoring_active {
                "⏸️ Stop Monitoring"
            } else {
                "▶️ Start Monitoring"
            };
            
            if ui.button(button_text).clicked() {
                self.monitoring_active = !self.monitoring_active;
            }
            
            ui.separator();
            
            ui.label("Status:");
            if self.monitoring_active {
                ui.colored_label(Color32::GREEN, "● ACTIVE");
            } else {
                ui.colored_label(Color32::GRAY, "● INACTIVE");
            }
        });
        
        ui.add_space(30.0);
        
        if self.monitoring_active {
            ui.label("🔍 Monitoring for USB device changes...");
            ui.add_space(10.0);
            ui.label("Connect or disconnect USB devices to see real-time updates.");
        } else {
            ui.label("Click 'Start Monitoring' to begin real-time USB device monitoring.");
        }
        
        ui.add_space(20.0);
        
        // Auto-refresh when monitoring
        if self.monitoring_active && self.last_refresh.elapsed().as_secs() >= 2 {
            self.refresh_devices();
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
        // Auto-refresh devices periodically
        if self.last_refresh.elapsed().as_secs() >= 5 {
            self.refresh_devices();
            self.last_refresh = Instant::now();
        }
        
        // Render UI
        self.render_top_panel(ctx);
        self.render_main_content(ctx);
        
        // Request repaint for animations
        if self.show_animations {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
    }
}