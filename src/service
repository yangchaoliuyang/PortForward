use std::process::Command;
use std::path::Path;
use std::io;
use tokio::sync::broadcast;
use crate::start_listen;
#[cfg(target_os = "windows")]
use windows_service::{
    service::{
        ServiceAccess, ServiceControl, ServiceControlAccept, ServiceErrorControl, ServiceExitCode,
        ServiceInfo, ServiceState, ServiceStatus, ServiceStartType, ServiceType,
    },
    service_control_handler::{self, ServiceControlHandlerResult},
    service_dispatcher,
    service_manager::{ServiceManager, ServiceManagerAccess},
};
#[cfg(target_os = "windows")]
use std::path::PathBuf;
#[cfg(target_os = "windows")]
use std::ffi::OsString;
#[cfg(target_os = "windows")]
use std::time::Duration;


#[cfg(target_os = "windows")]
use windows_service::define_windows_service;


#[cfg(target_os = "windows")]

define_windows_service!(ffi_service_main, service_main);


const SERVICE_NAME: &str = "PortForward";


const SERVICE_DISPLAY_NAME: &str = "Port Forwarding Service";


// main process for service
#[cfg(target_os = "windows")]
pub fn start_service() ->io::Result<()> {

    let _ = service_dispatcher::start(SERVICE_NAME, ffi_service_main);
    Ok(())
}

// main process for service
#[cfg(target_os = "windows")]
fn service_main(_:Vec<OsString>) -> windows_service::Result<()> {

    let (stop_sender,_) = broadcast::channel(1);

    let stop_sender2 = stop_sender.clone();

    let event_handler = move |control_event| -> ServiceControlHandlerResult {
        match control_event {
            ServiceControl::Stop | ServiceControl::Interrogate => {

                println!("receve stop signal from windows taskmanager");

                //sender stop signal to receivers
                let _ = stop_sender2.send(());
                
                ServiceControlHandlerResult::NoError
            }
            _ => ServiceControlHandlerResult::NotImplemented,
        }
    };

    // Register system service event handler
    let status_handle = service_control_handler::register(SERVICE_NAME, event_handler)?;

    let next_status = ServiceStatus {
        // Should match the one from system service registry
        service_type: ServiceType::OWN_PROCESS,
        // The new state
        current_state: ServiceState::Running,
        // Accept stop events when running
        controls_accepted: ServiceControlAccept::STOP,
        // Used to report an error when starting or stopping only, otherwise must be zero
        exit_code: ServiceExitCode::Win32(0),
        // Only used for pending states, otherwise must be zero
        checkpoint: 0,
        // Only used for pending states, otherwise must be zero
        wait_hint: Duration::default(),
        // Unused for setting status
        process_id: None,
    };

    // Tell the system that the service is running now
    status_handle.set_service_status(next_status)?;

    use tokio::runtime::Runtime;
    use tklog::error;



    // running async fuction as sync
    let rt = Runtime::new().unwrap();
    let result = rt.block_on(start_listen(stop_sender));
    if let  Err(e) =result  {
        {
            error!(e.to_string());
        }
    
    }
    
    let next_status2 = ServiceStatus {
        // Should match the one from system service registry
        service_type: ServiceType::OWN_PROCESS,
        // The new state
        current_state: ServiceState::Stopped,
        // Accept stop events when running
        controls_accepted: ServiceControlAccept::STOP,
        // Used to report an error when starting or stopping only, otherwise must be zero
        exit_code: ServiceExitCode::Win32(0),
        // Only used for pending states, otherwise must be zero
        checkpoint: 0,
        // Only used for pending states, otherwise must be zero
        wait_hint: Duration::default(),
        // Unused for setting status
        process_id: None,
    };

    let _ = status_handle.set_service_status(next_status2);

    Ok(())
}


// install as service
#[cfg(target_os = "windows")]
pub fn install(config_path: String, log_path: String) -> windows_service::Result<()> {
    let manager_access = ServiceManagerAccess::CONNECT | ServiceManagerAccess::CREATE_SERVICE;
    let service_manager = ServiceManager::local_computer(None::<&str>, manager_access)?;

    let service_binary_path = ::std::env::current_exe()
        .unwrap()
        .with_file_name("PortForward.exe");

    let mut launch_arguments = Vec::new();
    
    launch_arguments.push(OsString::from("--daemon"));
    launch_arguments.push(OsString::from("service"));
  
    launch_arguments.push(OsString::from("--config"));
    launch_arguments.push(OsString::from(config_path));
    
    
   
    launch_arguments.push(OsString::from("--log"));
    launch_arguments.push(OsString::from(log_path));
    

    let service_info = ServiceInfo {
        name: OsString::from(SERVICE_NAME),
        display_name: OsString::from(SERVICE_DISPLAY_NAME),
        service_type: ServiceType::OWN_PROCESS,
        start_type: ServiceStartType::AutoStart,
        error_control: ServiceErrorControl::Normal,
        executable_path: service_binary_path,
        launch_arguments,
        dependencies: vec![],
        account_name: None,
        account_password: None,
    };

    let _service = service_manager.create_service(
        &service_info,
        ServiceAccess::CHANGE_CONFIG | ServiceAccess::START,
    )?;

    println!("Service installed successfully with parameters");

    Ok(())
}


// uninstall from service
#[cfg(target_os = "windows")]
pub fn uninstall() -> windows_service::Result<()> {
    let manager_access = ServiceManagerAccess::CONNECT;
    let service_manager = ServiceManager::local_computer(None::<&str>, manager_access)?;

    let service_access = ServiceAccess::QUERY_STATUS | ServiceAccess::STOP | ServiceAccess::DELETE;
    let service = service_manager.open_service(SERVICE_NAME, service_access)?;

    let service_status = service.query_status()?;
    if service_status.current_state != ServiceState::Stopped {
        service.stop()?;
        while let Ok(status) = service.query_status() {
            if status.current_state == ServiceState::Stopped {
                break;
            }
            std::thread::sleep(Duration::from_secs(1));
        }
    }
    service.delete()?;
    println!("Service uninstalled successfully");
    Ok(())
}


#[cfg(not(target_os = "windows"))]

pub fn install_linux(config_path: String, log_path: String) -> io::Result<()>{


    // Get current executable path
    let service_binary_path = ::std::env::current_exe()
        .unwrap()
        .with_file_name("PortForward");

    // Create service file content
    let service_content = format!(
        "[Unit]
Description={}
After=network.target

[Service]
Type=simple
ExecStart={} --config {} --log {}
Restart=always
RestartSec=5s

[Install]
WantedBy=multi-user.target
",SERVICE_DISPLAY_NAME,service_binary_path.to_str().unwrap(),config_path, log_path
    );

    // Define service file path
    let service_file_path = format!("/etc/systemd/system/{}.service", SERVICE_NAME);

    // Write service file
    use std::io::Write;
    let mut file = std::fs::File::create(&service_file_path)?;
    file.write_all(service_content.as_bytes())?;

    // Reload systemd
    Command::new("systemctl")
        .args(["daemon-reload"])
        .status()?;

    // Enable the service
    Command::new("systemctl")
        .args(["enable", SERVICE_NAME])
        .status()?;

    println!("Successfully installed service . You can start it with: systemctl start {}", SERVICE_NAME);

    Ok(())
}

#[cfg(not(target_os = "windows"))]


pub fn uninstall_linux() -> io::Result<()> {

    // Stop the service if running
    let _ = Command::new("systemctl")
        .args(["stop", SERVICE_NAME])
        .status()?;

    // Disable the service
    Command::new("systemctl")
        .args(["disable", SERVICE_NAME])
        .status()?;

    // Remove service file
    let service_file_path = format!("/etc/systemd/system/{}.service", SERVICE_NAME);
    if Path::new(&service_file_path).exists() {
        std::fs::remove_file(&service_file_path)?;
    }

    // Reload systemd
    Command::new("systemctl")
        .args(["daemon-reload"])
        .status()?;

    println!("Successfully uninstalled service {}",SERVICE_NAME);

    Ok(())
}
