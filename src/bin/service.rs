use std::error::Error;



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
    use storm_server::service::run_storm_service;
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