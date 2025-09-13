use std::collections::HashMap;
use std::error::Error;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use urlencoding::decode;
use crate::conf::Conf;
use crate::server::http_stream::HttpStream;
use crate::server::http_server::response::Response;

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

    pub async fn read_body(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.stream.read(buf).await
    }

    pub async fn output_response(mut self, mut res: Response) -> Result<(), Box<dyn Error>> {
        self.stream.write(res.status_line().as_bytes()).await?;

        let mut idx = 0;
        for (key, value) in res.headers() {
            if idx == res.headers().len() - 1 {
                self.stream.write(format!("{}:{}", key, value).as_bytes()).await?;
            }
            else {
                self.stream.write(format!("{}:{}\n", key, value).as_bytes()).await?;
            }
            idx += 1;
        }

        self.stream.write("\r\n\r\n".as_bytes()).await?;

        loop {
            let mut buff = [0; 256 * 1024];
            let read_size = res.read(&mut buff)?;
            if read_size == 0 {
                return Ok(());
            }
            self.stream.write(&buff[0..read_size]).await?;
        }
    }
}