use std::io::{Read, Write};
use std::net::TcpStream;

pub trait FcgiStream: Read + Write + Send {}
impl FcgiStream for TcpStream {}

fn create_local_tcp_stream(port: u16) -> Box<dyn FcgiStream> {
    let address = format!("127.0.0.1:{}", port);
    let stream = TcpStream::connect(address).unwrap();
    Box::new(stream)
}

#[cfg(unix)]
pub mod fcgi_socket {
    use std::error::Error;
    use std::os::unix::net::{UnixStream};
    use std::process::exit;
    use crate::php::fcgi_socket::{create_local_tcp_stream, FcgiStream};

    impl FcgiStream for UnixStream {}
    pub fn get_socket(port: &Option<u16>, socket: &Option<String>) ->  Result<Box<dyn FcgiStream>, Box<dyn Error>> {
        let socket = socket.clone();
        if socket.is_some() {
            let socket = socket.unwrap();
            let stream = UnixStream::connect(socket)?;
            return Ok(Box::new(stream));
        }
        if port.is_some() {
            return Ok(create_local_tcp_stream(port.unwrap()))
        }

        eprintln!("php.port or php.socket is required");
        exit(0);
    }
}

#[cfg(windows)]
pub mod fcgi_socket {
    use std::error::Error;
    use std::net::TcpStream;
    use std::process::exit;
    use crate::php::fcgi_socket::{create_local_tcp_stream, FcgiStream};

    pub fn get_socket(port: &Option<u16>, socket: &Option<String>) ->  Result<Box<dyn FcgiStream>, Box<dyn Error>> {
        if port.is_some() {
            return Ok(create_local_tcp_stream(port.unwrap()))
        }

        eprintln!("php.port is required");
        exit(0);
    }
}