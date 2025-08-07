use crate::gui::*;
use eframe::egui::{self, *};
use egui_extras::{Column, TableBuilder};
use egui_plot::{Line, Plot, PlotPoints};

impl IronWatchGui {
    pub fn render_top_panel(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.add_space(4.0);
            
            ui.horizontal(|ui| {
                // Logo and title
                ui.heading("🛡️ IronWatch");
                ui.label("v1.0.0");
                
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Status indicators
                    if self.monitoring_active {
                        let pulse_color = Color32::from_rgb(
                            (255.0 * self.pulse_animation) as u8,
                            100,
                            100,
                        );
                        ui.colored_label(pulse_color, "● MONITORING");
                    } else {
                        ui.colored_label(Color32::GRAY, "● IDLE");
                    }
                    
                    ui.separator();
                    
                    // Device count
                    ui.label(format!("📱 {} devices", self.devices.len()));
                    
                    ui.separator();
                    
                    // FPS counter (debug)
                    if ui.small_button("🔧").clicked() {
                        self.show_settings = true;
                    }
                    
                    ui.label(format!("FPS: {:.0}", self.fps_counter));
                });
            });
            
            ui.add_space(4.0);
            ui.separator();
            
            // Tab bar
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.current_tab, Tab::Dashboard, "📊 Dashboard");
                ui.selectable_value(&mut self.current_tab, Tab::Devices, "💾 Devices");
                ui.selectable_value(&mut self.current_tab, Tab::Monitoring, "👁 Monitoring");
                ui.selectable_value(&mut self.current_tab, Tab::Analytics, "📈 Analytics");
                ui.selectable_value(&mut self.current_tab, Tab::Settings, "⚙️ Settings");
            });
            
            ui.add_space(2.0);
        });
    }
    
    pub fn render_main_content(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            match self.current_tab {
                Tab::Dashboard => self.render_dashboard(ui),
                Tab::Devices => self.render_devices_tab(ui),
                Tab::Monitoring => self.render_monitoring_tab(ui),
                Tab::Analytics => self.render_analytics_tab(ui),
                Tab::Settings => self.render_settings_tab(ui),
            }
        });
    }
    
    pub fn render_dashboard(&mut self, ui: &mut egui::Ui) {
        ui.heading("Dashboard");
        ui.add_space(10.0);
        
        // Stats cards
        ui.horizontal(|ui| {
            // Connected devices card
            self.render_stat_card(ui, "Connected Devices", &self.devices.len().to_string(), "💾", Color32::BLUE);
            
            ui.add_space(10.0);
            
            // Total connections card
            self.render_stat_card(ui, "Total Connections", &self.total_connections.to_string(), "🔌", Color32::GREEN);
            
            ui.add_space(10.0);
            
            // Total disconnections card
            self.render_stat_card(ui, "Total Disconnections", &self.total_disconnections.to_string(), "🔌", Color32::RED);
            
            ui.add_space(10.0);
            
            // Session uptime card
            let uptime = Utc::now().signed_duration_since(self.session_start);
            let uptime_str = format!("{}h {}m", uptime.num_hours(), uptime.num_minutes() % 60);
            self.render_stat_card(ui, "Session Uptime", &uptime_str, "⏱️", Color32::YELLOW);
        });
        
        ui.add_space(20.0);
        
        // Recent activity
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.heading("Recent Devices");
                ui.add_space(5.0);
                
                ScrollArea::vertical()
                    .max_height(200.0)
                    .show(ui, |ui| {
                        let mut devices: Vec<_> = self.devices.values().collect();
                        devices.sort_by(|a, b| b.device.timestamp.cmp(&a.device.timestamp));
                        
                        for (i, animated_device) in devices.iter().take(5).enumerate() {
                            let device = &animated_device.device;
                            let alpha = if self.show_animations {
                                self.get_fade_alpha(animated_device)
                            } else {
                                1.0
                            };
                            
                            let color = self.get_device_color(device);
                            let color_with_alpha = Color32::from_rgba_unmultiplied(
                                color.r(), color.g(), color.b(), (255.0 * alpha) as u8
                            );
                            
                            ui.horizontal(|ui| {
                                ui.colored_label(color_with_alpha, "●");
                                ui.label(device.product.as_deref().unwrap_or("Unknown Device"));
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    ui.small(format!("{:04X}:{:04X}", device.vendor_id, device.product_id));
                                });
                            });
                            
                            if i < 4 {
                                ui.separator();
                            }
                        }
                    });
            });
            
            ui.separator();
            
            ui.vertical(|ui| {
                ui.heading("Quick Actions");
                ui.add_space(5.0);
                
                if ui.button("🔍 Refresh Devices").clicked() {
                    // Trigger refresh
                }
                
                ui.add_space(5.0);
                
                let monitor_text = if self.monitoring_active {
                    "⏸️ Stop Monitoring"
                } else {
                    "▶️ Start Monitoring"
                };
                
                if ui.button(monitor_text).clicked() {
                    self.monitoring_active = !self.monitoring_active;
                }
                
                ui.add_space(5.0);
                
                if ui.button("📊 View Analytics").clicked() {
                    self.current_tab = Tab::Analytics;
                }
                
                ui.add_space(5.0);
                
                if ui.button("⚙️ Settings").clicked() {
                    self.show_settings = true;
                }
            });
        });
        
        ui.add_space(20.0);
        
        // Mini activity chart
        ui.heading("Activity Overview");
        ui.add_space(5.0);
        
        if !self.activity_data.is_empty() {
            let points: PlotPoints = self.activity_data
                .iter()
                .enumerate()
                .map(|(i, data)| [i as f64, data.device_count])
                .collect();
            
            Plot::new("dashboard_activity")
                .height(150.0)
                .show_axes([false, false])
                .show(ui, |plot_ui| {
                    plot_ui.line(
                        Line::new(points)
                            .color(Color32::BLUE)
                            .width(2.0)
                    );
                });
        } else {
            ui.label("No activity data yet...");
        }
    }
    
    pub fn render_devices_tab(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading("USB Devices");
            
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("🔄 Refresh").clicked() {
                    // Trigger device refresh
                }
                
                ui.separator();
                
                // View options
                ui.checkbox(&mut self.compact_view, "Compact");
                ui.checkbox(&mut self.show_animations, "Animations");
            });
        });
        
        ui.add_space(10.0);
        
        // Filters
        ui.horizontal(|ui| {
            ui.label("Search:");
            ui.text_edit_singleline(&mut self.search_filter);
            
            ui.separator();
            
            ui.label("Vendor:");
            ui.text_edit_singleline(&mut self.vendor_filter);
            
            ui.separator();
            
            if ui.button("Clear Filters").clicked() {
                self.search_filter.clear();
                self.vendor_filter.clear();
                self.device_class_filter = None;
            }
        });
        
        ui.add_space(10.0);
        
        // Device table
        self.render_device_table(ui);
    }
    
    pub fn render_device_table(&mut self, ui: &mut egui::Ui) {
        let text_height = egui::TextStyle::Body.resolve(ui.style()).size;
        let row_height = if self.compact_view { text_height + 4.0 } else { text_height + 8.0 };
        
        TableBuilder::new(ui)
            .striped(true)
            .resizable(true)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .column(Column::exact(30.0)) // Status indicator
            .column(Column::initial(120.0).range(80.0..=200.0)) // Manufacturer
            .column(Column::initial(150.0).range(100.0..=250.0)) // Product
            .column(Column::exact(80.0)) // VID:PID
            .column(Column::exact(60.0)) // Bus
            .column(Column::exact(80.0)) // Address
            .column(Column::exact(60.0)) // Class
            .column(Column::initial(100.0).range(80.0..=150.0)) // Serial
            .column(Column::remainder()) // Timestamp
            .header(20.0, |mut header| {
                header.col(|ui| { ui.strong(""); });
                header.col(|ui| { ui.strong("Manufacturer"); });
                header.col(|ui| { ui.strong("Product"); });
                header.col(|ui| { ui.strong("VID:PID"); });
                header.col(|ui| { ui.strong("Bus"); });
                header.col(|ui| { ui.strong("Addr"); });
                header.col(|ui| { ui.strong("Class"); });
                header.col(|ui| { ui.strong("Serial"); });
                header.col(|ui| { ui.strong("Connected"); });
            })
            .body(|mut body| {
                let mut devices: Vec<_> = self.devices.values().collect();
                devices.sort_by(|a, b| {
                    a.device.manufacturer.as_deref().unwrap_or("Unknown")
                        .cmp(b.device.manufacturer.as_deref().unwrap_or("Unknown"))
                });
                
                for animated_device in devices {
                    let device = &animated_device.device;
                    
                    // Apply filters
                    if !self.search_filter.is_empty() {
                        let search_lower = self.search_filter.to_lowercase();
                        let matches = device.manufacturer.as_deref().unwrap_or("").to_lowercase().contains(&search_lower)
                            || device.product.as_deref().unwrap_or("").to_lowercase().contains(&search_lower)
                            || device.serial_number.as_deref().unwrap_or("").to_lowercase().contains(&search_lower);
                        
                        if !matches {
                            continue;
                        }
                    }
                    
                    if !self.vendor_filter.is_empty() {
                        let vendor_lower = self.vendor_filter.to_lowercase();
                        if !device.manufacturer.as_deref().unwrap_or("").to_lowercase().contains(&vendor_lower) {
                            continue;
                        }
                    }
                    
                    let alpha = if self.show_animations {
                        self.get_fade_alpha(animated_device)
                    } else {
                        1.0
                    };
                    
                    body.row(row_height, |mut row| {
                        let color = self.get_device_color(device);
                        let color_with_alpha = Color32::from_rgba_unmultiplied(
                            color.r(), color.g(), color.b(), (255.0 * alpha) as u8
                        );
                        
                        // Status indicator
                        row.col(|ui| {
                            if animated_device.highlight {
                                let highlight_alpha = if let Some(start) = animated_device.highlight_start {
                                    let elapsed = Instant::now().duration_since(start).as_secs_f32();
                                    (1.0 - elapsed / 2.0).max(0.0)
                                } else {
                                    0.0
                                };
                                
                                let highlight_color = Color32::from_rgba_unmultiplied(
                                    255, 255, 0, (100.0 * highlight_alpha) as u8
                                );
                                
                                ui.painter().circle_filled(
                                    ui.available_rect_before_wrap().center(),
                                    8.0 + highlight_alpha * 4.0,
                                    highlight_color,
                                );
                            }
                            
                            ui.colored_label(color_with_alpha, "●");
                        });
                        
                        // Device info
                        row.col(|ui| {
                            ui.label(device.manufacturer.as_deref().unwrap_or("Unknown"));
                        });
                        
                        row.col(|ui| {
                            ui.label(device.product.as_deref().unwrap_or("Unknown"));
                        });
                        
                        row.col(|ui| {
                            ui.monospace(format!("{:04X}:{:04X}", device.vendor_id, device.product_id));
                        });
                        
                        row.col(|ui| {
                            ui.label(device.bus_number.to_string());
                        });
                        
                        row.col(|ui| {
                            ui.label(device.device_address.to_string());
                        });
                        
                        row.col(|ui| {
                            ui.monospace(format!("{:02X}", device.device_class));
                        });
                        
                        row.col(|ui| {
                            ui.small(device.serial_number.as_deref().unwrap_or("-"));
                        });
                        
                        row.col(|ui| {
                            ui.small(device.timestamp.format("%H:%M:%S").to_string());
                        });
                    });
                }
            });
    }
    
    pub fn render_monitoring_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Real-time Monitoring");
        ui.add_space(10.0);
        
        // Monitoring controls
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
                let pulse_color = Color32::from_rgb(
                    (255.0 * self.pulse_animation) as u8,
                    100,
                    100,
                );
                ui.colored_label(pulse_color, "● ACTIVE");
            } else {
                ui.colored_label(Color32::GRAY, "● INACTIVE");
            }
        });
        
        ui.add_space(20.0);
        
        // Event log
        ui.heading("Event Log");
        ui.add_space(5.0);
        
        ScrollArea::vertical()
            .max_height(300.0)
            .stick_to_bottom(true)
            .show(ui, |ui| {
                // Show recent device changes
                let mut events: Vec<_> = self.devices.values()
                    .filter(|d| d.highlight || d.fade_in || d.fade_out)
                    .collect();
                events.sort_by(|a, b| b.device.timestamp.cmp(&a.device.timestamp));
                
                for animated_device in events.iter().take(20) {
                    let device = &animated_device.device;
                    let event_type = if animated_device.fade_in {
                        ("🔌 Connected", Color32::GREEN)
                    } else if animated_device.fade_out {
                        ("🔌 Disconnected", Color32::RED)
                    } else {
                        ("🔄 Updated", Color32::BLUE)
                    };
                    
                    ui.horizontal(|ui| {
                        ui.colored_label(event_type.1, event_type.0);
                        ui.label(device.product.as_deref().unwrap_or("Unknown Device"));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.small(device.timestamp.format("%H:%M:%S").to_string());
                        });
                    });
                    ui.separator();
                }
                
                if events.is_empty() {
                    ui.label("No recent events...");
                }
            });
    }
    
    pub fn render_analytics_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Analytics");
        ui.add_space(10.0);
        
        if !self.activity_data.is_empty() {
            // Device count over time
            ui.heading("Device Count Over Time");
            let device_points: PlotPoints = self.activity_data
                .iter()
                .enumerate()
                .map(|(i, data)| [i as f64, data.device_count])
                .collect();
            
            Plot::new("device_count_plot")
                .height(200.0)
                .show(ui, |plot_ui| {
                    plot_ui.line(
                        Line::new(device_points)
                            .color(Color32::BLUE)
                            .width(2.0)
                            .name("Device Count")
                    );
                });
            
            ui.add_space(20.0);
            
            // Connection events over time
            ui.heading("Connection Events Over Time");
            let event_points: PlotPoints = self.activity_data
                .iter()
                .enumerate()
                .map(|(i, data)| [i as f64, data.connection_events])
                .collect();
            
            Plot::new("events_plot")
                .height(200.0)
                .show(ui, |plot_ui| {
                    plot_ui.line(
                        Line::new(event_points)
                            .color(Color32::RED)
                            .width(2.0)
                            .name("Total Events")
                    );
                });
        } else {
            ui.label("No analytics data available yet. Start monitoring to collect data.");
        }
    }
    
    pub fn render_settings_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Settings");
        ui.add_space(10.0);
        
        ui.checkbox(&mut self.dark_mode, "Dark Mode");
        ui.checkbox(&mut self.show_animations, "Enable Animations");
        ui.checkbox(&mut self.compact_view, "Compact View");
        ui.checkbox(&mut self.show_system_tray, "System Tray Icon");
        
        ui.add_space(20.0);
        
        if ui.button("💾 Save Settings").clicked() {
            // Save settings to config
        }
        
        if ui.button("🔄 Reset to Defaults").clicked() {
            self.dark_mode = true;
            self.show_animations = true;
            self.compact_view = false;
            self.show_system_tray = true;
        }
    }
    
    pub fn render_settings_dialog(&mut self, ctx: &egui::Context) {
        egui::Window::new("Settings")
            .resizable(false)
            .collapsible(false)
            .show(ctx, |ui| {
                self.render_settings_tab(ui);
                
                ui.add_space(10.0);
                ui.separator();
                ui.add_space(10.0);
                
                ui.horizontal(|ui| {
                    if ui.button("Close").clicked() {
                        self.show_settings = false;
                    }
                    
                    if ui.button("About").clicked() {
                        self.show_about = true;
                    }
                });
            });
    }
    
    pub fn render_about_dialog(&mut self, ctx: &egui::Context) {
        egui::Window::new("About IronWatch")
            .resizable(false)
            .collapsible(false)
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.heading("🛡️ IronWatch");
                    ui.label("Version 1.0.0");
                    ui.add_space(10.0);
                    ui.label("USB Device Input Monitor");
                    ui.label("by KnivInstitute");
                    ui.add_space(10.0);
                    ui.small("Built with Rust + egui");
                });
                
                ui.add_space(10.0);
                ui.separator();
                ui.add_space(10.0);
                
                if ui.button("Close").clicked() {
                    self.show_about = false;
                }
            });
    }
    
    fn render_stat_card(&self, ui: &mut egui::Ui, title: &str, value: &str, icon: &str, color: Color32) {
        egui::Frame::none()
            .fill(color.gamma_multiply(0.1))
            .rounding(8.0)
            .inner_margin(10.0)
            .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.colored_label(color, icon);
                    ui.heading(value);
                    ui.small(title);
                });
            });
    }
}