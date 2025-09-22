use clap::{Arg, Command};
use std::net::IpAddr;
use std::str::FromStr;
use tracing::{info, error, warn};
use tracing_subscriber;

mod config;
mod node;
mod error;

use config::Config;
use node::Node;
use error::O3StorageError;

#[tokio::main]
async fn main() -> Result<(), O3StorageError> {
    tracing_subscriber::init();

    let matches = Command::new("O3Storage")
        .version("0.1.0")
        .about("Distributed immutable object storage system")
        .arg(
            Arg::new("ip")
                .long("ip")
                .help("IP address for this node")
                .required(false)
        )
        .arg(
            Arg::new("peers")
                .long("peers")
                .help("Comma-separated list of peer IP addresses")
                .required(false)
        )
        .arg(
            Arg::new("port")
                .long("port")
                .help("Port to listen on")
                .default_value("8080")
        )
        .get_matches();

    info!("Starting O3Storage distributed object storage system");

    system::hardware_check().map_err(|e| O3StorageError::System(e.to_string()))?;

    let config = if let Some(ip_str) = matches.get_one::<String>("ip") {
        let ip = IpAddr::from_str(ip_str)
            .map_err(|e| O3StorageError::InvalidConfig(format!("Invalid IP address: {}", e)))?;
        
        let peers = if let Some(peers_str) = matches.get_one::<String>("peers") {
            peers_str
                .split(',')
                .map(|s| IpAddr::from_str(s.trim()))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| O3StorageError::InvalidConfig(format!("Invalid peer IP: {}", e)))?
        } else {
            Vec::new()
        };

        let port = matches.get_one::<String>("port")
            .unwrap()
            .parse::<u16>()
            .map_err(|e| O3StorageError::InvalidConfig(format!("Invalid port: {}", e)))?;

        Config::new(ip, port, peers)
    } else {
        interactive_setup().await?
    };

    info!("Node configuration: {:?}", config);

    let node = Node::new(config).await?;
    node.start().await?;

    Ok(())
}

async fn interactive_setup() -> Result<Config, O3StorageError> {
    use std::io::{self, Write};

    println!("O3Storage Node Setup");
    println!("===================");

    print!("Enter IP address for this node: ");
    io::stdout().flush().unwrap();
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    let ip = IpAddr::from_str(input.trim())
        .map_err(|e| O3StorageError::InvalidConfig(format!("Invalid IP address: {}", e)))?;

    print!("Enter port (default 8080): ");
    io::stdout().flush().unwrap();
    input.clear();
    io::stdin().read_line(&mut input).unwrap();
    let port = if input.trim().is_empty() {
        8080
    } else {
        input.trim().parse::<u16>()
            .map_err(|e| O3StorageError::InvalidConfig(format!("Invalid port: {}", e)))?
    };

    println!("Enter peer IP addresses (comma-separated, or press Enter to skip):");
    input.clear();
    io::stdin().read_line(&mut input).unwrap();
    let peers = if input.trim().is_empty() {
        Vec::new()
    } else {
        input
            .trim()
            .split(',')
            .map(|s| IpAddr::from_str(s.trim()))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| O3StorageError::InvalidConfig(format!("Invalid peer IP: {}", e)))?
    };

    Ok(Config::new(ip, port, peers))
}