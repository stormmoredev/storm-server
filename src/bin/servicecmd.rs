use std::path::PathBuf;
use std::time::Duration;
use storm_server::service::run_storm_service;

#[tokio::main]
async fn main() {
    tokio::spawn( async move {
        let _handlers = match run_storm_service(PathBuf::from("c:\\stormsrv")).await {
            Ok(h) => h,
            Err(e) => {
                println!("{}", e);
                return;
            }
        };
        loop {
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    });
    loop {
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}