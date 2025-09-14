use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::error::Error;
use std::path::PathBuf;
use std::sync::Arc;
use std::{fs, vec};
use storm_server::conf::Conf;
use storm_server::logger::Logger;
use storm_server::php::Php;
use storm_server::server::http_server::HttpServer;
use tokio::sync::watch::Sender;
use tokio::sync::{watch, Mutex};
use tokio::task::JoinHandle;

static COUNTER: Lazy<Arc<Mutex<i32>>> = Lazy::new(|| Arc::new(Mutex::new(0)));

async fn run_storm_service(dir: PathBuf) -> Result<Vec<(JoinHandle<i32>, Sender<bool>)>, Box<dyn Error>> {
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
    let conf_groups = get_server_confs(conf_dir).await?;
    for (port, confs) in conf_groups {
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
        let _ = server.run(logger, rx).await;

        let mut count = COUNTER.lock().await;
        *count += 1;
        *count
    });
    Ok((handle, shutdown_tx))
}

async fn get_server_confs(dir: PathBuf) -> Result<HashMap<u16, Vec<Conf>>, Box<dyn Error>> {
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
            if let Ok(conf) =  Conf::new(args) {
                configurations
                    .entry(conf.port)
                    .or_insert_with(Vec::new)
                    .push(conf);
            }
        }
    }
    Ok(configurations)
}

#[cfg(windows)]
fn main() -> windows_service::Result<()> {
    use crate::service::my_service_main;
    use windows_service::{define_windows_service, service_dispatcher};

    define_windows_service!(ffi_service_main, my_service_main);
    service_dispatcher::start("Storm Server Service", ffi_service_main)?;
    Ok(())
}

#[cfg(windows)]
pub mod service {
    use crate::run_storm_service;
    use std::ffi::OsString;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::runtime::Runtime;
    use tokio::sync::watch::Sender;
    use tokio::task::JoinHandle;
    use windows_service::service::{ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus, ServiceType};
    use windows_service::service_control_handler;
    use windows_service::service_control_handler::ServiceControlHandlerResult;

    pub fn my_service_main(_arguments: Vec<OsString>) {
        let running = Arc::new(AtomicBool::new(true));
        let running_clone = running.clone();

        let rt = Runtime::new().unwrap();

        let mut join_handlers: Vec<JoinHandle<i32>> = Vec::new();
        let mut senders: Vec<Sender<bool>> = Vec::new();
        let threats_info = rt.
            block_on(run_storm_service(PathBuf::from("c:\\stormsrv"))).
            unwrap_or_else(|_|Vec::new());
        for ti in threats_info {
            join_handlers.push(ti.0);
            senders.push(ti.1);
        }

        let status_handle = service_control_handler::register("Storm Service", move |control_event| {
            match control_event {
                ServiceControl::Stop => {
                    running_clone.store(false, Ordering::SeqCst);
                    for sender in senders.iter() {
                        let _ = sender.send(true);
                    }
                    ServiceControlHandlerResult::NoError
                }
                _ => ServiceControlHandlerResult::NotImplemented,
            }
        }).unwrap();

        status_handle.set_service_status(ServiceStatus {
            service_type: ServiceType::OWN_PROCESS,
            current_state: ServiceState::Running,
            controls_accepted: ServiceControlAccept::STOP,
            exit_code: ServiceExitCode::Win32(0),
            checkpoint: 0,
            wait_hint: Duration::default(),
            process_id: None
        }).unwrap();

        rt.block_on(async_main(running.clone(), join_handlers));

        status_handle.set_service_status(ServiceStatus {
            service_type: ServiceType::OWN_PROCESS,
            current_state: ServiceState::Stopped,
            controls_accepted: ServiceControlAccept::empty(),
            exit_code: ServiceExitCode::Win32(0),
            checkpoint: 0,
            wait_hint: Duration::default(),
            process_id: None,
        }).unwrap();
    }

    async fn async_main(running: Arc<AtomicBool>, join_handlers: Vec<JoinHandle<i32>>) {
        while running.load(Ordering::SeqCst) {
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
        for join_handle in join_handlers {
            let _ = join_handle.await;
        }
    }
}