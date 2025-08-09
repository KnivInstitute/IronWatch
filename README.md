# IronWatch v1.0.0

A modular GUI/CLI tool for monitoring USB device inputs by KnivInstitute.

## Features

### GUI Mode (Default)
- **Modern GUI Interface**: Beautiful, responsive GUI built with egui
- **Real-time Device Monitoring**: Live updates with smooth animations
- **Interactive Dashboard**: Overview of connected devices and statistics
- **Device Table View**: Detailed device information in tabular format
- **Filtering & Search**: Real-time filtering of devices
- **Settings Panel**: Configure monitoring preferences
- **Dark/Light Theme**: Customizable appearance

### CLI Mode (Advanced Users)
- **USB Device Monitoring**: Real-time monitoring of USB device connections and disconnections
- **Multiple Output Formats**: Support for JSON, Table, and CSV output formats
- **Filtering**: Filter devices by name patterns
- **Configuration Management**: Persistent configuration with JSON-based settings
- **Logging**: Comprehensive logging with configurable levels

### General Features
- **Cross-platform**: Built with Rust for Windows, macOS, and Linux
- **High Performance**: Efficient USB monitoring with minimal resource usage
- **Extensible Architecture**: Modular design for easy feature additions

## Installation

### From Source

```bash
git clone https://github.com/KnivInstitute/IronWatch.git
cd IronWatch

# Build GUI version (default)
cargo build --release

# Build CLI version only
cargo build --release --features cli --no-default-features
```

## Usage

### List Connected USB Devices

```bash
# Table format (default)
ironwatch list

# JSON format
ironwatch list --format json

# CSV format
ironwatch list --format csv
```

### Monitor USB Device Changes

```bash
# Single scan mode
ironwatch monitor

# Continuous monitoring mode
ironwatch monitor --continuous

# Filter devices by name pattern
ironwatch monitor --filter "camera" --continuous

# Output to file
ironwatch monitor --continuous --output usb_events.log
```

### Configuration Management

```bash
# Show current configuration
ironwatch config show

# Set configuration values
ironwatch config set monitoring.poll_interval_ms 1000
ironwatch config set output.default_format json
ironwatch config set logging.level debug
```

### Command Line Options

```bash
# Global options
ironwatch --help                    # Show help
ironwatch --version                 # Show version
ironwatch --config /path/to/config  # Use custom config file
ironwatch -v                        # Verbose logging
ironwatch -vv                       # Very verbose logging

# Subcommand help
ironwatch list --help
ironwatch monitor --help
ironwatch config --help
```

## Configuration

IronWatch uses a JSON configuration file located at:

- **Windows**: `%APPDATA%\ironwatch\config.json`
- **macOS**: `~/Library/Application Support/ironwatch/config.json`
- **Linux**: `~/.config/ironwatch/config.json`

### Configuration Options

```json
{
  "monitoring": {
    "poll_interval_ms": 500,
    "auto_start": false,
    "track_input_events": true,
    "detect_suspicious_activity": true
  },
  "output": {
    "default_format": "table",
    "timestamp_format": "%Y-%m-%d %H:%M:%S UTC",
    "include_metadata": true,
    "color_output": true,
    "max_log_entries": 1000
  },
  "logging": {
    "level": "info",
    "file_logging": false,
    "log_file_path": null,
    "max_log_file_size_mb": 10,
    "rotate_logs": true
  },
  "filters": {
    "ignored_vendors": [],
    "ignored_products": [],
    "allowed_device_classes": null,
    "name_patterns": []
  }
}
```

## Output Formats

### Table Format
```
Bus VID:PID  Address Manufacturer         Product                   Class           Timestamp
------------------------------------------------------------------------------------------------
2   1022:15BA 0       AMD                  USB Controller            09              23:31:00
3   5986:118C 1       Generic              Integrated Camera         EF              23:31:00
```

### JSON Format
```json
{
  "timestamp": "2025-08-06T23:31:07Z",
  "device_count": 2,
  "devices": [
    {
      "bus_number": 2,
      "device_address": 0,
      "vendor_id": 4130,
      "product_id": 5562,
      "manufacturer": "AMD",
      "product": "USB Controller",
      "device_class": 9,
      "timestamp": "2025-08-06T23:31:07Z",
      "connection_status": "Connected"
    }
  ]
}
```

### CSV Format
```csv
Bus,VendorID,ProductID,Address,Manufacturer,Product,SerialNumber,DeviceClass,Timestamp
2,1022,15BA,0,AMD,USB Controller,,09,2025-08-06T23:31:07Z
```

## Architecture

IronWatch is built with a modular architecture:

- **`cli.rs`**: Command-line interface and argument parsing
- **`usb_monitor.rs`**: USB device detection and monitoring logic
- **`config.rs`**: Configuration management and persistence
- **`output.rs`**: Output formatting and display management
- **`main.rs`**: Application orchestration and entry point

## Dependencies

- **rusb**: USB device access and monitoring
- **clap**: Command-line argument parsing
- **tokio**: Async runtime for monitoring
- **serde**: Serialization/deserialization
- **chrono**: Date and time handling
- **crossterm**: Cross-platform terminal manipulation
- **anyhow**: Error handling

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## Security Considerations

- IronWatch requires appropriate permissions to access USB devices
- On Linux, you may need to run with `sudo` or add your user to the appropriate groups
- The tool logs device information which may be sensitive in some environments

## Support

For issues and questions, please open an issue on the GitHub repository.

---