use clap::{Arg, Command, ArgMatches};
use std::path::PathBuf;

/// Command line interface configuration and parsing
pub struct CliConfig {
    pub monitor_mode: bool,
    pub output_format: OutputFormat,
    pub config_file: Option<PathBuf>,
    pub log_level: String,
    pub device_filter: Option<String>,
    pub continuous: bool,
    pub output_file: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub enum OutputFormat {
    Json,
    Table,
    Csv,
}

impl Default for CliConfig {
    fn default() -> Self {
        Self {
            monitor_mode: false,
            output_format: OutputFormat::Table,
            config_file: None,
            log_level: "info".to_string(),
            device_filter: None,
            continuous: false,
            output_file: None,
        }
    }
}

/// Build the CLI application structure
pub fn build_cli() -> Command {
    Command::new("IronWatch")
        .version("1.0.0")
        .author("KnivInstitute")
        .about("A modular CLI tool for monitoring USB device inputs")
        .subcommand(
            Command::new("monitor")
                .about("Start monitoring USB device inputs")
                .arg(
                    Arg::new("continuous")
                        .short('c')
                        .long("continuous")
                        .action(clap::ArgAction::SetTrue)
                        .help("Run in continuous monitoring mode")
                )
                .arg(
                    Arg::new("filter")
                        .short('f')
                        .long("filter")
                        .value_name("DEVICE_PATTERN")
                        .help("Filter devices by name pattern")
                )
                .arg(
                    Arg::new("output")
                        .short('o')
                        .long("output")
                        .value_name("FILE")
                        .help("Output results to file")
                )
        )
        .subcommand(
            Command::new("list")
                .about("List all connected USB devices")
                .arg(
                    Arg::new("format")
                        .short('f')
                        .long("format")
                        .value_name("FORMAT")
                        .value_parser(["json", "table", "csv"])
                        .default_value("table")
                        .help("Output format")
                )
        )
        .subcommand(
            Command::new("config")
                .about("Manage configuration settings")
                .subcommand(
                    Command::new("show")
                        .about("Show current configuration")
                )
                .subcommand(
                    Command::new("set")
                        .about("Set configuration value")
                        .arg(
                            Arg::new("key")
                                .value_name("KEY")
                                .required(true)
                                .help("Configuration key")
                        )
                        .arg(
                            Arg::new("value")
                                .value_name("VALUE")
                                .required(true)
                                .help("Configuration value")
                        )
                )
        )
        .arg(
            Arg::new("config")
                .short('c')
                .long("config")
                .value_name("FILE")
                .help("Path to configuration file")
        )
        .arg(
            Arg::new("verbose")
                .short('v')
                .long("verbose")
                .action(clap::ArgAction::Count)
                .help("Increase logging verbosity")
        )
}

/// Parse command line arguments into configuration
pub fn parse_args(matches: &ArgMatches) -> anyhow::Result<CliConfig> {
    let mut config = CliConfig::default();
    
    // Global arguments
    if let Some(config_file) = matches.get_one::<String>("config") {
        config.config_file = Some(PathBuf::from(config_file));
    }
    
    // Set log level based on verbosity
    config.log_level = match matches.get_count("verbose") {
        0 => "info".to_string(),
        1 => "debug".to_string(),
        _ => "trace".to_string(),
    };
    
    // Handle subcommands
    match matches.subcommand() {
        Some(("monitor", sub_matches)) => {
            config.monitor_mode = true;
            config.continuous = sub_matches.get_flag("continuous");
            
            if let Some(filter) = sub_matches.get_one::<String>("filter") {
                config.device_filter = Some(filter.clone());
            }
            
            if let Some(output) = sub_matches.get_one::<String>("output") {
                config.output_file = Some(PathBuf::from(output));
            }
        }
        Some(("list", sub_matches)) => {
            if let Some(format) = sub_matches.get_one::<String>("format") {
                config.output_format = match format.as_str() {
                    "json" => OutputFormat::Json,
                    "csv" => OutputFormat::Csv,
                    _ => OutputFormat::Table,
                };
            }
        }
        _ => {}
    }
    
    Ok(config)
}

/// Print application banner
pub fn print_banner() {
    println!(r#"
██╗██████╗  ██████╗ ███╗   ██╗██╗    ██╗ █████╗ ████████╗ ██████╗██╗  ██╗
██║██╔══██╗██╔═══██╗████╗  ██║██║    ██║██╔══██╗╚══██╔══╝██╔════╝██║  ██║
██║██████╔╝██║   ██║██╔██╗ ██║██║ █╗ ██║███████║   ██║   ██║     ███████║
██║██╔══██╗██║   ██║██║╚██╗██║██║███╗██║██╔══██║   ██║   ██║     ██╔══██║
██║██║  ██║╚██████╔╝██║ ╚████║╚███╔███╔╝██║  ██║   ██║   ╚██████╗██║  ██║
╚═╝╚═╝  ╚═╝ ╚═════╝ ╚═╝  ╚═══╝ ╚══╝╚══╝ ╚═╝  ╚═╝   ╚═╝    ╚═════╝╚═╝  ╚═╝
                                                                           
                    USB Device Input Monitor v1.0.0
                         by KnivInstitute
"#);
}