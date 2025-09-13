use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::io::{BufReader};
use std::sync::Arc;
use crate::server::http_server::request::Request;
use crate::server::http_server::response::Response;
use crate::logger::Logger;
use crate::php::Php;

impl Response {
    pub async fn php(request: &mut Request, php: Php) -> Result<Response, Box<dyn Error>> {
        match php.get_client() {
            Some(client) => {
                match client.handle(request).await {
                    Ok(response) => {
                       Ok(Response {
                           status: response.status(),
                           headers: response.headers(), 
                           content_size: None,
                           content: Box::new(response)
                       })
                    },
                    Err(e) => {
                        return Err(e);
                    }
                }
            },
            None => Ok(Response::get_php_raw_file_response(request.file_path()))
        }
    }

    fn get_php_raw_file_response(path: &str) -> Response {
        let file = File::open(path).unwrap();
        let file = BufReader::new(file);
        let file_reader = Box::new(BufReader::new(file));

        let mut  headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "text/plain".to_string());
        headers.insert("Connection".to_string(), "close".to_string());

        Response {
            status: 200,
            content_size: None,
            headers,
            content: file_reader
        }
    }
}