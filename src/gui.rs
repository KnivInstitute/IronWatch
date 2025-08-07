use crate::usb_monitor::{UsbMonitor, UsbDeviceInfo, UsbDeviceChange};
use crate::config::ConfigManager;

use eframe::egui::{self, *};
use egui_extras::{Column, TableBuilder};
use egui_plot::{Line, Plot, PlotPoints};
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc;
use chrono::{DateTime, Utc};
use notify_rust::Notification;
use palette::{Hsv, IntoColor, Srgb};

const ANIMATION_DURATION: f32 = 0.8;
const MAX_ACTIVITY_POINTS: usize = 100;
const REFRESH_RATE: Duration = Duration::from_millis(16); // ~60 FPS

#[derive(Debug, Clone)]
pub enum GuiMessage {
    DeviceConnected(UsbDeviceInfo),
    DeviceDisconnected(UsbDeviceInfo),
    DeviceReconnected(UsbDeviceInfo),
    DeviceListUpdated(Vec<UsbDeviceInfo>),
    MonitoringStarted,
    MonitoringStopped,
    Error(String),
}

#[derive(Debug, Clone)]
struct AnimatedDevice {
    device: UsbDeviceInfo,
    animation_start: Instant,
    fade_in: bool,
    fade_out: bool,
    highlight: bool,
    highlight_start: Option<Instant>,
}

#[derive(Debug)]
struct ActivityData {
    timestamp: f64,
    device_count: f64,
    connection_events: f64,
}

pub struct IronWatchGui {
    // Core state
    devices: HashMap<String, AnimatedDevice>,
    config_manager: Arc<Mutex<ConfigManager>>,
    usb_monitor: Arc<Mutex<Option<UsbMonitor>>>,
    
    // UI state
    current_tab: Tab,
    monitoring_active: bool,
    show_settings: bool,
    show_about: bool,
    
    // Animation state
    last_frame_time: Instant,
    global_animation_time: f32,
    pulse_animation: f32,
    
    // Filtering and search
    search_filter: String,
    device_class_filter: Option<u8>,
    vendor_filter: String,
    
    // Activity tracking
    activity_data: VecDeque<ActivityData>,
    total_connections: u64,
    total_disconnections: u64,
    session_start: DateTime<Utc>,
    
    // Communication
    message_receiver: Option<mpsc::UnboundedReceiver<GuiMessage>>,
    message_sender: mpsc::UnboundedSender<GuiMessage>,
    
    // Visual settings
    dark_mode: bool,
    show_animations: bool,
    compact_view: bool,
    show_system_tray: bool,
    
    // Performance
    fps_counter: f32,
    frame_times: VecDeque<f32>,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Tab {
    Dashboard,
    Devices,
    Monitoring,
    Analytics,
    Settings,
}

impl Default for IronWatchGui {
    fn default() -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();
        
        Self {
            devices: HashMap::new(),
            config_manager: Arc::new(Mutex::new(
                ConfigManager::new(None).unwrap_or_else(|_| {
                    log::warn!("Failed to load config, using defaults");
                    ConfigManager::new(None).unwrap()
                })
            )),
            usb_monitor: Arc::new(Mutex::new(None)),
            
            current_tab: Tab::Dashboard,
            monitoring_active: false,
            show_settings: false,
            show_about: false,
            
            last_frame_time: Instant::now(),
            global_animation_time: 0.0,
            pulse_animation: 0.0,
            
            search_filter: String::new(),
            device_class_filter: None,
            vendor_filter: String::new(),
            
            activity_data: VecDeque::with_capacity(MAX_ACTIVITY_POINTS),
            total_connections: 0,
            total_disconnections: 0,
            session_start: Utc::now(),
            
            message_receiver: Some(receiver),
            message_sender: sender,
            
            dark_mode: true,
            show_animations: true,
            compact_view: false,
            show_system_tray: true,
            
            fps_counter: 0.0,
            frame_times: VecDeque::with_capacity(60),
        }
    }
}

impl IronWatchGui {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Configure egui style
        let mut style = (*cc.egui_ctx.style()).clone();
        style.visuals.dark_mode = true;
        style.visuals.window_rounding = Rounding::same(10.0);
        style.visuals.button_rounding = Rounding::same(8.0);
        style.visuals.menu_rounding = Rounding::same(8.0);
        cc.egui_ctx.set_style(style);
        
        let mut app = Self::default();
        
        // Initialize USB monitoring
        app.initialize_usb_monitoring();
        
        // Start background tasks
        app.start_background_tasks(cc.egui_ctx.clone());
        
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
                let _ = self.message_sender.send(GuiMessage::Error(
                    format!("Failed to initialize USB monitoring: {}", e)
                ));
            }
        }
    }
    
    fn start_background_tasks(&self, ctx: egui::Context) {
        let sender = self.message_sender.clone();
        let usb_monitor = Arc::clone(&self.usb_monitor);
        
        // USB monitoring task
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(500));
            
            loop {
                interval.tick().await;
                
                if let Some(ref mut monitor) = *usb_monitor.lock().unwrap() {
                    match monitor.get_connected_devices() {
                        Ok(devices) => {
                            let _ = sender.send(GuiMessage::DeviceListUpdated(devices));
                        }
                        Err(e) => {
                            log::error!("Error getting devices: {}", e);
                        }
                    }
                }
                
                ctx.request_repaint();
            }
        });
        
        // Animation update task
        let ctx_clone = ctx.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(REFRESH_RATE);
            
            loop {
                interval.tick().await;
                ctx_clone.request_repaint();
            }
        });
    }
    
    fn update_animations(&mut self, dt: f32) {
        self.global_animation_time += dt;
        
        // Pulse animation for monitoring indicator
        self.pulse_animation = (self.global_animation_time * 2.0).sin() * 0.5 + 0.5;
        
        // Update device animations
        let now = Instant::now();
        for device in self.devices.values_mut() {
            if device.fade_in {
                let elapsed = now.duration_since(device.animation_start).as_secs_f32();
                if elapsed >= ANIMATION_DURATION {
                    device.fade_in = false;
                }
            }
            
            if device.fade_out {
                let elapsed = now.duration_since(device.animation_start).as_secs_f32();
                if elapsed >= ANIMATION_DURATION {
                    // Remove device after fade out
                    continue;
                }
            }
            
            if let Some(highlight_start) = device.highlight_start {
                let elapsed = now.duration_since(highlight_start).as_secs_f32();
                if elapsed >= 2.0 {
                    device.highlight = false;
                    device.highlight_start = None;
                }
            }
        }
        
        // Remove faded out devices
        self.devices.retain(|_, device| {
            if device.fade_out {
                let elapsed = now.duration_since(device.animation_start).as_secs_f32();
                elapsed < ANIMATION_DURATION
            } else {
                true
            }
        });
    }
    
    fn process_messages(&mut self) {
        if let Some(ref mut receiver) = self.message_receiver {
            while let Ok(message) = receiver.try_recv() {
                match message {
                    GuiMessage::DeviceListUpdated(devices) => {
                        self.update_device_list(devices);
                    }
                    GuiMessage::DeviceConnected(device) => {
                        self.handle_device_connected(device);
                    }
                    GuiMessage::DeviceDisconnected(device) => {
                        self.handle_device_disconnected(device);
                    }
                    GuiMessage::DeviceReconnected(device) => {
                        self.handle_device_reconnected(device);
                    }
                    GuiMessage::Error(error) => {
                        log::error!("GUI Error: {}", error);
                        // Could show error dialog here
                    }
                    _ => {}
                }
            }
        }
    }
    
    fn update_device_list(&mut self, devices: Vec<UsbDeviceInfo>) {
        let now = Instant::now();
        let mut new_device_keys = std::collections::HashSet::new();
        
        for device in devices {
            let key = format!("{}:{}:{}:{}", 
                device.vendor_id, device.product_id, 
                device.bus_number, device.device_address);
            
            new_device_keys.insert(key.clone());
            
            if !self.devices.contains_key(&key) {
                // New device - animate in
                let animated_device = AnimatedDevice {
                    device,
                    animation_start: now,
                    fade_in: true,
                    fade_out: false,
                    highlight: true,
                    highlight_start: Some(now),
                };
                
                self.devices.insert(key, animated_device);
                self.total_connections += 1;
                
                // Show notification
                if self.monitoring_active {
                    self.show_device_notification(&self.devices[&key].device, "connected");
                }
            } else {
                // Update existing device
                self.devices.get_mut(&key).unwrap().device = device;
            }
        }
        
        // Mark disconnected devices for fade out
        for (key, device) in self.devices.iter_mut() {
            if !new_device_keys.contains(key) && !device.fade_out {
                device.fade_out = true;
                device.animation_start = now;
                self.total_disconnections += 1;
                
                if self.monitoring_active {
                    self.show_device_notification(&device.device, "disconnected");
                }
            }
        }
        
        // Update activity data
        self.update_activity_data();
    }
    
    fn handle_device_connected(&mut self, device: UsbDeviceInfo) {
        self.show_device_notification(&device, "connected");
    }
    
    fn handle_device_disconnected(&mut self, device: UsbDeviceInfo) {
        self.show_device_notification(&device, "disconnected");
    }
    
    fn handle_device_reconnected(&mut self, device: UsbDeviceInfo) {
        self.show_device_notification(&device, "reconnected");
    }
    
    fn show_device_notification(&self, device: &UsbDeviceInfo, action: &str) {
        if !self.monitoring_active {
            return;
        }
        
        let device_name = device.product.as_deref()
            .or(device.manufacturer.as_deref())
            .unwrap_or("Unknown Device");
        
        let _ = Notification::new()
            .summary(&format!("IronWatch - Device {}", action))
            .body(&format!("{} ({:04X}:{:04X})", device_name, device.vendor_id, device.product_id))
            .icon("usb")
            .timeout(3000)
            .show();
    }
    
    fn update_activity_data(&mut self) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs_f64();
        
        let activity = ActivityData {
            timestamp: now,
            device_count: self.devices.len() as f64,
            connection_events: (self.total_connections + self.total_disconnections) as f64,
        };
        
        self.activity_data.push_back(activity);
        
        if self.activity_data.len() > MAX_ACTIVITY_POINTS {
            self.activity_data.pop_front();
        }
    }
    
    fn get_device_color(&self, device: &UsbDeviceInfo) -> Color32 {
        // Generate color based on vendor ID for consistency
        let hue = (device.vendor_id as f32 * 137.508) % 360.0; // Golden angle
        let hsv = Hsv::new(hue, 0.7, 0.9);
        let rgb: Srgb = hsv.into_color();
        
        Color32::from_rgb(
            (rgb.red * 255.0) as u8,
            (rgb.green * 255.0) as u8,
            (rgb.blue * 255.0) as u8,
        )
    }
    
    fn get_fade_alpha(&self, device: &AnimatedDevice) -> f32 {
        let now = Instant::now();
        
        if device.fade_in {
            let elapsed = now.duration_since(device.animation_start).as_secs_f32();
            let progress = (elapsed / ANIMATION_DURATION).clamp(0.0, 1.0);
            // Simple ease-out: 1 - (1-t)^2
            let ease_out = 1.0 - (1.0 - progress).powi(2);
            ease_out
        } else if device.fade_out {
            let elapsed = now.duration_since(device.animation_start).as_secs_f32();
            let progress = (elapsed / ANIMATION_DURATION).clamp(0.0, 1.0);
            // Simple ease-in: t^2
            1.0 - progress.powi(2)
        } else {
            1.0
        }
    }
}

impl eframe::App for IronWatchGui {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Update timing
        let now = Instant::now();
        let dt = now.duration_since(self.last_frame_time).as_secs_f32();
        self.last_frame_time = now;
        
        // Update FPS counter
        self.frame_times.push_back(dt);
        if self.frame_times.len() > 60 {
            self.frame_times.pop_front();
        }
        self.fps_counter = 1.0 / (self.frame_times.iter().sum::<f32>() / self.frame_times.len() as f32);
        
        // Process messages and update animations
        self.process_messages();
        if self.show_animations {
            self.update_animations(dt);
        }
        
        // Main UI
        self.render_top_panel(ctx);
        self.render_main_content(ctx);
        
        // Dialogs
        if self.show_settings {
            self.render_settings_dialog(ctx);
        }
        
        if self.show_about {
            self.render_about_dialog(ctx);
        }
    }
}