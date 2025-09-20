use std::error::Error;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::{PathBuf};
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::watch::Receiver;
use tokio_rustls::TlsAcceptor;

use uuid::Uuid;
use crate::conf::Conf;
use request::Request;
use crate::logger::Logger;
use crate::server::endpoint_dispatcher::Dispatcher;
use crate::server::http_server::cert::build_tls_config;
use crate::server::http_server::response::Response;
use crate::server::http_stream::{HttpStream};
use crate::php::Php;
use crate::server::cache::Cache;
use crate::server::http_server::http_server_socket::HttpServerSocket;

pub mod request;
mod response;
mod cert;
pub mod http_server_socket;


pub struct HttpServer {
    hosts_configuration: Arc<Vec<Conf>>,
}

impl HttpServer {
    pub fn new(conf: Vec<Conf>) -> HttpServer {
        HttpServer {
            hosts_configuration: Arc::new(conf)
        }
    }

    pub async fn run(&self, server_logger: Logger,  mut rx: Receiver<bool>) -> Result<(), Box<dyn Error>> {
        if self.hosts_configuration.is_empty() {
            return Err("No hosts configuration found")?;
        }

        let conf  = self.hosts_configuration.get(0).unwrap();
        let port = conf.port;
        let tls_enabled = conf.https_enabled;
        for conf in self.hosts_configuration.iter() {
            if conf.port != port {
                return Err("All hosts must use identical port configurations")?;
            }
            if conf.https_enabled != tls_enabled {
                return Err("All hosts must use identical HTTPS configurations")?;
            }
        }

        let mut acceptor: Option<TlsAcceptor> = None;
        if tls_enabled {
            acceptor = match build_tls_config(&self.hosts_configuration) {
                Ok(c) => Some(TlsAcceptor::from(Arc::new(c))),
                Err(e) => return Err(format!("Could not build TLS config: {}", e).as_str())?
            };
        }

        let address = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port);
        let ip = address.ip();
        let listener = match TcpListener::bind(address).await {
            Ok(l) => l,
            Err(_) => return Err(format!("Could not bind to {}", address).as_str())?
        };
        let protocol = if conf.https_enabled { "Https" } else { "Http" };
        server_logger.log_i(format!("{} server listening on  {}:{}", protocol, ip, conf.port).as_str());

        let server_logger = Arc::new(server_logger);

        loop {
            let acceptor = acceptor.clone();
            let server_logger = server_logger.clone();
            let confs = self.hosts_configuration.clone();

            tokio::select! {
                result = listener.accept() => {
                    let (stream, addr) = match result {
                        Ok(r) => r,
                        Err(e) => {
                            server_logger.log_e(format!("Error accepting connection: {}", e).as_str());
                            break;
                        }
                    };
                    tokio::spawn(async move {
                        accept_request(addr, stream, tls_enabled, acceptor, confs, server_logger.clone()).await;
                    });
                }

                _ = rx.changed() => {
                    break;
                }
            }
        }
        Ok(())
    }
}

async fn accept_request(addr: SocketAddr,
                        stream: TcpStream,
                        tls_enabled: bool,
                        acceptor:Option<TlsAcceptor>,
                        confs: Arc<Vec<Conf>>,
                        server_logger: Arc<Logger>)
{
    let rw_stream = if tls_enabled {
        let acceptor = match acceptor {
            Some(a) => a,
            None => { return; }
        };
        match acceptor.accept(stream).await {
            Ok(s) => HttpServerSocket::Tls(s),
            Err(e) => {
                server_logger.log_e(format!("TLS accept error: {}", e).as_str());
                return;
            }
        }
    }
    else {
        HttpServerSocket::Plain(stream)
    };

    let http_stream = match HttpStream::new(rw_stream).await
    {
        Ok(stream) => stream,
        Err(e)  => {
            server_logger.log_e(format!("{}", e).as_str());
            return;
        }
    };

    let host = http_stream.headers.iter().find(|&x| x.0.eq_ignore_ascii_case("HOST"));
    let conf = match (host, confs.len()) {
        (None, n) if n > 1 => {
            server_logger.log_e("No Host header found");
            return;
        }
        (None, 1) => confs.get(0),
        (Some(_), 1) => confs.get(0),
        (Some(h), _) => {
            let conf = confs.iter().find(|x| x.domain.eq(h.1));
            if conf.is_none() {
                server_logger.log_e(format!("Host {} not found", h.1).as_str());
                return;
            }
            conf
        }
        _ => return,
    };

    let conf = conf.unwrap();
    let logger = Logger::new(conf.logs_dir.clone());
    let logger = Arc::new(logger);

    if conf.load_balancing_enabled {
        let dispatcher = Arc::new(Mutex::new(Dispatcher::new(&conf)));
        match dispatch_request(http_stream, dispatcher, conf).await {
            Ok(_) => server_logger.log_d("Request passed upstream successfully!"),
            Err(e) => server_logger.log_e(format!("Could not transfer stream. {}", e).as_str()),
        }
    } else {
        let _ = handle_request(http_stream, addr, logger, conf).await;
    }
}

async fn handle_request(
                http_stream: HttpStream,
                addr: SocketAddr,
                logger: Arc<Logger>,
                conf: &Conf) -> Result<(),Box<dyn Error>> {
    let mut request = match Request::new(http_stream, addr, &conf) {
        Ok(request) => request,
        Err(e) => {
            logger.log_e(format!("{}", e). as_str());
            return Err(e);
        }
    };
    let id = Uuid::new_v4();
    logger.log_i(format!("{}| Request {} {}", id, request.method(), request.query_path()).as_str());

    let req_path = request.path().to_string();
    let req_query_path = request.query_path().to_string();
    if let Ok(_) = Cache::try_serve_cached(request.stream_mut(), &req_path, &req_query_path, conf).await {
       logger.log_i(format!("{}| Request succeed [CACHE]", id).as_str());
       return Ok(());
   }

    let response = match create_response(&mut request, conf).await {
        Ok(response) => response,
        Err(e) => {
            logger.log_e(format!("{}", e).as_str());
            return Err(e);
        }
    };

    if let Err(e) = request.output_response(response, conf).await {
        logger.log_e(format!("{}| Request failed| {}", id, e).as_str());
        return Err(e);
    }

    logger.log_i(format!("{}| Request succeed", id).as_str());

    Ok(())
}

async fn create_response(request: &mut Request, conf: &Conf) -> Result<Response, Box<dyn Error>> {
    if request.file_path.is_file() {
        return get_file_path_response(request, conf).await;
    }
    if conf.php_index.is_some() {
        let path = PathBuf::from(conf.dir.as_str());
        let path = path.join(conf.php_index.as_ref().unwrap());
        if path.is_file() {
            request.rewrite(path);
            return get_file_path_response(request, conf).await;
        }
    }

    if conf.browsing_enabled && request.file_path.is_dir() {
        return Ok(Response::dir(&request.file_path, request.query_path(), &conf))
    }

    Ok(Response::not_found(&request.query_path()))
}

async fn get_file_path_response(request: &mut Request, conf: &Conf) -> Result<Response, Box<dyn Error>> {
    if let Some(ext) = request.file_path.extension() {
        if ext == "php" {
            let php = Php::new(&conf);
            return Response::php(request, php).await
        }
    }
    Ok(Response::file(&request.file_path))
}

async fn dispatch_request(mut downstream: HttpStream,
                          dispatcher: Arc<Mutex<Dispatcher>>,
                          conf: &Conf) -> Result<(), Box<dyn Error>> {
    let ds_path = downstream.path().to_string();
    let ds_query_path = downstream.query_path().to_string();
    if let Ok(_) = Cache::try_serve_cached(&mut downstream, &ds_path, &ds_query_path, conf).await {
        return Ok(());
    }
    let endpoint = match dispatcher.lock().unwrap().get() {
        Some(e) => e,
        None => return Err("No endpoint to handle request")?
    };

    let mut upstream = match TcpStream::connect(endpoint).await {
        Ok(stream) => stream,
        Err(_) => Err("Could not connect with upstream")?
    };

    upstream.write_all(&downstream.header_block()).await?;
    loop {
        let mut buff = [0; 4 * 1024];
        let read_size = downstream.read_body(&mut buff).await?;
        if read_size == 0 {
            break;
        }
        upstream.write_all(&buff[0..read_size]).await?;
        break;
    }

    let mut resp_buf: Vec<u8> = Vec::new();
    let mut headers_parsed = false;
    let mut cache_path: Option<PathBuf> = None;

    loop {
        let mut buff = [0; 1 * 1024];
        let read_size = upstream.read(&mut buff).await?;
        if read_size == 0 {
            break;
        }

        if conf.cache_enabled && !headers_parsed {
            resp_buf.extend_from_slice(&buff[..read_size]);
            if let Some(pos) = resp_buf.windows(4).position(|w| w == b"\r\n\r\n") {
                let header_end = pos + 4;
                let (header_bytes, _) = resp_buf.split_at(header_end);
                let header_str = String::from_utf8_lossy(header_bytes);
                let mut lines = header_str.lines();
                let first_line = lines.next().unwrap_or_default().to_string();
                let mut headers: Vec<(String, String)> = lines
                    .filter(|l| !l.is_empty())
                    .filter_map(|line| {
                        line.find(':').map(|idx| (
                            line[..idx].to_string(),
                            line[idx + 1..].trim().to_string(),
                        ))
                    })
                    .collect();
                cache_path = Cache::process_headers(&mut headers, conf);
                headers_parsed = true;

                let body = resp_buf[header_end..].to_vec();
                resp_buf.clear();
                resp_buf.extend_from_slice(first_line.as_bytes());
                resp_buf.extend_from_slice(b"\r\n");
                for (name, value) in headers {
                    let header_line = format!("{}: {}\r\n", name, value);
                    resp_buf.extend_from_slice(header_line.as_bytes());
                }
                resp_buf.extend_from_slice(b"\r\n");
                resp_buf.extend_from_slice(&body);

                downstream.write(&resp_buf).await?;
            }
        } else {
            downstream.write(&buff[0..read_size]).await?;
            if cache_path.is_some() {
                resp_buf.extend_from_slice(&buff[..read_size]);
            }
        }
    }
    if let Some(path) = cache_path {
        let _ = Cache::write(&resp_buf, &path);
    }

    Ok(())
}

