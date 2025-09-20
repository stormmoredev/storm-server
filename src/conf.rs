mod conf_error;
mod conf_builder;
mod args;

use crate::conf::conf_builder::ConfBuilder;
use std::error::Error;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;

pub struct Conf {
    pub dir: String,
    pub port: u16,
    pub domain: String,
    pub browsing_enabled: bool,
    pub workers: usize,
    pub timeout: Duration,
    pub php_enabled: bool,
    pub php_index: Option<String>,
    pub php_port: Option<u16>,
    pub php_socket: Option<String>,
    pub https_enabled: bool,
    pub https_pub_cert: String,
    pub https_private_key: String,
    pub logs_enabled: bool,
    pub logs_min_level: String,
    pub logs_dir: Option<PathBuf>,
    pub load_balancing_enabled: bool,
    pub load_balancing_servers: Vec<SocketAddr>,
    pub cache_enabled: bool,
    pub cache_dir: Option<PathBuf>,
    pub cache_patterns: Vec<String>
}

impl Conf {
    pub fn new(args: Vec<String>) -> Result<Conf, Box<dyn Error>> {
        ConfBuilder::new(args)
    }
}

