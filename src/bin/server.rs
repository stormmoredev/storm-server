use std::{env, process};
use storm_server::conf::Conf;
use storm_server::logger::Logger;
use storm_server::php::Php;
use storm_server::server::http_server::HttpServer;
use tokio::sync::watch;

#[tokio::main]
async fn main() {
    let args = env::args().collect();
    let conf = match Conf::new(args) {
        Ok(conf) => conf,
        Err(error) => {
            eprintln!("Configuration error: {}", error);
            process::exit(1);
        }
    };

    Php::init_fast_cgi();

    let confs = vec![conf];
    let (shutdown_tx, rx) = watch::channel(false);
    let srv = HttpServer::new(confs);
    let logger = Logger::new(None);
    if let Err(e) = srv.run(logger, rx).await {
        eprintln!("Server error: {}", e);
    }
}
