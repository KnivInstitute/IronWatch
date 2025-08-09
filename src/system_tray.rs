use tray_icon::{
    menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem},
    TrayIcon, TrayIconBuilder, TrayIconEvent,
};
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop, EventLoopBuilder},
    window::WindowBuilder,
};
use std::sync::mpsc;
use anyhow::Result;

pub enum TrayMessage {
    Show,
    Hide,
    Quit,
    ToggleMonitoring,
    ShowSettings,
    ShowAbout,
}

pub struct SystemTray {
    _tray_icon: TrayIcon,
    event_receiver: mpsc::Receiver<TrayMessage>,
}

impl SystemTray {
    pub fn new() -> Result<(Self, mpsc::Sender<TrayMessage>)> {
        let (sender, receiver) = mpsc::channel();
        
        // Create tray menu
        let show_item = MenuItem::new("Show IronWatch", true, None);
        let hide_item = MenuItem::new("Hide IronWatch", true, None);
        let separator1 = PredefinedMenuItem::separator();
        let monitoring_item = MenuItem::new("Toggle Monitoring", true, None);
        let separator2 = PredefinedMenuItem::separator();
        let settings_item = MenuItem::new("Settings", true, None);
        let about_item = MenuItem::new("About", true, None);
        let separator3 = PredefinedMenuItem::separator();
        let quit_item = MenuItem::new("Quit", true, None);
        
        let tray_menu = Menu::new();
        tray_menu.append(&show_item)?;
        tray_menu.append(&hide_item)?;
        tray_menu.append(&separator1)?;
        tray_menu.append(&monitoring_item)?;
        tray_menu.append(&separator2)?;
        tray_menu.append(&settings_item)?;
        tray_menu.append(&about_item)?;
        tray_menu.append(&separator3)?;
        tray_menu.append(&quit_item)?;
        
        // Create tray icon
        let tray_icon = TrayIconBuilder::new()
            .with_menu(Box::new(tray_menu))
            .with_tooltip("IronWatch - USB Device Monitor")
            .with_icon(Self::create_tray_icon())
            .build()?;
        
        // Store menu item IDs for comparison
        let show_id = show_item.id().clone();
        let hide_id = hide_item.id().clone();
        let monitoring_id = monitoring_item.id().clone();
        let settings_id = settings_item.id().clone();
        let about_id = about_item.id().clone();
        let quit_id = quit_item.id().clone();
        
        // Handle menu events
        let sender_clone = sender.clone();
        MenuEvent::set_event_handler(Some(move |event: tray_icon::menu::MenuEvent| {
            match event.id {
                id if id == show_id => {
                    let _ = sender_clone.send(TrayMessage::Show);
                }
                id if id == hide_id => {
                    let _ = sender_clone.send(TrayMessage::Hide);
                }
                id if id == monitoring_id => {
                    let _ = sender_clone.send(TrayMessage::ToggleMonitoring);
                }
                id if id == settings_id => {
                    let _ = sender_clone.send(TrayMessage::ShowSettings);
                }
                id if id == about_id => {
                    let _ = sender_clone.send(TrayMessage::ShowAbout);
                }
                id if id == quit_id => {
                    let _ = sender_clone.send(TrayMessage::Quit);
                }
                _ => {}
            }
        }));
        
        // Handle tray icon events
        let sender_clone = sender.clone();
        TrayIconEvent::set_event_handler(Some(move |event| {
            match event {
                TrayIconEvent::Click { .. } => {
                    let _ = sender_clone.send(TrayMessage::Show);
                }
                _ => {}
            }
        }));
        
        Ok((
            Self {
                _tray_icon: tray_icon,
                event_receiver: receiver,
            },
            sender,
        ))
    }
    
    pub fn try_recv(&self) -> Option<TrayMessage> {
        self.event_receiver.try_recv().ok()
    }
    
    fn create_tray_icon() -> tray_icon::Icon {
        // Create a simple 16x16 icon for the system tray
        let size = 16;
        let mut rgba = Vec::with_capacity(size * size * 4);
        
        for y in 0..size {
            for x in 0..size {
                let is_border = x == 0 || y == 0 || x == size - 1 || y == size - 1;
                let is_center = (x > 6 && x < 10) && (y > 6 && y < 10);
                
                if is_border {
                    rgba.extend_from_slice(&[100, 150, 255, 255]); // Blue border
                } else if is_center {
                    rgba.extend_from_slice(&[255, 255, 255, 255]); // White center
                } else {
                    rgba.extend_from_slice(&[50, 100, 200, 200]); // Semi-transparent blue
                }
            }
        }
        
        tray_icon::Icon::from_rgba(rgba, size as u32, size as u32)
            .expect("Failed to create tray icon")
    }
    
    pub fn update_icon(&self, monitoring: bool) -> Result<()> {
        // Update icon based on monitoring state
        let icon = if monitoring {
            Self::create_monitoring_icon()
        } else {
            Self::create_tray_icon()
        };
        
        self._tray_icon.set_icon(Some(icon))?;
        Ok(())
    }
    
    fn create_monitoring_icon() -> tray_icon::Icon {
        // Create a pulsing/active icon when monitoring
        let size = 16;
        let mut rgba = Vec::with_capacity(size * size * 4);
        
        for y in 0..size {
            for x in 0..size {
                let is_border = x == 0 || y == 0 || x == size - 1 || y == size - 1;
                let is_center = (x > 6 && x < 10) && (y > 6 && y < 10);
                
                if is_border {
                    rgba.extend_from_slice(&[255, 100, 100, 255]); // Red border when monitoring
                } else if is_center {
                    rgba.extend_from_slice(&[255, 255, 100, 255]); // Yellow center
                } else {
                    rgba.extend_from_slice(&[200, 50, 50, 200]); // Semi-transparent red
                }
            }
        }
        
        tray_icon::Icon::from_rgba(rgba, size as u32, size as u32)
            .expect("Failed to create monitoring icon")
    }
    
    pub fn show_notification(&self, title: &str, message: &str) -> Result<()> {
        #[cfg(target_os = "windows")]
        {
            use notify_rust::Notification;
            Notification::new()
                .summary(title)
                .body(message)
                .icon("usb")
                .timeout(3000)
                .show()?;
        }
        
        #[cfg(not(target_os = "windows"))]
        {
            use notify_rust::Notification;
            Notification::new()
                .summary(title)
                .body(message)
                .timeout(3000)
                .show()?;
        }
        
        Ok(())
    }
}