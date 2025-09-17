use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use once_cell::sync::Lazy;
use tokio::sync::{watch, Mutex};
use tokio::sync::watch::Sender;
use tokio::task::JoinHandle;
use crate::conf::Conf;
use crate::logger::Logger;
use crate::php::Php;
use crate::server::http_server::HttpServer;

static COUNTER: Lazy<Arc<Mutex<i32>>> = Lazy::new(|| Arc::new(Mutex::new(0)));

pub async fn run_storm_service(dir: PathBuf) -> Result<Vec<(JoinHandle<i32>, Sender<bool>)>, Box<dyn Error>> {
    Php::init_fast_cgi();
    let mut senders:Vec<(JoinHandle<i32>, Sender<bool>)> = vec!();
    let logs_dir = dir.join("logs");
    let conf_dir = dir.join("conf");
    if !logs_dir.exists() {
        fs::create_dir_all(&logs_dir)?;
    }
    if !conf_dir.exists() {
        fs::create_dir_all(&conf_dir)?;
    }
    let logger = Logger::new(Some(logs_dir));
    logger.log_i("Service is running");
    let conf_groups = get_server_confs(conf_dir, logger.clone()).await?;
    for (_, confs) in conf_groups {
        if let Ok(sender) = run_http_server(confs ,logger.clone()).await {
            senders.push(sender);
        }
    }
    Ok(senders)
}

async fn run_http_server(confs: Vec<Conf>, logger: Logger) -> Result<(JoinHandle<i32>, Sender<bool>), Box<dyn Error>> {
    let (shutdown_tx,  rx) = watch::channel(false);
    let handle: JoinHandle<i32> = tokio::spawn( async move {
        let server = HttpServer::new(confs);
        match server.run(logger.clone(), rx).await {
            Err(e) => {
                logger.log_e(format!("Server error: {}", e).as_str());
            }
            _ => { }
        }

        let mut count = COUNTER.lock().await;
        *count += 1;
        *count
    });
    Ok((handle, shutdown_tx))
}

async fn get_server_confs(dir: PathBuf, logger: Logger) -> Result<HashMap<u16, Vec<Conf>>, Box<dyn Error>> {
    let mut configurations:HashMap<u16, Vec<Conf>> = HashMap::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() && path.extension().is_some_and(|p| p == "conf") {
            let path = &path.to_string_lossy();
            let args: Vec<String> = vec!["", "-f", path].
                iter().
                map(|x| x.to_string()).
                collect();
            match  Conf::new(args) {
                Ok(conf) => {
                    configurations
                        .entry(conf.port)
                        .or_insert_with(Vec::new)
                        .push(conf);
                }
                Err(e) => {
                    logger.log_e(
                        format!("Error in configuration file {}: {}", path, e).as_str(),
                    );
                }
            }
        }
    }
    Ok(configurations)
}