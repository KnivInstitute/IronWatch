mod cli;
mod usb_monitor;
mod config;
mod output;

use cli::{build_cli, parse_args, print_banner, CliConfig};
use usb_monitor::{UsbMonitor, UsbDeviceChange};
use config::ConfigManager;
use output::OutputManager;

use anyhow::{Result, Context};
use env_logger;
use log::{info, error, debug, warn};
use tokio;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::signal;

#[tokio::main]
async fn main() -> Result<()> {
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

    // Handle subcommands
    match matches.subcommand() {
        Some(("monitor", _)) => {
            run_monitoring_mode(cli_config, config_manager).await?;
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

/// Run USB device monitoring mode
async fn run_monitoring_mode(cli_config: CliConfig, config_manager: ConfigManager) -> Result<()> {
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

        // Handle Ctrl+C gracefully
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
            }
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

/// Run device listing mode
async fn run_list_mode(cli_config: CliConfig, config_manager: ConfigManager) -> Result<()> {
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

/// Run configuration management mode
async fn run_config_mode(
    matches: &clap::ArgMatches,
    mut config_manager: ConfigManager,
) -> Result<()> {
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
