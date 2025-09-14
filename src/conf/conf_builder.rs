use crate::conf::args::args_parser::{ArgKind, ArgsParser};
use crate::conf::conf_error::ConfError;
use crate::conf::Conf;
use std::error::Error;
use std::net::{IpAddr, SocketAddr};
use std::path::Path;
use std::str::FromStr;
use std::time::Duration;
use std::{env, fs, u16, u64};

pub struct ConfBuilder { }

impl ConfBuilder {
    pub fn new(args: Vec<String>) -> Result<Conf, Box<dyn Error>> {
        let mut dir = String::new();
        if let Ok(path) = env::current_dir() {
            dir = path.to_string_lossy().to_string();
        }
        let mut conf = Conf {
            dir,
            port: 80,
            domain: "localhost".to_string(),
            browsing_enabled: true,
            workers: 64,
            timeout: Duration::from_secs(30),
            php_enabled: true,
            php_index: None,
            php_port: None,
            php_socket: None,
            https_enabled: false,
            https_pub_cert: "".to_string(),
            https_private_key: "".to_string(),
            logs_enabled: true,
            logs_min_level: "info".to_string(),
            logs_dir: None,
            load_balancing_enabled: false,
            load_balancing_servers: Vec::new(),
            cache_enabled: false,
            cache_dir: None,
            cache_patterns: Vec::new(),
        };

        Self::parse_args(&mut conf, args)?;

        Ok(conf)
    }

    fn parse_args(conf: &mut Conf, args: Vec<String>) -> Result<(), Box<dyn Error>> {
        let mut parser = ArgsParser::new();
        parser.add(ArgKind::Value("-f".to_string()));
        parser.add(ArgKind::Value("-p".to_string()));
        parser.add(ArgKind::Value("-d".to_string()));

        let args = parser.parse(&args)?;

        if args.contains_key("-f") {
            let filename = args.get("-f").unwrap();
            Self::parse_file(conf, filename)?;
        }

        if args.contains_key("-p") {
            let port = args.get("-p").unwrap();
            let port = Self::parse_u16(
                port,
                "Port is not valid integer"
            )?;
            conf.port = port;
        }

        if let Some(dir) = args.get("-d") {
            conf.dir  = Self::parse_path(dir)?
        }

        Ok(())
    }

    fn parse_file(conf: &mut Conf, path: &str) -> Result<(), Box<dyn Error>> {
        let enabled_values = ["1", "true", "t", "enabled", "y", "yes"];
        let mut line_no = 1;
        let contents = fs::read_to_string(path)?;
        for line in contents.lines() {
            if line.trim().starts_with(";") {
                continue;
            }
            let split: Vec<&str> = line.split("=").collect();
            let key = split.first().unwrap().trim();
            let value = split.last().unwrap().trim();

            if key == "server.port" {
                conf.port = Self::parse_u16(
                    value,
                    format!("Port is not valid integer. Line no. {}", line_no).as_str(),
                )?;
            }
            if key == "server.dir" {
                conf.dir = value.to_string();
            }
            if key == "server.workers" {
                conf.workers = Self::parse_usize(
                    value,
                    format!("Workers is not valid integer. Line no. {}", line_no).as_str(),
                )?;
            }
            if key == "server.timeout" {
                let timeout = Self::parse_u16(
                    value,
                    format!("Timeout is not valid integer. Line no. {}", line_no).as_str(),
                )?;
                let timeout = u64::from(timeout);
                conf.timeout = Duration::from_secs(timeout);
            }
            if key == "server.domain" {
                conf.domain = value.to_string();
            }
            if key == "server.browsing_enabled" {
                conf.browsing_enabled = enabled_values.contains(&value.to_string().as_str());
            }

            if key == "logs.enabled" {
                conf.logs_enabled = enabled_values.contains(&value.to_string().as_str());
            }
            if key == "logs.min_level" {
                let levels = ["debug", "info", "error"];
                if levels.contains(&value) {
                    conf.logs_min_level = value.to_string();
                } else {
                    return Err(Box::new(ConfError::new(
                        "Invalid min log level value. Line no. {}",
                    )));
                }
            }
            if key == "logs.dir" {
                let path = Path::new(value);
                if path.exists() && path.is_dir() {
                    conf.logs_dir = Some(path.into());
                } else {
                    return Err(format!("Invalid log dir. Line no. {}", line_no))?;
                }
            }

            if key == "load_balancer.enabled" {
                conf.load_balancing_enabled = enabled_values.contains(&value.to_lowercase().as_str());
            }
            if key == "load_balancer.servers" {
                let server_addr = Self::parse_server_addr(value, line_no)?;
                conf.load_balancing_servers.push(server_addr);
            }

            if key == "https.enabled" {
                conf.https_enabled = enabled_values.contains(&value.to_string().as_str());
            }
            if key == "https.public_key" {
                conf.https_pub_cert = match Path::new(value).is_file() {
                    true => value.to_string(),
                    false => {
                        return Err(format!("Public key doesn't exist. Line no. {}", line_no))?
                    }
                };
            }
            if key == "https.private_key" {
                conf.https_private_key = match Path::new(value).is_file() {
                    true => value.to_string(),
                    false => {
                        return Err(format!("Private key doesn't exist. Line no. {}", line_no))?
                    }
                };
            }

            if key == "php.enabled" {
                conf.php_enabled = enabled_values.contains(&value.to_string().as_str());
            }
            if key == "php.index" {
                conf.php_index = Some(value.to_string());
            }
            if key == "php.port" {
                let port = Self::parse_u16(
                    value,
                    format!("PHP FPM/FastCGI port is not valid integer. Line no. {}", line_no).as_str(),
                )?;
                conf.php_port = Some(port);
            }
            if key == "php.socket" {
                conf.php_socket = Some(value.to_string());
            }

            if key == "cache.enabled" {
                conf.cache_enabled = enabled_values.contains(&value.to_lowercase().as_str());
            }
            if key == "cache.dir" {
                let path = Path::new(value);
                if path.exists() && path.is_dir() {
                    conf.cache_dir = Some(path.into());
                } else {
                    return Err(format!("Invalid cache dir. Line no. {}", line_no))?;
                }
            }
            if key == "cache.pattern" {
                conf.cache_patterns.push(value.to_string());
            }

            line_no += 1;
        }

        Ok(())
    }

    fn parse_path(path: &String) -> Result<String, Box<dyn Error>> {
        let dir_path = Path::new(path);
        if dir_path.is_absolute()  && dir_path.is_dir()  {
            return Ok(path.clone());
        }
        else if dir_path.is_relative() {
            let current_dir = env::current_dir().unwrap();
            let path = current_dir.join(path);
            if path.is_dir() {
                return Ok(path.to_str().unwrap().to_string());
            }
        }
        Err(format!("Directory does not exist: {}", path))?
    }

    fn parse_usize(value: &str, msg: &str) -> Result<usize, Box<dyn Error>> {
        match value.parse::<usize>() {
            Ok(p) => Ok(p),
            Err(error) => Err(msg)?
        }
    }

    fn parse_u16(value: &str, msg: &str) -> Result<u16, Box<dyn Error>> {
        match value.parse::<u16>() {
            Ok(p) => Ok(p),
            Err(error) => Err(error)?
        }
    }

    fn parse_i16(value: &str, msg: &str) -> Result<i16, Box<dyn Error>> {
        match value.parse::<i16>() {
            Ok(p) => Ok(p),
            Err(_) => Err(msg)?
        }
    }

    fn parse_server_addr(ip_string: &str, line_no: usize) -> Result<SocketAddr, Box<dyn Error>> {
        let addr_port = ip_string.split(":").collect::<Vec<&str>>();
        if addr_port.len() != 2 {
            let msg =
                format!("Invalid load balancer server address. Line no. {}", line_no).to_string();
            return Err(msg)?;
        }
        let port = addr_port[1].trim().to_string();

        let addr = match IpAddr::from_str(addr_port[0]) {
            Ok(ip) => ip,
            Err(_) => {
                let msg = format!(
                    "Invalid load balancer server IP address {} in line no. {}",
                    addr_port[0], line_no
                );
                return Err(msg)?;
            }
        };

        let port = match port.parse() {
            Ok(port) => port,
            Err(_) => {
                return Err(format!("Invalid load balancer port {} in line no{}", line_no, port))?
            }
        };

        Ok(SocketAddr::new(addr, port))
    }
}
