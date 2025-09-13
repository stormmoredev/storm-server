use std::process::Command;
use std::{process, thread};
use port_check::{free_local_port_in_range, is_local_port_free};
use crate::conf::Conf;
use crate::php::fcgi_client::FcgiClient;

mod fcgi_response;
mod fcgi_client;
mod fcgi_socket;

static FCGI_PORT: u16 = 7077;

pub struct Php {
    enabled: bool,
    port: Option<u16>,
    sock: Option<String>,
    pub server_name: String,
    pub server_port: u16
}

impl Php {
    pub fn new(conf: &Conf) -> Php {
        let mut port: Option<u16> = None;
        if conf.php_enabled && conf.php_socket.is_none() {
            if conf.php_port.is_none() {
                if is_local_port_free(9000) {
                    port = Some(FCGI_PORT);
                }
                else {
                    port = Some(9000);
                }
            }
            else {
                port = conf.php_port;
            }
        }
        if conf.php_enabled && conf.php_socket.is_some() {
            let sock = conf.php_socket.clone().unwrap();
            if cfg!(target_os = "windows") {
                eprintln!("Unix sockets are not supported on Windows: {}", sock);
                process::exit(1);
            }
        }

        Php {
            enabled: conf.php_enabled,
            port,
            sock: conf.php_socket.clone(),
            server_name: conf.domain.clone(),
            server_port: conf.port
        }
    }

    pub fn init_fast_cgi()
    {
        if is_local_port_free(FCGI_PORT) {
            let _ = Self::try_run_php_cgi(FCGI_PORT);
        }
    }

    pub fn get_client(&self) -> Option<FcgiClient> {
        if self.enabled && (self.port.is_some() || self.sock.is_some()) {
            return Some(FcgiClient::new(&self.port,
                                        &self.sock,
                        self.server_port,
                        &self.server_name));
        }
        return None;
    }

    fn try_run_php_cgi(port: u16) -> bool {
        match Command::new("php-cgi")
            .args(["-v"])
            .output()
        {
            Ok(output) => {
                thread::spawn(move || {
                    let address = format!("127.0.0.1:{}", port);
                    let _ = Command::new("php-cgi")
                        .args(["-b", address.as_str()])
                        .output();
                });
                true
            },
            Err(msg) => false
        }
    }
}