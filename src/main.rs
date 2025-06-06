use tokio::net::{TcpListener, TcpStream};
use tokio::io::copy_bidirectional;
use tokio::io::AsyncReadExt;
use tokio::fs::File;
use tokio::task::JoinSet;

use tokio::sync::broadcast;

use std::io;
use std::env;
use std::path::Path;
use std::process::Command;

use serde::Deserialize;
use clap::Parser;
use clap::Subcommand;

use tklog::{
    async_error, async_info,  LEVEL, Format, ASYNC_LOG,LOG,info
};


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


#[derive(Parser, Debug)]
#[command(version = "1.0.1")]
struct Args {

    /// run or service
    #[arg(short, long,default_value="app")]
    daemon: String,

    /// config path
    #[arg(short, long,default_value="config.toml")]
    config: String,

    /// log path
    #[arg(short, long,default_value="PortForward.log")]
    log: String,


    /// Subcommand to execute
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Install the application as service
    Install {
    },
    /// Uninstall the application from service
    Uninstall {
    },

}


#[derive(Deserialize)]
struct Forward {
    name:String,
    local_addr: String,
    remote_addr: String,
}

#[derive(Deserialize)]
struct Config {
    forwards: Vec<Forward>,
}


#[tokio::main]
async fn main() -> io::Result<()> {


    let mut args = Args::parse();

    #[cfg(target_os = "windows")]
    let default_path = Path::new("D:\\").to_path_buf();

    #[cfg(not(target_os = "windows"))]

    let default_path = Path::new("./").to_path_buf();
   
    let  app_path= match env::current_exe() {
        Ok(exe_path) => {
            
            if let Some(parent) = exe_path.parent() {
                parent.to_path_buf()
            } else {

                default_path

            }
            
        },
        Err(e) => 
        {
            async_info!("Failed to get current exe path: ",e.to_string());

            default_path
        }
    };

    
    if args.config == "config.toml".to_string(){
        args.config = app_path.join("config.toml").to_str().unwrap().to_string();

    }
    if args.log == "PortForward.log".to_string(){
        args.log = app_path.join("PortForward.log").to_str().unwrap().to_string();
    }
 
    async_log_init(args.log.to_string()).await;
    log_init(args.log.to_string());


    
    async_info!("Run type is: ",args.daemon);
    async_info!("App path is: ",app_path.to_str().unwrap().to_string());
    async_info!("Config path is: ",args.config);
    async_info!("Log path is: ",args.log);


    match args.command {
        Some(Commands::Install {}) => {

            #[cfg(target_os = "windows")]
            let _ = install(args.config,args.log);


            #[cfg(not(target_os = "windows"))]
            let _ = install_linux(args.config,args.log);
        },
        Some(Commands::Uninstall {}) => {

            #[cfg(target_os = "windows")]
            let _ = uninstall();

            #[cfg(not(target_os = "windows"))]

            let _ = uninstall_linux();
        },
        None => {


            if args.daemon=="service"
            {
                

                async_info!("run as service");

                
                #[cfg(target_os = "windows")]
                let _ = service_dispatcher::start(SERVICE_NAME, ffi_service_main);


            }
            else {
                
            
                // command run mode
                println!("Running with config: {}, log: {}", args.config, args.log);


                let (stop_sender, _) = broadcast::channel(1);

                if let Err(e) = start_listen(args.config,stop_sender).await{
                    async_error!(e.to_string());
                }

                
            }
        }
    }

    Ok(())
}


async fn async_log_init(log_path:String) {
    // Configure global singleton

    ASYNC_LOG
        .set_console(true) // Disable console output
        .set_level(LEVEL::Trace) // Set log level to Trace
        .set_format(Format::LevelFlag | Format::Date |Format::Time | Format::ShortFileName) // Define structured logging output
        .set_cutmode_by_size(&log_path, 1_000_000, 10, false) // Rotate log files by size, every 10,000 bytes, with 10 backups
        .await;
}

fn log_init(log_path:String) {
    // Configure global singleton

    LOG
        .set_console(true) // Disable console output
        .set_level(LEVEL::Trace) // Set log level to Trace
        .set_format(Format::LevelFlag | Format::Date |Format::Time | Format::ShortFileName) // Define structured logging output
        .set_cutmode_by_size(&log_path, 1_000_000, 10, false); // Rotate log files by size, every 10,000 bytes, with 10 backups
        
}


// sync functions for sockets

async fn handle_client(mut local: TcpStream, remote_addr: String)  -> io::Result<()>{


    let mut remote = TcpStream::connect(remote_addr).await?;

    let _ = copy_bidirectional(&mut local,&mut remote).await;

    

    Ok(())
    
}

async  fn listening(listener: TcpListener,name : String,remote_addr: String, mut  stop_receiver:  tokio::sync::broadcast::Receiver<()>)  -> io::Result<()>{

    loop {
        async_info!("Start listening loop for ",name);
        tokio::select! {
            // normal work
            _ = async {
                
                let (socket, addr) = listener.accept().await?;

                async_info!( "[ ",name," ] receive connection from ",addr.ip().to_string());
        
                let remote  = remote_addr.clone();
                
                tokio::spawn(handle_client(socket, remote));

                Ok::<(), std::io::Error>(()) 
                
            } =>{},
            
            
            // stop signal
            _ = stop_receiver.recv() => {
                async_info!("Worker received stop signal for ",name);
                return Ok(())
            }
        }
    }

}

async  fn start_listen(config:String,  stop_sender:tokio::sync::broadcast::Sender<()>) ->io::Result<()>{

    // read config
    let mut config_content = File::open(config).await?;
    let mut content_string = String::from("");
    let _ = config_content.read_to_string(&mut content_string).await?;



    let result: Result<Config, toml::de::Error> = toml::from_str(&content_string);
    match   result{
        Ok(config)=>
        {

        

            let mut set = JoinSet::new();
            for forward in config.forwards {

                async_info!("[",forward.name,"] from ",forward.local_addr," to ",forward.remote_addr);
        
                let listener: TcpListener = TcpListener::bind(forward.local_addr).await?;

                let  stop_reveiver =  stop_sender.subscribe();

                set.spawn(listening(listener,forward.name,forward.remote_addr, stop_reveiver));
                
            }
            set.join_all().await;

        }
        Err(_)=>{
            async_error!("Extract config.toml failure");
        
        }
        
    }; 

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

                info!("receve stop signal from windows taskmanager");

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

    let args = Args::parse();

    // running async fuction as sync
    let rt = Runtime::new().unwrap();
    let result = rt.block_on(start_listen(args.config,stop_sender));
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

    info!("Service installed successfully with parameters");

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
    info!("Service uninstalled successfully");
    Ok(())
}


#[cfg(not(target_os = "windows"))]

fn install_linux(config_path: String, log_path: String) -> io::Result<()>{


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

    info!("Successfully installed service . You can start it with: systemctl start ", SERVICE_NAME);

    Ok(())
}

#[cfg(not(target_os = "windows"))]


fn uninstall_linux() -> io::Result<()> {

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

    info!("Successfully uninstalled service ",SERVICE_NAME);

    Ok(())
}
