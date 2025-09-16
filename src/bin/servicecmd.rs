use std::path::PathBuf;
use tokio::task::JoinHandle;
use std::time::Duration;
use storm_server::service::run_storm_service;

#[tokio::main]
async fn main() {
    tokio::spawn( async move {
        let mut handlers;
        match run_storm_service(PathBuf::from("c:\\stormsrv")).await {
            Ok(h) => handlers = h,
            Err(e) => {
                println!("{}", e);
            }
        }
        loop {
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    });
    loop {
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}