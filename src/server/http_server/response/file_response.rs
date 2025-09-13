use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use crate::server::http_server::response::mime::get_mime;
use crate::server::http_server::response::Response;

impl Response {
    pub fn file(path: &PathBuf) -> Response {
        let file = File::open(path).unwrap();
        let size = file.metadata().unwrap().len();
        let ext = path.extension().unwrap_or_else(|| OsStr::new(""));
        let ext = ext.to_str().unwrap_or("");
        let file_reader = Box::new(BufReader::new(file));
        let mut  headers = HashMap::new();
        
        headers.insert("Content-Length".to_string(), size.to_string());
        headers.insert("Content-Type".to_string(), get_mime(ext));
        headers.insert("Connection".to_string(), "close".to_string());

        Response {
            status: 200,
            content_size: Some(size),
            headers,
            content: file_reader
        }
    }
}