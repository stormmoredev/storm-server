use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio_rustls::server::TlsStream;

pub enum HttpServerSocket {
    Plain(TcpStream),
    Tls(TlsStream<TcpStream>),
}

impl HttpServerSocket {
    pub async fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            HttpServerSocket::Plain(s) => s.read(buf).await,
            HttpServerSocket::Tls(s) => s.read(buf).await,
        }
    }

    pub async fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()> {
        match self {
            HttpServerSocket::Plain(s) => s.write_all(buf).await,
            HttpServerSocket::Tls(s) => s.write_all(buf).await,
        }
    }
}