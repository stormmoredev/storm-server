use std::path::PathBuf;
use std::time::Duration;
use storm_server::service::run_storm_service;

#[tokio::main]
async fn main() {
    tokio::spawn( async move {
        if let Err(e) = run_storm_service(PathBuf::from("c:\\stormsrv")).await {
            eprintln!("Error: {}", e);
        }
    });
    loop {
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}