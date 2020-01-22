use clap::{App, AppSettings, Arg, SubCommand};
use futures::prelude::Future;
use shiplift::{builder::PullOptions, Docker, builder::LogsOptions, tty::StreamType};
use tokio::prelude::*;
use std::fs::{File};
use std::boxed::Box;
use serde::{Deserialize};
use serde_json::{Value};
use std::fmt::{self, Debug};
use std::error::Error;
use std::process; 
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::sync::mpsc;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use std::collections::HashMap;
use reqwest::blocking::Client;
use reqwest::Method;
use std::collections::HashSet;


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
    let test_tool_image_name = "kiln/bundler-audit"; 
    let tool_image_tag = "master-latest";
    let test_tool_name = String::from("bundler-audit-kiln-container"); 

    let mut env_vec = Vec::new(); 
    let mut env_app_name = "APP_NAME=".to_string();
    let mut env_scan_env= "SCAN_ENV=".to_string();
    let mut env_df_url = "DATA_COLLECTOR_URL=".to_string();

    match parse_kiln_toml_file() { 
        Err(e) => { 
            eprintln!("{}", e); 
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

                    let prep_fut = prepare_tool_image(test_tool_image_name.to_owned(), tool_image_tag.to_owned(),use_local_image);
                    tokio::run(prep_fut);

                    let path = std::env::current_dir().unwrap().to_str().unwrap().to_string() + ":" + "/code";
                    let mut path_vec = Vec::new();
                    path_vec.push(path.as_ref());

                    let docker = Docker::new();
                    let tool_image_name_full =format!("{}:{}",test_tool_image_name, tool_image_tag); 
                    let tool_container_future = docker
                        .containers()
                        .create(
                            &shiplift::ContainerOptions::builder(&tool_image_name_full)
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
            eprintln!("Error occured while opening the kiln.toml file. Please ensure you have this in your current working directory (Err: {})", e); 
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
            eprintln!("Error reading kiln.toml file (Err: {})", e); 
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

static DOCKER_AUTH_URL: &'static str=
"https://auth.docker.io/token?service=registry.docker.io&scope=repository"; 

static DOCKER_REGISTRY_URL: &'static str=
"https://registry.hub.docker.com"; 

pub fn get_fs_layers_for_docker_image(repo_name: String, tag: String) -> Result<HashSet<String>, reqwest::Error>{
    let client = Client::new();

    let docker_auth_url = format!("{}:{}:pull",DOCKER_AUTH_URL, repo_name);
    let req = client.request(Method::GET, &docker_auth_url)
        .build()?;
    let resp = client.execute(req)?;
    let resp_body: Value = resp.json()?;
    let token = resp_body["token"].as_str().unwrap();  

    let docker_manifest_url = format!("{}/v2/{}/manifests/{}",DOCKER_REGISTRY_URL,repo_name, tag);
    let manifest_req = client.request(Method::GET, &docker_manifest_url)
        .bearer_auth(token)
        .build()?;
    let manifest_resp = client.execute(manifest_req)?;
    let manifest_resp_body:Value = manifest_resp.json()?;

    let layers = manifest_resp_body["fsLayers"]
        .as_array()
        .unwrap()
        .iter()
        .map(|hashval| {
            let hashstr: String = hashval["blobSum"].as_str().unwrap().to_string(); 
            let v:Vec<&str> = hashstr.split(":").collect(); 
            (&(v[1].to_string())[..12]).to_string()
        })
    .collect::<HashSet<_>>();
    Ok(layers)
} 


struct ProgressBarDisplay { 
    prog_channels : HashMap<std::string::String, Arc<Mutex<mpsc::Sender<serde_json::value::Value>>>>, 
    multibar_arc : Arc<MultiProgress>,
    pull_started : bool, 
}

static PBAR_FMT: &'static str =
"{msg} {percent}% [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} eta: {eta}";

impl ProgressBarDisplay { 
    fn create_progress_bar(len: u64) -> ProgressBar {
        let progbar = ProgressBar::new(len);

        progbar.set_style(
            ProgressStyle::default_bar()
            .template(PBAR_FMT)
            .progress_chars("=> "),
        );

        progbar
    }

    pub fn new() -> ProgressBarDisplay {
        ProgressBarDisplay {
            multibar_arc: Arc::new(MultiProgress::new()),
            prog_channels: HashMap::new(),
            pull_started:false
        }
    } 

    pub fn create_threads_for_progress_bars(&mut self, layers: HashSet<String>)  { 
        for layer in layers { 
            let pgbar = self.multibar_arc.add(Self::create_progress_bar(10));

            let (sender, receiver) = mpsc::channel();
            let sender = Arc::new(Mutex::new(sender));
            self.prog_channels.insert(layer.clone().to_string(),Arc::clone(&sender));
            thread::spawn(move || {
                let mut status_val = "".to_string(); 
                let mut bar_length = 0; 
                loop { 
                    let output_val = receiver.recv();
                    match output_val { 
                        Err(_e) => break,
                        Ok(val) => {
                            if val["status"] != serde_json::Value::Null {
                                if (val["status"].as_str().unwrap().to_string() == "Pull complete") || 
                                    (val["status"].as_str().unwrap().to_string() == "Already exists")    
                                { 
                                    pgbar.finish_and_clear();
                                    break;
                                } 
                                if val["status"].as_str().unwrap().to_string() != status_val { 
                                    status_val = val["status"].as_str().unwrap().to_string();
                                    if val["id"] != serde_json::Value::Null {
                                        pgbar.set_message([val["id"].as_str().unwrap().to_string(),":".to_string(),status_val.clone()].concat().as_ref());
                                    }
                                }
                            }
                            if val["progressDetail"] != serde_json::Value::Null {
                                if val["progressDetail"]["current"] != serde_json::Value::Null { 
                                    if val["progressDetail"]["total"] != serde_json::Value::Null { 
                                        let curr_count = (val["progressDetail"]["current"]).as_u64().unwrap();
                                        let total_count = (val["progressDetail"]["total"]).as_u64().unwrap();
                                        if bar_length < total_count { 
                                            pgbar.set_length(total_count);
                                            pgbar.set_position(curr_count);
                                            bar_length = total_count;
                                        } 
                                        if curr_count < total_count {
                                            pgbar.set_position(curr_count);
                                        }
                                        if curr_count >= total_count { 
                                            pgbar.set_length(0);
                                            pgbar.set_position(0);
                                            bar_length = 0;
                                        } 
                                    }
                                }
                            }
                        }
                    } 
                } 
            });
        }

        let multibar_arc_clone = self.multibar_arc.clone();
        thread::spawn(move || {
            multibar_arc_clone.join().unwrap();
        });
    }

    pub fn update_progress_bar(&mut self, output : serde_json::Value) -> () { 
        if output["id"] != serde_json::Value::Null {
            if output["status"] != serde_json::Value::Null {
                if self.pull_started == true { 
                    let status = output["status"].as_str().unwrap().to_string(); 
                    let id = output["id"].as_str().unwrap().to_string();
                    match self.prog_channels.get(&id) {
                        Some(trx) => 
                            trx.lock().unwrap().send(output).unwrap(), 
                        None => { 
                            eprintln!("Error: Cannot find channel for sending progress update message in kiln-cli for id {} and status {}",id, status); 
                        },
                    }
                } else { 
                    println!("{}",output["status"].as_str().unwrap().to_string());      
                    self.pull_started = true;
                } 
            }
        } else if output["status"] != serde_json::Value::Null { 
            println!("{}",output["status"].as_str().unwrap().to_string());      
        }
    } 
} 

fn prepare_tool_image(tool_image_name: String, tool_image_tag: String, use_local_image: bool) -> Box<dyn Future<Item=(), Error=()> + Send + 'static> {
    let docker = Docker::new();
    let tool_image_name_full = format!("{}:{}", tool_image_name, tool_image_tag); 
    if use_local_image {
        return Box::new(
            docker.images()
            .get(tool_image_name_full.as_ref())
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
            .image(&tool_image_name_full)
            .build();

        let layers = get_fs_layers_for_docker_image(tool_image_name, tool_image_tag);
        let mut prog_bar_disp: Option<ProgressBarDisplay> =  match layers { 
            Ok(fslayers) => {  
                let mut p = ProgressBarDisplay::new(); 
                p.create_threads_for_progress_bars(fslayers); 
                Some(p)
            },
            Err(e) =>  { 
                eprintln!("Error: Unable to get fs layers for tool image {}",e); 
                None 
            },
        };

        return Box::new(
            docker.images()
            .pull(&pull_options)
            .for_each(move |output| {
                if prog_bar_disp.is_some() { 
                    prog_bar_disp.as_mut().unwrap().update_progress_bar(output);
                }
                Ok(())
            })
            .then(move |res| {
                match res {
                    Ok(_) => {
                        Ok(())
                    },
                    Err(err) => {
                        match &err {
                            shiplift::errors::Error::Fault{code, message: _} if *code == 404 => eprintln!("Could not find {} on Docker Hub. Quitting!", tool_image_name_full),
                            _  => eprintln!("{}", err)
                        };
                        Err(())
                    }
                }
            })
            )
    }
}


