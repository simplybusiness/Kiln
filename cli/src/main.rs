use clap::{App, AppSettings, Arg, SubCommand};
use futures::prelude::Future;
use shiplift::{builder::PullOptions, Docker, builder::LogsOptions, tty::StreamType};
use tokio::prelude::*;
use std::fs::{File};
use std::boxed::Box;
use serde::{Deserialize};
use std::fmt::{self, Debug};
use std::error::Error;
use std::process; 

#[derive(Debug)]
#[derive(Deserialize)]
enum ScanEnv { 
    Local, 
    CI,
} 

impl fmt::Display for ScanEnv {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

#[derive(Debug)]
#[derive(Deserialize)]
struct CliConfigOptions{ 
    data_collector_url: Option<String>, 
    app_name: Option<String>, 
    scan_env: Option<ScanEnv>,
} 

#[derive(Debug)]
pub struct ConfigFileError {
        pub error_message: String,
        pub toml_field_name: String,
}
 
impl Error for ConfigFileError { } 

impl fmt::Display for ConfigFileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Error in kiln.toml config file: {} ({})", self.error_message, self.toml_field_name)
    }
}

impl ConfigFileError {
    pub fn app_name_unspecified() -> ConfigFileError {
        ConfigFileError {
            error_message: "Field unspecified".into(),
            toml_field_name: "app_name".to_string(),
        }
    }
    pub fn app_name_empty() -> ConfigFileError {
        ConfigFileError {
            error_message: "Field left empty".into(),
            toml_field_name: "app_name".to_string(),
        }
    }
    pub fn data_collector_url_empty() -> ConfigFileError {
        ConfigFileError {
            error_message: "Field left empty".into(),
            toml_field_name: "data_collector_url".to_string(),
        }
    }
    pub fn data_collector_url_unspecified() -> ConfigFileError {
        ConfigFileError {
            error_message: "Field left unspecified".into(),
            toml_field_name: "data_collector_url".to_string(),
        }
    }
}

fn main() {
    let matches = App::new("Kiln CLI")
        .setting(AppSettings::SubcommandRequired)
        .arg(Arg::with_name("use-local-image")
            .long("use-local-image")
            .help("Do not try and pull the latest version of a tool image. Useful for development and scanning without network access"))
        .subcommand(SubCommand::with_name("ruby")
            .about("perform security testing of Ruby based projects")
            .setting(AppSettings::SubcommandRequired)
            .subcommand(SubCommand::with_name("dependencies")
                .about("Use Bundler-audit to find known vulnerabilities in project dependencies")
            )
        ).get_matches();

    let use_local_image = matches.is_present("use-local-image");
    let test_tool_image_name = "kiln/bundler-audit:latest"; 
    let test_tool_name = String::from("bundler-audit-kiln-container"); 
    
    let mut env_vec = Vec::new(); 
    let mut env_app_name = "APP_NAME=".to_string();
    let mut env_scan_env= "SCAN_ENV=".to_string();
    let mut env_df_url = "DATA_COLLECTOR_URL=".to_string();
    
    match parse_kiln_toml_file() { 
        Err(e) => { 
            println!("{}", e); 
            process::exit(1);
        },
        Ok(config_info) =>  { 
            env_app_name.push_str((config_info.app_name.unwrap()).as_ref()); 
            
            match config_info.scan_env { 
                Some(scan_env) => 
                    env_scan_env.push_str((scan_env).to_string().as_ref()),
                None => 
                    env_scan_env.push_str("Local".to_string().as_ref()),
            };

            env_df_url.push_str((config_info.data_collector_url.unwrap()).as_ref()); 
                    
            env_vec.push(env_df_url.as_ref());
            env_vec.push(env_app_name.as_ref());
            env_vec.push(env_scan_env.as_ref());
        } 
    };

    match matches.subcommand() {
        ("ruby", Some(sub_m)) => {
            match sub_m.subcommand_name() {
                Some("dependencies") => {
                    let prep_fut = prepare_tool_image(test_tool_image_name.to_owned(), use_local_image);
                    tokio::run(prep_fut);

                    let path = std::env::current_dir().unwrap().to_str().unwrap().to_string() + ":" + "/code";
                    let mut path_vec = Vec::new();
                    path_vec.push(path.as_ref());

                    let docker = Docker::new();
                    let tool_container_future = docker
                        .containers()
                        .create(
                            &shiplift::ContainerOptions::builder(test_tool_image_name)
                            .name(&test_tool_name)
                            .attach_stdout(true)
                            .attach_stderr(true)
                            .auto_remove(true)
                            .volumes(path_vec)
                            .env(env_vec)
                            .build(),)
                        .map_err(|e| eprintln!("Error: {}", e))
                        .and_then(|container| { 
                            let docker = Docker::new();
                            docker
                                .containers()
                                .get(&container.id)
                                .start()
                                .map_err(|e| eprintln!("Error: {}", e))
                        })
                    .and_then(move |_|{
                        let docker = Docker::new();
                        let log_future = docker
                            .containers()
                            .get(&test_tool_name)
                            .logs(&LogsOptions::builder().stdout(true).stderr(true).follow(true).build())
                            .for_each(|chunk| {
                                match chunk.stream_type {
                                    StreamType::StdOut => println!("{}", chunk.as_string().unwrap()),
                                    StreamType::StdErr => eprintln!("{}", chunk.as_string().unwrap()),
                                    StreamType::StdIn => (),
                                }
                                Ok(())
                            })
                        .map_err(|e| eprintln!("Error: {}", e));
                    tokio::spawn(log_future);
                    Ok(())
                    });
                    tokio::run(tool_container_future); 
                },
                _ => unreachable!()
            }
        },
        _ => unreachable!()
    };
}

fn parse_kiln_toml_file() -> Result<CliConfigOptions,ConfigFileError> {  
    /* Read default kiln config file */
    let kiln_config_file_name = std::env::current_dir().unwrap().to_str().unwrap().to_string() + "/kiln.toml";
    let mut kiln_config_file = match File::open(kiln_config_file_name) { 
        Ok(f) => f, 
        Err(e) => { 
            println!("Error occured while opening the kiln.toml file. Please ensure you have this in your current working directory (Err: {})", e); 
            process::exit(1);
        } 
    }; 
    
    let mut config_file_str = String::new();
    match kiln_config_file.read_to_string(&mut config_file_str) { 
        Ok(_s) => {
            let config_info: CliConfigOptions = toml::from_str(config_file_str.as_ref()).unwrap(); 
            validate_config_info(&config_info)?;
            Ok(config_info)
        },
        Err(e) => { 
            println!("Error reading kiln.toml file (Err: {})", e); 
            process::exit(1);
        } 
    }
}

fn validate_config_info(config_info: &CliConfigOptions) -> Result<(), ConfigFileError> {
    match &config_info.app_name {
        Some(app_name) =>  {
            if app_name.is_empty() {
                Err(ConfigFileError::app_name_empty())?
            }
        }
        None => Err(ConfigFileError::app_name_unspecified())?
    }; 
    match &config_info.data_collector_url {
        Some(url) => 
            if url.is_empty() { 
                Err(ConfigFileError::data_collector_url_empty())?
            } 
        None => Err(ConfigFileError::data_collector_url_unspecified())?
    };
                   
    Ok(())
} 

fn prepare_tool_image<T>(tool_image_name: T, use_local_image: bool) -> Box<dyn Future<Item=(), Error=()> + Send + 'static> 
where T: AsRef<str> + std::fmt::Display + Send + 'static {
    let docker = Docker::new();
    if use_local_image {
        return Box::new(
            docker.images()
            .get(tool_image_name.as_ref())
            .inspect()
            .then(move |res| {
                match res {
                    Ok(_) => futures::future::ok(()),
                    Err(err) => {
                        match &err {
                            shiplift::errors::Error::Fault{code, message: _} if *code == 404 => eprintln!("Could not find {} locally. Quitting!", tool_image_name),
                            _  => eprintln!("{}", err)
                        };
                        futures::future::err(())
                    }
                }
            })
        );
    } else {
        let pull_options = PullOptions::builder()
            .image(tool_image_name.as_ref())
            .build();

        return Box::new(
            docker.images()
            .pull(&pull_options)
            .inspect(|item| println!("{}", item["status"].as_str().unwrap()))
            .collect()
            .then(move |res| {
                match res {
                    Ok(_) => {
                        futures::future::ok(())
                    },
                    Err(err) => {
                        match &err {
                            shiplift::errors::Error::Fault{code, message: _} if *code == 404 => eprintln!("Could not find {} on Docker Hub. Quitting!", tool_image_name),
                            _  => eprintln!("{}", err)
                        };
                        futures::future::err(())
                    }
                }
            })
        );
    }
}


