use tokio::net::{TcpListener, TcpStream};
use tokio::io::{ AsyncReadExt, AsyncWriteExt};
use tokio::fs::File;
use tokio::task::JoinSet;
use tokio::sync::broadcast;




use std::io;
use std::path::Path;
use std::env;



use serde::Deserialize;
use clap::Parser;
use clap::Subcommand;

use tklog::{
    async_error, async_info,  LEVEL, Format, ASYNC_LOG,LOG,info
};

mod encryption;
use encryption::{SimpleEncryptionContext, encrypt_and_prepend_length};

mod buffer;
use buffer::PacketBuffer;

mod service;

#[cfg(target_os = "windows")]
use service::{start_service, install, uninstall};


#[cfg(not(target_os = "windows"))]
use service::{install_linux, uninstall_linux};


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


#[derive(Deserialize, Clone)]
struct Forward {
    name:String,
    local_addr: String,
    remote_addr: String,
    local_encryption: bool,
    remote_encryption: bool,
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
                let _ = start_service();


            }
            else {
                
            
                // command run mode
                async_info!("Run as app, Running with config: ", args.config,"log: ", args.log);


                let (stop_sender, _) = broadcast::channel(1);


                if let Err(e) = start_listen(stop_sender).await{
                    
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





// 使用缓冲区的版本
async fn handle_client_buffered(
    forward: Forward,
    mut local: TcpStream, 
) -> io::Result<()> {
    async_info!("[ ",forward.name," ] Connect remote addr:",forward.remote_addr);
    let mut remote = TcpStream::connect(forward.remote_addr).await?;
    
    let ctx = if forward.local_encryption || forward.remote_encryption {
        Some(SimpleEncryptionContext::new())
    } else {
        None
    };
    
    let (mut local_reader, mut local_writer) = local.split();
    let (mut remote_reader, mut remote_writer) = remote.split();
    
    let mut local_buffer = PacketBuffer::new();
    let mut remote_buffer = PacketBuffer::new();
    
    // 从本地到远程的流量处理
    let client_to_server = async {
        let mut read_buffer: Vec<u8> = vec![0u8; 4096];
        loop {
            // 读取数据到缓冲区
            let n: usize = local_reader.read(&mut read_buffer).await?;
            
            if n == 0 {
                break;
            }
            
            if forward.local_encryption {

                local_buffer.push_data(&read_buffer[..n]);

                
                // 处理所有完整的数据包
                while let Some(decrypted_data)= local_buffer.try_read_packet(ctx.as_ref().unwrap())? {
                    let processed_data: Vec<u8> = if forward.remote_encryption {
                        encrypt_and_prepend_length(&decrypted_data, ctx.as_ref().unwrap()).await?
                    } else {
                        decrypted_data
                    };
                    
                    remote_writer.write_all(&processed_data).await?;
                }
            } else {
                // 非加密模式直接读取，远程需要加密就加密后再发
                let processed_data: Vec<u8> = if forward.remote_encryption {
                    encrypt_and_prepend_length(&read_buffer[..n], ctx.as_ref().unwrap()).await?
                } else {
                    read_buffer[..n].to_vec()
                };
                remote_writer.write_all(&processed_data).await?;
            }
        }
        Ok::<(), io::Error>(())
    };
    
    // 从远程到本地的流量处理（类似逻辑）
    let server_to_client = async {
        let mut read_buffer = vec![0u8; 4096];
        loop {
            let n: usize = remote_reader.read(&mut read_buffer).await?;
            if n == 0 {
                break;
            }
            
            if forward.remote_encryption {
                remote_buffer.push_data(&read_buffer[..n]);
                
                while let Some(decrypted_data) = remote_buffer.try_read_packet(ctx.as_ref().unwrap())? {
                    let processed_data = if forward.local_encryption {
                        encrypt_and_prepend_length(&decrypted_data, ctx.as_ref().unwrap()).await?
                    } else {
                        decrypted_data
                    };
                    
                    local_writer.write_all(&processed_data).await?;
                }
            } else {

                let processed_data = if forward.local_encryption {
                    encrypt_and_prepend_length(&read_buffer[..n], ctx.as_ref().unwrap()).await?
                } else {
                    read_buffer[..n].to_vec()
                };
                local_writer.write_all(&processed_data).await?;
            }
        }
        Ok(())
    };
    
    tokio::try_join!(client_to_server, server_to_client)?;
    Ok(())
}


async  fn listening(listener: TcpListener, forward :Forward, mut  stop_receiver:  tokio::sync::broadcast::Receiver<()>)  -> io::Result<()>{

    loop {

        let fw = forward.clone();
        async_info!("[ ",forward.name," ] Start listening loop for ");
        tokio::select! {
            // normal work
            _ = async {
                
                let (socket, addr) = listener.accept().await?;
                async_info!( "[ ",fw.name," ] receive connection from ",addr.ip().to_string());
                //tokio::spawn(handle_client(socket, remote));
                tokio::spawn(handle_client_buffered(fw, socket));
                Ok::<(), std::io::Error>(()) 
                
            } =>{},
            
            
            // stop signal
            _ = stop_receiver.recv() => {
                async_info!("[ ",forward.name," ] Worker received stop signal for ");
                return Ok(())
            }
        }
    }

}

pub async  fn start_listen(stop_sender:tokio::sync::broadcast::Sender<()>) ->io::Result<()>{

    // read config
    let args = Args::parse();
    async_info!("Start reading confg file");
    let mut config_content = File::open(args.config).await?;
    let mut content_string = String::from("");
    let _ = config_content.read_to_string(&mut content_string).await?;
    
    async_info!("Start extract confg file");
    let result: Result<Config, toml::de::Error> = toml::from_str(&content_string);
    match   result{
        Ok(config)=>
        {

        

            let mut set = JoinSet::new();
            for forward in config.forwards {

                async_info!("[ ",forward.name," ] from ",forward.local_addr," to ",forward.remote_addr," local encryption ",forward.local_encryption," remote encryption ",forward.remote_encryption);
                
                let fw = forward.clone();

                let listener: TcpListener = TcpListener::bind(forward.local_addr).await?;

                let  stop_reveiver =  stop_sender.subscribe();
                
                set.spawn(listening(listener,fw, stop_reveiver));
                
            }
            set.join_all().await;

        }
        Err(_)=>{
            async_error!("Extract config.toml failure");
        
        }
        
    }; 

    Ok(())

}




