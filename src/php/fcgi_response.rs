use crate::php::fcgi_socket::FcgiStream;
use std::collections::HashMap;
use std::io::{Read, Result};

pub struct FcgiResponse {
    status: u32,
    stream: Box<dyn FcgiStream>,
    buf: Vec<u8>,
    headers: HashMap<String, String>,
}

impl FcgiResponse {
    pub fn new(stream: Box<dyn FcgiStream>) -> FcgiResponse {
        let mut response = FcgiResponse {
            status: 200,
            stream,
            buf: Vec::new(),
            headers: HashMap::new()
        };

        response.init();

        response
    }

    pub fn status(&self) -> u32 {
        self.status
    }

    pub fn headers(&self) -> HashMap<String, String> {
        self.headers.clone()
    }

    fn init(&mut self) {
        self.headers.insert("Connection".to_string(), "close".to_string());
        loop {
            let res = self.read_record();
            if res.unwrap_or(0) == 0 {
                break;
            }
            let text = String::from_utf8_lossy(&self.buf);
            if text.contains("\r\n\r\n") {
                let index = text.find("\r\n\r\n").unwrap();
                let mut headers: Vec<_> = self.buf.drain(..index + 4).collect();
                headers.truncate(headers.len() - 4);
                let headers = String::from_utf8_lossy(&headers);
                let status_line = headers.lines().find(|x| x.to_lowercase().starts_with("status:"));
                if let Some(status_line) = status_line {
                    if let Some(code) = status_line.split_whitespace().nth(1) {
                        self.status = code.parse::<u32>().unwrap_or(200);
                    }
                }
                headers
                    .lines()
                    .skip(1)
                    .for_each(|header| {
                        let header = header.trim().splitn(2,':').collect::<Vec<&str>>();
                        if header.len() == 2 {
                            self.headers.insert(header[0].to_string(), header[1].to_string());
                        }
                    });
                break;
            }
        }
    }

    fn read_record(&mut self) -> Result<usize> {
        let mut buf = [0u8; 8];
        if self.stream.read_exact(&mut buf).is_err() {
            return Ok(0);
        }

        let record_type = buf[1];
        let content_length = u16::from_be_bytes([buf[4], buf[5]]) as usize;
        let padding_length = buf[6] as usize;

        let mut content = vec![0u8; content_length];
        if self.stream.read_exact(&mut content).is_err() {
            return Ok(0);
        }
        self.buf.extend(content);

        let mut padding = vec![0u8; padding_length];
        let _ = self.stream.read_exact(&mut padding);

        match record_type {
            6 => Ok(content_length),
            7 => Ok(content_length),
            3 => Ok(0),
            _ => Ok(0)
        }
    }
}

impl Read for FcgiResponse {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        if self.buf.len() == 0 {
            if self.read_record().unwrap_or(0) == 0 {
                return Ok(0);
            }
        }
        let size = if buf.len() > self.buf.len() { self.buf.len() } else { buf.len() };
        let read: Vec<_> = self.buf.drain(0..size).collect();
        buf[0..size].copy_from_slice(&read);
        Ok(size)
    }
}