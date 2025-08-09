mod usb_monitor;
mod config;
mod error;
mod communication;
mod monitoring_service;

#[cfg(feature = "gui")]
mod gui_simple;
#[cfg(feature = "gui")]
mod system_tray;

#[cfg(feature = "cli")]
mod cli;
#[cfg(feature = "cli")]
mod output;

use anyhow::{Result, Context};
use env_logger;
use log::{info, error, debug, warn};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::signal;

#[cfg(feature = "gui")]
use eframe::egui;

#[cfg(feature = "cli")]
use {
    cli::{build_cli, parse_args, print_banner, CliConfig},
    usb_monitor::{UsbMonitor, UsbDeviceChange},
    config::ConfigManager,
    output::OutputManager,
    std::sync::Arc,
    tokio::sync::Mutex,
    tokio::signal,
};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    init_logging("info")?;
    
    // Setup graceful shutdown
    let shutdown_flag = Arc::new(AtomicBool::new(false));
    let shutdown_flag_clone = shutdown_flag.clone();
    
    // Handle Ctrl+C gracefully
    tokio::spawn(async move {
        if let Err(e) = signal::ctrl_c().await {
            error!("Failed to listen for shutdown signal: {}", e);
            return;
        }
        info!("Shutdown signal received, initiating graceful shutdown...");
        shutdown_flag_clone.store(true, Ordering::Relaxed);
    });

    let result = {
        #[cfg(feature = "gui")]
        {
            // Launch GUI application
            launch_gui_with_shutdown(shutdown_flag.clone()).await
        }

        #[cfg(all(feature = "cli", not(feature = "gui")))]
        {
            // Launch CLI application
            launch_cli_with_shutdown(shutdown_flag.clone()).await
        }

        #[cfg(not(any(feature = "gui", feature = "cli")))]
        {
            eprintln!("No interface enabled. Please enable either 'gui' or 'cli' feature.");
            std::process::exit(1);
        }
    };
    
    // Cleanup
    info!("Application shutting down...");
    result
}

#[cfg(feature = "gui")]
async fn launch_gui_with_shutdown(shutdown_flag: Arc<AtomicBool>) -> Result<()> {
    use communication::CommunicationHub;
    use monitoring_service::start_monitoring_service_with_recovery;
    
    info!("Starting IronWatch GUI...");
    
    // Create communication hub
    let (communication_hub, communication_receiver) = CommunicationHub::new();
    
    // Start monitoring service in background
    let monitoring_handle = start_monitoring_service_with_recovery(communication_receiver, 3)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to start monitoring service: {}", e))?;
    
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_min_inner_size([800.0, 600.0])
            .with_icon(load_icon())
            .with_title("IronWatch - USB Device Monitor"),
        centered: true,
        follow_system_theme: false,
        default_theme: eframe::Theme::Dark,
        run_and_return: false,
        ..Default::default()
    };

    // Store communication hub for shutdown
    let communication_hub_shutdown = communication_hub.clone();
    
    let result = eframe::run_native(
        "IronWatch",
        options,
        Box::new(move |cc| Box::new(gui_simple::IronWatchGui::new(cc, communication_hub))),
    ).map_err(|e| anyhow::anyhow!("Failed to run GUI: {}", e));
    
    // GUI has closed, shutdown the monitoring service
    info!("GUI closed, shutting down monitoring service...");
    let _ = communication_hub_shutdown.shutdown();
    
    // Wait for monitoring service to finish
    let _ = monitoring_handle.await;
    
    result
}

#[cfg(feature = "gui")]
fn load_icon() -> egui::IconData {
    // let icon_data = include_bytes!("../assets/icon.png"); // Uncomment for actual icon file
    
    // For now, create a simple colored square as fallback
    let (icon_rgba, icon_width, icon_height) = {
        let size = 32;
        let mut rgba = Vec::with_capacity(size * size * 4);
        
        for y in 0..size {
            for x in 0..size {
                let is_border = x == 0 || y == 0 || x == size - 1 || y == size - 1;
                let is_inner = (x > 8 && x < size - 8) && (y > 8 && y < size - 8);
                
                if is_border {
                    rgba.extend_from_slice(&[100, 150, 255, 255]); // Blue border
                } else if is_inner {
                    rgba.extend_from_slice(&[150, 200, 255, 255]); // Light blue center
                } else {
                    rgba.extend_from_slice(&[50, 100, 200, 255]); // Dark blue
                }
            }
        }
        
        (rgba, size as u32, size as u32)
    };

    egui::IconData {
        rgba: icon_rgba,
        width: icon_width,
        height: icon_height,
    }
}

#[cfg(feature = "cli")]
async fn launch_cli_with_shutdown(shutdown_flag: Arc<AtomicBool>) -> Result<()> {
    // Parse command line arguments
    let matches = build_cli().get_matches();
    let cli_config = parse_args(&matches)?;

    // Initialize logging
    init_logging(&cli_config.log_level)?;

    // Print banner
    print_banner();

    // Load configuration
    let config_manager = ConfigManager::new(cli_config.config_file.clone())
        .context("Failed to initialize configuration manager")?;
    
    // Validate configuration
    config_manager.validate()
        .context("Configuration validation failed")?;

    // Handle subcommands with shutdown support
    match matches.subcommand() {
        Some(("monitor", _)) => {
            run_monitoring_mode_with_shutdown(cli_config, config_manager, shutdown_flag).await?;
        }
        Some(("list", _)) => {
            run_list_mode(cli_config, config_manager).await?;
        }
        Some(("config", sub_matches)) => {
            run_config_mode(sub_matches, config_manager).await?;
        }
        _ => {
            // Default behavior - show help
            println!("No subcommand provided. Use --help for usage information.");
            build_cli().print_help()?;
        }
    }

    Ok(())
}

/// Initialize logging based on configuration
fn init_logging(log_level: &str) -> Result<()> {
    let level = match log_level {
        "error" => log::LevelFilter::Error,
        "warn" => log::LevelFilter::Warn,
        "info" => log::LevelFilter::Info,
        "debug" => log::LevelFilter::Debug,
        "trace" => log::LevelFilter::Trace,
        _ => log::LevelFilter::Info,
    };

    env_logger::Builder::from_default_env()
        .filter_level(level)
        .format_timestamp_secs()
        .init();

    debug!("Logging initialized at level: {}", log_level);
    Ok(())
}

#[cfg(feature = "cli")]
async fn run_monitoring_mode_with_shutdown(cli_config: CliConfig, config_manager: ConfigManager, shutdown_flag: Arc<AtomicBool>) -> Result<()> {
    use usb_monitor::{UsbMonitor, UsbDeviceChange};
    use config::ConfigManager;
    use output::OutputManager;
    use std::sync::Arc;
    use tokio::sync::Mutex;
    use tokio::signal;
    info!("Starting USB device monitoring mode");

    // Create USB monitor
    let mut usb_monitor = UsbMonitor::new()
        .context("Failed to create USB monitor")?;

    // Set device filter if provided
    usb_monitor.set_filter(cli_config.device_filter.clone());

    // Create output manager
    let mut output_manager = OutputManager::new(
        cli_config.output_format,
        cli_config.output_file,
        config_manager.get_config().output.color_output,
        config_manager.get_config().output.include_metadata,
    ).context("Failed to create output manager")?;

    if cli_config.continuous {
        // Continuous monitoring mode
        info!("Running in continuous monitoring mode");
        
        // Get initial device count for status display
        let initial_devices = usb_monitor.get_connected_devices()
            .context("Failed to get initial device list")?;
        
        output_manager.display_monitoring_status(
            initial_devices.len(),
            cli_config.device_filter.as_deref(),
        )?;

        // Create shared output manager for the callback
        let output_manager_arc = Arc::new(Mutex::new(output_manager));
        let output_manager_clone = output_manager_arc.clone();

        // Start monitoring with callback
        let monitoring_task = tokio::spawn(async move {
            usb_monitor.start_monitoring(move |changes: Vec<UsbDeviceChange>| {
                let output_manager = output_manager_clone.clone();
                tokio::spawn(async move {
                    let mut manager = output_manager.lock().await;
                    if let Err(e) = manager.display_changes(&changes) {
                        error!("Failed to display changes: {}", e);
                    }
                });
                Ok(())
            }).await
        });

        // Handle Ctrl+C gracefully and check shutdown flag
        tokio::select! {
            result = monitoring_task => {
                match result {
                    Ok(Ok(_)) => info!("Monitoring completed successfully"),
                    Ok(Err(e)) => error!("Monitoring error: {}", e),
                    Err(e) => error!("Task error: {}", e),
                }
            }
            _ = signal::ctrl_c() => {
                info!("Received interrupt signal, shutting down gracefully...");
                shutdown_flag.store(true, Ordering::Relaxed);
            }
        }
        
        // Wait a bit for cleanup
        while !shutdown_flag.load(Ordering::Relaxed) {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
    } else {
        // Single scan mode
        info!("Running single device scan");
        
        let devices = usb_monitor.get_connected_devices()
            .context("Failed to get device list")?;
        
        output_manager.display_devices(&devices)
            .context("Failed to display devices")?;
        
        info!("Found {} USB devices", devices.len());
    }

    Ok(())
}

#[cfg(feature = "cli")]
async fn run_list_mode(cli_config: CliConfig, config_manager: ConfigManager) -> Result<()> {
    use usb_monitor::UsbMonitor;
    use config::ConfigManager;
    use output::OutputManager;
    info!("Listing USB devices");

    // Create USB monitor
    let usb_monitor = UsbMonitor::new()
        .context("Failed to create USB monitor")?;

    // Get connected devices
    let devices = usb_monitor.get_connected_devices()
        .context("Failed to get device list")?;

    // Create output manager
    let mut output_manager = OutputManager::new(
        cli_config.output_format,
        cli_config.output_file,
        config_manager.get_config().output.color_output,
        config_manager.get_config().output.include_metadata,
    ).context("Failed to create output manager")?;

    // Display devices
    output_manager.display_devices(&devices)
        .context("Failed to display devices")?;

    info!("Listed {} USB devices", devices.len());
    Ok(())
}

#[cfg(feature = "cli")]
async fn run_config_mode(
    matches: &clap::ArgMatches,
    mut config_manager: ConfigManager,
) -> Result<()> {
    use config::ConfigManager;
    match matches.subcommand() {
        Some(("show", _)) => {
            // Display current configuration
            let config_json = serde_json::to_string_pretty(config_manager.get_config())
                .context("Failed to serialize configuration")?;
            
            println!("Current configuration:");
            println!("{}", config_json);
            println!("\nConfiguration file: {}", config_manager.get_config_path().display());
        }
        Some(("set", sub_matches)) => {
            // Set configuration value
            let key = sub_matches.get_one::<String>("key")
                .context("Key argument is required")?;
            let value = sub_matches.get_one::<String>("value")
                .context("Value argument is required")?;
            
            config_manager.set_value(key, value)
                .with_context(|| format!("Failed to set configuration value: {} = {}", key, value))?;
            
            // Validate and save
            config_manager.validate()
                .context("Configuration validation failed after update")?;
            
            config_manager.save()
                .context("Failed to save configuration")?;
            
            println!("Configuration updated: {} = {}", key, value);
        }
        _ => {
            warn!("Unknown config subcommand");
            return Ok(());
        }
    }

    Ok(())
}
