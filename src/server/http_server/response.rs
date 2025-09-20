mod mime;
mod string_reader;
mod dir_response;
mod not_found_response;
mod file_response;
mod php_response;

use std::collections::HashMap;
use std::io;
use std::io::Read;

pub struct Response {
    status: u32,
    //content_size: Option<u64>,
    headers: HashMap<String, String>,
    content: Box<dyn Read + Send>
}

impl Response {
    pub fn headers(&self) -> &HashMap<String, String> {
        &self.headers
    }

    pub fn status_line(&self) -> String {
        format!("HTTP/1.1 {} OK\r\n", self.status)
    }

    pub fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.content.read(buf)
    }
}