use crate::server::http_server::response::string_reader::StringReader;
use crate::server::http_server::response::Response;
use std::collections::HashMap;

impl Response {
    pub fn not_found(query_path: &str) -> Response {
        let mut body = String::new();
        body.push_str(include_str!("../request_handler/templates/404.html"));
        body = body.replace("%path%", query_path);

        let mut headers = HashMap::new();
        headers.insert("Content-Length".to_string(), body.len().to_string());
        headers.insert("Content-Type".to_string(), "text/html".to_string());
        headers.insert("Connection".to_string(), "close".to_string());

        Response {
            status: 404,
            headers,
            content_size: Some(body.len() as u64),
            content: Box::new(StringReader::new(body))
        }
    }
}