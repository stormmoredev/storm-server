use crate::php::fcgi_response::FcgiResponse;
use crate::php::fcgi_socket::fcgi_socket::get_socket;
use crate::php::fcgi_socket::FcgiStream;
use crate::server::http_server::request::Request;
use std::error::Error;
use std::io::Write;

const FCGI_VERSION: u8 = 1;
const FCGI_BEGIN_REQUEST: u8 = 1;
const FCGI_PARAMS: u8 = 4;
const FCGI_STDIN: u8 = 5;
const FCGI_RESPONDER: u16 = 1;

pub struct FcgiClient<'a> {
    port: &'a Option<u16>,
    socket: &'a Option<String>,
    server_port: u16,
    server_name: &'a str
}

impl<'a> FcgiClient<'a> {
    pub fn new(port: &'a Option<u16>, socket: &'a Option<String>, server_port: u16, server_name: &'a str) -> FcgiClient<'a> {
        FcgiClient {
            port,
            socket,
            server_port,
            server_name
        }
    }

    pub async fn handle(&self, request: &mut Request) -> Result<FcgiResponse, Box<dyn Error>> {
        let mut stream = get_socket(&self.port, &self.socket)?;

        let begin_body = [0u8, FCGI_RESPONDER as u8, 0, 0, 0, 0, 0, 0];
        self.write_record(&mut stream, FCGI_BEGIN_REQUEST, 1, &begin_body)?;

        let  params = [
            ("GATEWAY_INTERFACE", "CGI/1.1"),
            ("SCRIPT_FILENAME", request.file_path()),
            ("SCRIPT_NAME", request.path()),
            ("DOCUMENT_ROOT", request.doc_root()),
            ("REQUEST_METHOD", request.method()),
            ("QUERY_STRING", request.query()),
            ("REQUEST_URI", request.query_path()),
            ("REMOTE_ADDR", &request.peer_addr().ip().to_string()),
            ("REMOTE_PORT", &request.peer_addr().port().to_string()),
            ("SERVER_ADDR", "127.0.0.1"),
            ("SERVER_PROTOCOL", "HTTP/1.1"),
            ("SERVER_PORT", &self.server_port.to_string()),
            ("SERVER_NAME", self.server_name),
            ("PATH_INFO", "")
        ];
        for (name, value) in params {
            let content = self.encode_name_value(name, value);
            self.write_record(&mut stream, FCGI_PARAMS, 1, &content)?;
        }
        for (name, value) in request.headers() {
            let content = self.encode_name_value(format!("HTTP_{}", name).as_str(), value);
            self.write_record(&mut stream, FCGI_PARAMS, 1, &content)?;
        }
        let content_len = request.headers()
                .iter()
                .find(|(name, _)| name.eq_ignore_ascii_case("content-length"));
        let content_type = request.headers()
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case("content-type"));
        let cookie = request.headers()
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case("cookie"));

        if content_type.is_some() {
            let content_type = content_type.unwrap().1;
            let content = self.encode_name_value("CONTENT_TYPE", content_type);
            self.write_record(&mut stream, FCGI_PARAMS, 1, &content)?;
        }
        if content_len.is_some() {
            let content_len = content_len.unwrap().1;
            let content = self.encode_name_value("CONTENT_LENGTH", content_len);
            self.write_record(&mut stream, FCGI_PARAMS, 1, &content)?;
        }
        if cookie.is_some() {
            let cookie = cookie.unwrap().1;
            let content = self.encode_name_value("HTTP_COOKIE", cookie);
            self.write_record(&mut stream, FCGI_PARAMS, 1, &content)?;
        }
        self.write_record(&mut stream, FCGI_PARAMS, 1, &[])?;

        if request.has_body() {
            loop {
                let mut buff = [0; 4 * 1024];
                let read_size = request.read_body(&mut buff).await?;
                if read_size == 0 {
                    break;
                }
                self.write_record(&mut stream, FCGI_STDIN, 1, &buff[..read_size])?;
            }
        }
        self.write_record(&mut stream, FCGI_STDIN, 1, &[])?;

        Ok(FcgiResponse::new(stream))
    }

    fn write_record(&self,
                    stream: &mut Box<dyn FcgiStream>,
                    record_type: u8,
                    request_id: u16,
                    content: &[u8]) -> Result<(), Box<dyn Error>> {
        let content_length = content.len() as u16;
        let padding_length = (8 - (content.len() % 8)) % 8;
        let header = vec![
            FCGI_VERSION,
            record_type,
            (request_id >> 8) as u8,
            request_id as u8,
            (content_length >> 8) as u8,
            content_length as u8,
            padding_length as u8,
            0,
        ];
        stream.write_all(&header)?;
        stream.write_all(content)?;
        if padding_length > 0 {
            stream.write_all(&vec![0u8; padding_length])?;
        }

        Ok(())
    }

    fn encode_name_value(&self, name: &str, value: &str) -> Vec<u8> {
        let mut out = Vec::new();
        self.write_len(name.len(), &mut out);
        self.write_len(value.len(), &mut out);
        out.extend_from_slice(name.as_bytes());
        out.extend_from_slice(value.as_bytes());
        out
    }

    fn write_len(&self, len: usize, out: &mut Vec<u8>) {
        if len < 128 {
            out.push(len as u8);
        } else {
            out.push(((len >> 24) | 0x80) as u8);
            out.push((len >> 16) as u8);
            out.push((len >> 8) as u8);
            out.push(len as u8);
        }
    }
}