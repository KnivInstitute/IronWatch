use crate::usb_monitor::{UsbDeviceInfo, UsbDeviceChange, DeviceStatistics, DeviceAnalytics, SecurityEvent};
use crate::cli::OutputFormat;
use serde_json;
use std::fs::OpenOptions;
use std::io::{Write, BufWriter};
use std::path::PathBuf;
use anyhow::{Result, Context};
use crossterm::{
    style::Stylize,
    terminal::{Clear, ClearType},
    cursor::MoveTo,
    execute,
};
use std::io::stdout;
use chrono::Utc;

pub struct OutputManager {
    format: OutputFormat,
    output_file: Option<BufWriter<std::fs::File>>,
    use_colors: bool,
    include_metadata: bool,
}

impl OutputManager {
    /// Create a new output manager
    pub fn new(
        format: OutputFormat,
        output_file_path: Option<PathBuf>,
        use_colors: bool,
        include_metadata: bool,
    ) -> Result<Self> {
        let output_file = match output_file_path {
            Some(path) => {
                let file = OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&path)
                    .with_context(|| format!("Failed to open output file: {}", path.display()))?;
                Some(BufWriter::new(file))
            }
            None => None,
        };

        Ok(Self {
            format,
            output_file,
            use_colors,
            include_metadata,
        })
    }

    /// Display a list of USB devices
    pub fn display_devices(&mut self, devices: &[UsbDeviceInfo]) -> Result<()> {
        match self.format {
            OutputFormat::Json => self.output_json_devices(devices),
            OutputFormat::Table => self.output_table_devices(devices),
            OutputFormat::Csv => self.output_csv_devices(devices),
        }
    }

    /// Display USB device changes
    pub fn display_changes(&mut self, changes: &[UsbDeviceChange]) -> Result<()> {
        for change in changes {
            match self.format {
                OutputFormat::Json => self.output_json_change(change)?,
                OutputFormat::Table => self.output_table_change(change)?,
                OutputFormat::Csv => self.output_csv_change(change)?,
            }
        }
        
        if let Some(ref mut file) = self.output_file {
            file.flush().context("Failed to flush output file")?;
        }
        
        Ok(())
    }

    /// Export device history to a file
    pub fn export_device_history(
        &mut self,
        devices: &[UsbDeviceInfo],
        device_stats: &[(String, DeviceStatistics)],
        analytics: &DeviceAnalytics,
        security_events: &[SecurityEvent],
        export_path: &PathBuf,
    ) -> Result<()> {
        let timestamp = Utc::now();
        
        match self.format {
            OutputFormat::Json => self.export_json_history(devices, device_stats, analytics, security_events, export_path, timestamp),
            OutputFormat::Table => self.export_table_history(devices, device_stats, analytics, security_events, export_path, timestamp),
            OutputFormat::Csv => self.export_csv_history(devices, device_stats, analytics, security_events, export_path, timestamp),
        }
    }

    /// Export device history in JSON format
    fn export_json_history(
        &mut self,
        devices: &[UsbDeviceInfo],
        device_stats: &[(String, DeviceStatistics)],
        analytics: &DeviceAnalytics,
        security_events: &[SecurityEvent],
        export_path: &PathBuf,
        timestamp: chrono::DateTime<Utc>,
    ) -> Result<()> {
        let export_data = serde_json::json!({
            "export_timestamp": timestamp,
            "export_format": "json",
            "summary": {
                "total_devices": devices.len(),
                "total_connections": analytics.connection_frequency.iter().map(|(_, count)| count).sum::<u32>(),
                "unique_devices": analytics.unique_devices,
                "blocked_devices": analytics.blocked_devices,
                "security_violations": analytics.security_violations,
                "device_classes": analytics.device_class_distribution.len(),
                "vendors": analytics.vendor_distribution.len(),
            },
            "current_devices": devices,
            "device_statistics": device_stats,
            "analytics": {
                "device_class_distribution": analytics.device_class_distribution,
                "vendor_distribution": analytics.vendor_distribution,
                "connection_frequency": analytics.connection_frequency,
            },
            "security": {
                "total_blocked": analytics.blocked_devices,
                "total_violations": analytics.security_violations,
                "security_events": security_events,
            }
        });

        let json_string = serde_json::to_string_pretty(&export_data)
            .context("Failed to serialize export data to JSON")?;
        
        std::fs::write(export_path, json_string)
            .with_context(|| format!("Failed to write export file: {}", export_path.display()))?;
        
        Ok(())
    }

    /// Export device history in CSV format
    fn export_csv_history(
        &mut self,
        devices: &[UsbDeviceInfo],
        device_stats: &[(String, DeviceStatistics)],
        analytics: &DeviceAnalytics,
        security_events: &[SecurityEvent],
        export_path: &PathBuf,
        timestamp: chrono::DateTime<Utc>,
    ) -> Result<()> {
        let mut csv_content = String::new();
        
        // Header
        csv_content.push_str("Export Timestamp,Device Count,Total Connections,Unique Devices,Blocked Devices,Security Violations\n");
        csv_content.push_str(&format!("{},{},{},{},{},{}\n", 
            timestamp.format("%Y-%m-%d %H:%M:%S UTC"),
            devices.len(),
            analytics.connection_frequency.iter().map(|(_, count)| count).sum::<u32>(),
            analytics.unique_devices,
            analytics.blocked_devices,
            analytics.security_violations
        ));
        
        csv_content.push_str("\nCurrent Devices\n");
        csv_content.push_str("Bus,Vendor ID,Product ID,Manufacturer,Product,Serial,Class,Status,Timestamp\n");
        
        for device in devices {
            csv_content.push_str(&format!("{},{:04x},{:04x},{},{},{},{:02x},{:?},{}\n",
                device.bus_number,
                device.vendor_id,
                device.product_id,
                device.manufacturer.as_deref().unwrap_or("Unknown"),
                device.product.as_deref().unwrap_or("Unknown"),
                device.serial_number.as_deref().unwrap_or("Unknown"),
                device.device_class,
                device.connection_status,
                device.timestamp.format("%Y-%m-%d %H:%M:%S UTC")
            ));
        }
        
        csv_content.push_str("\nDevice Statistics\n");
        csv_content.push_str("Device Key,Total Connections,Total Disconnections,Total Blocked,First Seen,Last Seen,Connection Duration,Current Count\n");
        
        for (key, stats) in device_stats {
            csv_content.push_str(&format!("{},{},{},{},{},{},{},{}\n",
                key,
                stats.total_connections,
                stats.total_disconnections,
                stats.total_blocked,
                stats.first_seen.format("%Y-%m-%d %H:%M:%S UTC"),
                stats.last_seen.format("%Y-%m-%d %H:%M:%S UTC"),
                stats.connection_duration.as_secs(),
                stats.connection_count
            ));
        }
        
        csv_content.push_str("\nSecurity Events\n");
        csv_content.push_str("Timestamp,Event Type,Device Product,Vendor ID,Product ID,Reason,Action\n");
        
        for event in security_events {
            csv_content.push_str(&format!("{},{:?},{},{:04x},{:04x},{},{:?}\n",
                event.timestamp.format("%Y-%m-%d %H:%M:%S UTC"),
                event.event_type,
                event.device_info.product.as_deref().unwrap_or("Unknown"),
                event.device_info.vendor_id,
                event.device_info.product_id,
                event.reason,
                event.action_taken
            ));
        }
        
        csv_content.push_str("\nDevice Class Distribution\n");
        csv_content.push_str("Class ID,Count\n");
        
        for (class_id, count) in &analytics.device_class_distribution {
            csv_content.push_str(&format!("0x{:02x},{}\n", class_id, count));
        }
        
        csv_content.push_str("\nVendor Distribution\n");
        csv_content.push_str("Vendor ID,Count\n");
        
        for (vendor_id, count) in &analytics.vendor_distribution {
            csv_content.push_str(&format!("0x{:04x},{}\n", vendor_id, count));
        }
        
        csv_content.push_str("\nConnection Frequency (24h)\n");
        csv_content.push_str("Hour,Connections\n");
        
        for (timestamp, count) in &analytics.connection_frequency {
            csv_content.push_str(&format!("{},{}\n", 
                timestamp.format("%H:00"), 
                count
            ));
        }
        
        std::fs::write(export_path, csv_content)
            .with_context(|| format!("Failed to write export file: {}", export_path.display()))?;
        
        Ok(())
    }

    /// Export device history in table format (human-readable)
    fn export_table_history(
        &mut self,
        devices: &[UsbDeviceInfo],
        device_stats: &[(String, DeviceStatistics)],
        analytics: &DeviceAnalytics,
        security_events: &[SecurityEvent],
        export_path: &PathBuf,
        timestamp: chrono::DateTime<Utc>,
    ) -> Result<()> {
        let mut table_content = String::new();
        
        // Header
        table_content.push_str("IronWatch Device History Export\n");
        table_content.push_str("================================\n\n");
        table_content.push_str(&format!("Export Time: {}\n", timestamp.format("%Y-%m-%d %H:%M:%S UTC")));
        table_content.push_str(&format!("Total Devices: {}\n", devices.len()));
        table_content.push_str(&format!("Total Connections (24h): {}\n", 
            analytics.connection_frequency.iter().map(|(_, count)| count).sum::<u32>()));
        table_content.push_str(&format!("Unique Devices: {}\n", analytics.unique_devices));
        table_content.push_str(&format!("Blocked Devices: {}\n", analytics.blocked_devices));
        table_content.push_str(&format!("Security Violations: {}\n", analytics.security_violations));
        table_content.push_str(&format!("Device Classes: {}\n", analytics.device_class_distribution.len()));
        table_content.push_str(&format!("Vendors: {}\n", analytics.vendor_distribution.len()));
        table_content.push_str("\n");
        
        // Current Devices
        table_content.push_str("CURRENT DEVICES\n");
        table_content.push_str("===============\n");
        table_content.push_str("Bus  Vendor ID  Product ID  Manufacturer          Product               Class  Status\n");
        table_content.push_str("---- ---------- ----------- -------------------- -------------------- ------ --------\n");
        
        for device in devices {
            table_content.push_str(&format!("{:3}  {:04x}       {:04x}      {:<20} {:<20} {:02x}    {:?}\n",
                device.bus_number,
                device.vendor_id,
                device.product_id,
                device.manufacturer.as_deref().unwrap_or("Unknown").chars().take(20).collect::<String>(),
                device.product.as_deref().unwrap_or("Unknown").chars().take(20).collect::<String>(),
                device.device_class,
                device.connection_status
            ));
        }
        
        table_content.push_str("\nDEVICE STATISTICS\n");
        table_content.push_str("=================\n");
        table_content.push_str("Device Key                    Connections  Disconnections  Blocked  First Seen           Last Seen            Duration\n");
        table_content.push_str("---------------------------- ------------ --------------- -------- -------------------- -------------------- ---------\n");
        
        for (key, stats) in device_stats {
            let key_short = if key.len() > 28 { format!("{}...", &key[..25]) } else { key.clone() };
            table_content.push_str(&format!("{:<28} {:>12} {:>15} {:>8} {:>19} {:>19} {:>9}s\n",
                key_short,
                stats.total_connections,
                stats.total_disconnections,
                stats.total_blocked,
                stats.first_seen.format("%Y-%m-%d %H:%M"),
                stats.last_seen.format("%Y-%m-%d %H:%M"),
                stats.connection_duration.as_secs()
            ));
        }
        
        // Security Events
        table_content.push_str("\nSECURITY EVENTS\n");
        table_content.push_str("===============\n");
        table_content.push_str("Timestamp           Event Type      Device Product        Vendor ID  Product ID  Reason                    Action\n");
        table_content.push_str("------------------- --------------- -------------------- ---------- ----------- ------------------------- --------\n");
        
        for event in security_events {
            table_content.push_str(&format!("{}  {:<14} {:<20} {:04x}      {:04x}      {:<25} {:?}\n",
                event.timestamp.format("%Y-%m-%d %H:%M"),
                format!("{:?}", event.event_type),
                event.device_info.product.as_deref().unwrap_or("Unknown").chars().take(20).collect::<String>(),
                event.device_info.vendor_id,
                event.device_info.product_id,
                event.reason.chars().take(25).collect::<String>(),
                event.action_taken
            ));
        }
        
        // Analytics summary
        table_content.push_str("\nANALYTICS SUMMARY\n");
        table_content.push_str("=================\n");
        
        if !analytics.device_class_distribution.is_empty() {
            table_content.push_str("Device Class Distribution:\n");
            for (class_id, count) in &analytics.device_class_distribution {
                table_content.push_str(&format!("  0x{:02x}: {}\n", class_id, count));
            }
            table_content.push_str("\n");
        }
        
        if !analytics.vendor_distribution.is_empty() {
            table_content.push_str("Vendor Distribution:\n");
            for (vendor_id, count) in &analytics.vendor_distribution {
                table_content.push_str(&format!("  0x{:04x}: {}\n", vendor_id, count));
            }
            table_content.push_str("\n");
        }
        
        std::fs::write(export_path, table_content)
            .with_context(|| format!("Failed to write export file: {}", export_path.display()))?;
        
        Ok(())
    }

    /// Output devices in JSON format
    fn output_json_devices(&mut self, devices: &[UsbDeviceInfo]) -> Result<()> {
        let json = if self.include_metadata {
            serde_json::json!({
                "timestamp": Utc::now(),
                "device_count": devices.len(),
                "devices": devices
            })
        } else {
            serde_json::json!(devices)
        };

        let output = serde_json::to_string_pretty(&json)
            .context("Failed to serialize devices to JSON")?;
        
        self.write_output(&output)?;
        Ok(())
    }

    /// Output devices in table format
    fn output_table_devices(&mut self, devices: &[UsbDeviceInfo]) -> Result<()> {
        if devices.is_empty() {
            self.write_output("No USB devices found.\n")?;
            return Ok(());
        }

        // Header
        let header = if self.use_colors {
            format!(
                "{:<4} {:<8} {:<8} {:<25} {:<25} {:<15} {:<20}",
                "Bus".bold().blue(),
                "VID:PID".bold().blue(),
                "Address".bold().blue(),
                "Manufacturer".bold().blue(),
                "Product".bold().blue(),
                "Class".bold().blue(),
                "Timestamp".bold().blue()
            )
        } else {
            format!(
                "{:<4} {:<8} {:<8} {:<25} {:<25} {:<15} {:<20}",
                "Bus", "VID:PID", "Address", "Manufacturer", "Product", "Class", "Timestamp"
            )
        };
        
        self.write_output(&format!("{}\n", header))?;
        self.write_output(&format!("{}\n", "-".repeat(110)))?;

        // Device rows
        for device in devices {
            let manufacturer = device.manufacturer.as_deref().unwrap_or("Unknown");
            let product = device.product.as_deref().unwrap_or("Unknown");
            let timestamp = device.timestamp.format("%H:%M:%S").to_string();
            
            let row = format!(
                "{:<4} {:04X}:{:04X} {:<8} {:<25} {:<25} {:<15} {:<20}",
                device.bus_number,
                device.vendor_id,
                device.product_id,
                device.device_address,
                Self::truncate_string(manufacturer, 25),
                Self::truncate_string(product, 25),
                format!("{:02X}", device.device_class),
                timestamp
            );
            
            self.write_output(&format!("{}\n", row))?;
        }

        if self.include_metadata {
            self.write_output(&format!("\nTotal devices: {}\n", devices.len()))?;
        }

        Ok(())
    }

    /// Output devices in CSV format
    fn output_csv_devices(&mut self, devices: &[UsbDeviceInfo]) -> Result<()> {
        // CSV Header
        let header = "Bus,VendorID,ProductID,Address,Manufacturer,Product,SerialNumber,DeviceClass,Timestamp";
        self.write_output(&format!("{}\n", header))?;

        // Device rows
        for device in devices {
            let manufacturer = device.manufacturer.as_deref().unwrap_or("");
            let product = device.product.as_deref().unwrap_or("");
            let serial = device.serial_number.as_deref().unwrap_or("");
            
            let row = format!(
                "{},{:04X},{:04X},{},{},{},{},{:02X},{}",
                device.bus_number,
                device.vendor_id,
                device.product_id,
                device.device_address,
                Self::escape_csv_field(manufacturer),
                Self::escape_csv_field(product),
                Self::escape_csv_field(serial),
                device.device_class,
                device.timestamp.to_rfc3339()
            );
            
            self.write_output(&format!("{}\n", row))?;
        }

        Ok(())
    }

    /// Output a device change in JSON format
    fn output_json_change(&mut self, change: &UsbDeviceChange) -> Result<()> {
        let json = serde_json::json!({
            "change_type": change.get_change_type(),
            "device": change.get_device_info(),
            "timestamp": Utc::now()
        });

        let output = serde_json::to_string(&json)
            .context("Failed to serialize change to JSON")?;
        
        self.write_output(&format!("{}\n", output))?;
        Ok(())
    }

    /// Output a device change in table format
    fn output_table_change(&mut self, change: &UsbDeviceChange) -> Result<()> {
        let device = change.get_device_info();
        let change_type = change.get_change_type();
        let manufacturer = device.manufacturer.as_deref().unwrap_or("Unknown");
        let product = device.product.as_deref().unwrap_or("Unknown");
        let timestamp = device.timestamp.format("%Y-%m-%d %H:%M:%S").to_string();

        let change_indicator = if self.use_colors {
            match change_type {
                "CONNECTED" => format!("[{}]", "CONNECTED".green().bold()),
                "DISCONNECTED" => format!("[{}]", "DISCONNECTED".red().bold()),
                "RECONNECTED" => format!("[{}]", "RECONNECTED".yellow().bold()),
                _ => format!("[{}]", change_type),
            }
        } else {
            format!("[{}]", change_type)
        };

        let output = format!(
            "{} {} {:04X}:{:04X} {} - {} (Bus {}, Address {})",
            timestamp,
            change_indicator,
            device.vendor_id,
            device.product_id,
            manufacturer,
            product,
            device.bus_number,
            device.device_address
        );

        self.write_output(&format!("{}\n", output))?;
        Ok(())
    }

    /// Output a device change in CSV format
    fn output_csv_change(&mut self, change: &UsbDeviceChange) -> Result<()> {
        let device = change.get_device_info();
        let change_type = change.get_change_type();
        let manufacturer = device.manufacturer.as_deref().unwrap_or("");
        let product = device.product.as_deref().unwrap_or("");

        let row = format!(
            "{},{},{:04X},{:04X},{},{},{}",
            device.timestamp.to_rfc3339(),
            change_type,
            device.vendor_id,
            device.product_id,
            Self::escape_csv_field(manufacturer),
            Self::escape_csv_field(product),
            device.bus_number
        );

        self.write_output(&format!("{}\n", row))?;
        Ok(())
    }

    /// Write output to console and/or file
    fn write_output(&mut self, content: &str) -> Result<()> {
        // Write to console
        print!("{}", content);
        
        // Write to file if configured
        if let Some(ref mut file) = self.output_file {
            file.write_all(content.as_bytes())
                .context("Failed to write to output file")?;
        }
        
        Ok(())
    }

    /// Display monitoring status
    pub fn display_monitoring_status(&mut self, device_count: usize, filter: Option<&str>) -> Result<()> {
        let status = if self.use_colors {
            format!(
                "{} {} USB devices{}",
                "Monitoring".green().bold(),
                device_count.to_string().cyan().bold(),
                match filter {
                    Some(f) => format!(" (filtered: {})", f.yellow()),
                    None => String::new(),
                }
            )
        } else {
            format!(
                "Monitoring {} USB devices{}",
                device_count,
                match filter {
                    Some(f) => format!(" (filtered: {})", f),
                    None => String::new(),
                }
            )
        };

        self.write_output(&format!("{}\n", status))?;
        self.write_output("Press Ctrl+C to stop monitoring...\n\n")?;
        Ok(())
    }

    /// Clear the screen (for continuous monitoring)
    pub fn clear_screen(&self) -> Result<()> {
        execute!(stdout(), Clear(ClearType::All), MoveTo(0, 0))
            .context("Failed to clear screen")?;
        Ok(())
    }

    /// Truncate string to specified length
    fn truncate_string(s: &str, max_len: usize) -> String {
        if s.len() <= max_len {
            s.to_string()
        } else {
            format!("{}...", &s[..max_len.saturating_sub(3)])
        }
    }

    /// Escape CSV field if it contains special characters
    fn escape_csv_field(field: &str) -> String {
        if field.contains(',') || field.contains('"') || field.contains('\n') {
            format!("\"{}\"", field.replace('"', "\"\""))
        } else {
            field.to_string()
        }
    }

    /// Display error message
    pub fn display_error(&mut self, error: &str) -> Result<()> {
        let message = if self.use_colors {
            format!("{}: {}", "Error".red().bold(), error)
        } else {
            format!("Error: {}", error)
        };
        
        self.write_output(&format!("{}\n", message))?;
        Ok(())
    }

    /// Display warning message
    pub fn display_warning(&mut self, warning: &str) -> Result<()> {
        let message = if self.use_colors {
            format!("{}: {}", "Warning".yellow().bold(), warning)
        } else {
            format!("Warning: {}", warning)
        };
        
        self.write_output(&format!("{}\n", message))?;
        Ok(())
    }

    /// Display info message
    pub fn display_info(&mut self, info: &str) -> Result<()> {
        let message = if self.use_colors {
            format!("{}: {}", "Info".blue().bold(), info)
        } else {
            format!("Info: {}", info)
        };
        
        self.write_output(&format!("{}\n", message))?;
        Ok(())
    }
}