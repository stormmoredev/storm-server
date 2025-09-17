use std::collections::HashMap;
use std::error::Error;
use std::io;
use std::io::{Read, Write};
use std::net::SocketAddr;
use crate::server::http_server::http_server_socket::HttpServerSocket;

pub struct HttpStream {
    stream: HttpServerSocket,
    buffer: Vec<u8>,
    len: Option<usize>,
    read: usize,
    method: String,
    query_path: String,
    path: String,
    query: String,
    pub headers: HashMap<String, String>
}


impl HttpStream {
    pub async fn new(stream: HttpServerSocket) -> Result<HttpStream, Box<dyn Error>> {
        let mut http_reader = HttpStream {
            stream,
            buffer: Vec::with_capacity(1024),
            len: None,
            read: 0,
            method: String::new(),
            query_path: String::new(),
            path: String::new(),
            query: String::new(),
            headers: HashMap::new()
        };
        http_reader.init().await?;

        Ok(http_reader)
    }

    pub fn path(&self) -> &str { self.path.as_str() }
    pub fn query(&self) -> &str { self.query.as_str() }
    pub fn query_path(&self) -> &str {  self.query_path.as_str() }
    pub fn method(&self) -> &str { self.method.as_str() }
    pub async fn write(&mut self, buf: &[u8]) -> io::Result<()> {
        self.stream.write_all(buf).await
    }

    pub async fn read_body(&mut self, mut buf: &mut [u8]) -> io::Result<usize> {
        if self.len.is_some() {
            let len = self.len.unwrap();
            if self.read >= len {
                return Ok(0);
            }
        }
        else {
            return Ok(0);
        }
        let result;
        if self.buffer.len() > 0 {
            let size = if self.buffer.len() > buf.len() { buf.len() } else { self.buffer.len() };
            let to_copy = self.buffer.drain(..size).collect::<Vec<u8>>();
            result = buf.write(&to_copy)?;
        }
        else {
            result = self.stream.read(buf).await?;
        }
        self.read += result;
        Ok(result)
    }

    pub fn header_block(&self) -> Vec<u8> {
        let mut header_block = Vec::new();
        let status_line = format!("{} {} HTTP/1.1\r\n", self.method, self.query_path);
        header_block.extend_from_slice(status_line.as_bytes());
        for (name, value) in &self.headers {
            let header_line = format!("{}: {}\r\n", name, value);
            header_block.extend_from_slice(header_line.as_bytes());
        }
        header_block.extend_from_slice(b"\r\n");
        header_block
    }

    async fn init(&mut self) -> Result<(), Box<dyn Error>>  {
        let max = 8 * 1024;
        let mut buffer_size = 0;
        let methods = ["GET", "POST", "HEAD", "DELETE", "TRACE", "PUT", "PATCH", "OPTIONS"];

        loop {
            let mut buf = [0; 4 * 1024];
            let read = match self.stream.read(&mut buf).await {
                Ok(r) => r,
                Err(e) => {
                    if buffer_size == 0{
                        return Err("No data received. Probably browser pre-connection.")?;
                    };
                    return Err(e)?;
                }
            };
            if read == 0 {
                return Err("No valid header received")?
            }
            buffer_size += read;

            self.buffer.extend_from_slice(&buf[..read]);

            if self.buffer.windows(4).any(|window| window == [13,10,13,10]) {
                break;
            }
            if buffer_size > max {
                return Err("Request exceed max size of header")?
            }
        }

        let pos = self.buffer
            .windows(4)
            .position(|window| window == [13,10,13,10])
            .unwrap() + 4;

        let header_block = self.buffer.drain(..pos).collect::<Vec<u8>>();
        let header_block = match String::from_utf8(header_block) {
            Ok(h) => h,
            Err(e) => return Err(e)?
        };
        let mut lines = header_block.lines();
        let status_header = lines.next();
        let status_header = status_header.unwrap_or(&"");
        let mut  parts = status_header.splitn(3, " ");
        let status_header = match (parts.next(), parts.next()) {
            (Some(name), Some(value)) => Some((name.to_string(), value.to_string())),
            _ => None,
        };
        if status_header.is_none() {
            return Err("Malformed status HTTP header")?
        }
        let http_header = status_header.unwrap();

        self.method = http_header.0.to_uppercase();
        self.query_path = http_header.1;

        if !methods.contains(&self.method.as_str()) {
            return Err("Invalid HTTP method")?
        }
        match self.query_path.find('?') {
            Some(index) => {
                self.path = self.query_path[..index].to_string();
                self.query  = self.query_path[index+1..].to_string();
            }
            None => {
                self.path = self.query_path.clone();
            }
        }

        self.headers = lines
            .filter_map(|line| {
                let mut parts = line.splitn(2, ": ");
                match (parts.next(), parts.next()) {
                    (Some(name), Some(value)) => Some((name.to_string(), value.to_string())),
                    _ => None,
                }
            })
            .collect();

        let len_header = self.headers.iter()
            .find(|(name, _)| name.eq_ignore_ascii_case("content-length"));
        if let Some(len_header) = len_header {
            let len = len_header.1.parse::<usize>().unwrap_or(0);
            self.len = Some(len)
        }

        if ["POST", "PUT"].contains(&self.method.as_str()) && self.len.is_none() {
            return Err("Content-Length required")?;
        }

        Ok(())
    }
}
