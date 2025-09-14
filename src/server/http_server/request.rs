use std::collections::HashMap;
use std::error::Error;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::fs;
use std::io::Read;
use urlencoding::decode;
use crate::conf::Conf;
use crate::server::http_stream::HttpStream;
use crate::server::http_server::response::Response;
use crate::server::cache::Cache;

pub struct Request {
    stream:  HttpStream,
    dir_path: String,
    peer_addr: SocketAddr,
    pub file_path: PathBuf

}

impl Request {
    pub fn new(stream: HttpStream, addr: SocketAddr, config: &Conf) ->  Result<Request, Box<dyn Error>>  {
        let path = &stream.path();
        let mut local_path = String::from(&config.dir);
        if !path.contains("/..") {
            local_path.push_str(path);
        }

        let local_path = decode(&local_path)?;
        let local_path = local_path.as_ref();
        let file_path = Path::new(&local_path).to_path_buf();

        Ok(Request {
            stream,
            dir_path: config.dir.clone(),
            peer_addr: addr,
            file_path,
        })
    }

    pub fn headers(&self) -> &HashMap<String, String> { &self.stream.headers }
    pub fn query(&self) -> &str { self.stream.query() }
    pub fn method(&self) -> &str { self.stream.method() }
    pub fn path(&self) -> &str { self.stream.path() }
    pub fn query_path(&self) -> &str { self.stream.query_path() }
    pub fn doc_root(&self) -> &str { self.dir_path.as_str() }
    pub fn peer_addr(&self) -> SocketAddr { self.peer_addr }
    pub fn file_path(&self) -> &str { self.file_path.to_str().unwrap_or_default() }
    pub fn has_body(&self) -> bool {
        self.method() == "POST" ||
        self.method() == "PUT" ||
        self.method() == "PATCH" ||
        self.method() == "DELETE"
    }

    pub fn rewrite(&mut self, file_path: PathBuf) {
        self.file_path = file_path;
    }

    pub fn stream_mut(&mut self) -> &mut HttpStream { &mut self.stream }

    pub async fn read_body(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.stream.read(buf).await
    }

    pub async fn output_response(
        mut self,
        mut res: Response,
        conf: &Conf,
    ) -> Result<(), Box<dyn Error>> {
        let mut headers: Vec<(String, String)> = res
            .headers()
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        let cache_path = Cache::process_headers(&mut headers, self.query_path(), conf);

        let status_line = res.status_line();
        self.stream.write(status_line.as_bytes()).await?;
        let mut cache_buf: Option<Vec<u8>> = if cache_path.is_some() {
            let mut v = Vec::new();
            v.extend_from_slice(status_line.as_bytes());
            Some(v)
        } else {
            None
        };

        let headers_len = headers.len();
        for (idx, (key, value)) in headers.iter().enumerate() {
            let line = if idx == headers_len - 1 {
                format!("{}:{}", key, value)
            } else {
                format!("{}:{}\n", key, value)
            };
            self.stream.write(line.as_bytes()).await?;
            if let Some(buf) = cache_buf.as_mut() {
                buf.extend_from_slice(line.as_bytes());
            }
        }

        self.stream.write("\r\n\r\n".as_bytes()).await?;
        if let Some(buf) = cache_buf.as_mut() {
            buf.extend_from_slice("\r\n\r\n".as_bytes());
        }

        loop {
            let mut buff = [0; 256 * 1024];
            let read_size = res.read(&mut buff)?;
            if read_size == 0 {
                break;
            }
            self.stream.write(&buff[0..read_size]).await?;
            if let Some(buf) = cache_buf.as_mut() {
                buf.extend_from_slice(&buff[0..read_size]);
            }
        }

        if let (Some(buf), Some(final_path)) = (cache_buf, cache_path) {
            let _ = Cache::write(&buf, &final_path);
        }

        Ok(())
    }
}