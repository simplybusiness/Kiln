use bollard::{
    container::{
        self, CreateContainerOptions, CreateContainerResults, HostConfig, LogOutput, LogsOptions,
        MountPoint, StartContainerOptions, WaitContainerOptions,
    },
    image::{
        CreateImageOptions, CreateImageProgressDetail, CreateImageResults, ListImagesOptions,
        RemoveImageOptions,
    },
    Docker,
};
use clap::{App, AppSettings, Arg, SubCommand};
use failure::err_msg;
use futures::stream::StreamExt;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use regex::Regex;
use reqwest::Client;
use reqwest::Method;
use serde::Deserialize;
use serde_json::Value;
use std::boxed::Box;
use std::collections::HashMap;
use std::collections::HashSet;
use std::error::Error;
use std::fmt::{self, Debug};
use std::fs::File;
use std::io::Read;
use std::process;
use std::sync::mpsc;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;

#[derive(Debug, Deserialize)]
enum ScanEnv {
    Local,
    CI,
}

impl fmt::Display for ScanEnv {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

#[derive(Debug, Default, Deserialize)]
struct CliConfigOptions {
    data_collector_url: Option<String>,
    app_name: Option<String>,
    scan_env: Option<ScanEnv>,
    tool_image_name: Option<String>,
}

#[derive(Debug)]
pub struct ConfigFileError {
    pub error_message: String,
    pub toml_field_name: String,
}

impl Error for ConfigFileError {}

impl fmt::Display for ConfigFileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Error in kiln.toml config file: {} ({})",
            self.error_message, self.toml_field_name
        )
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let matches = App::new("Kiln CLI")
        .setting(AppSettings::SubcommandRequired)
        .version(clap::crate_version!())
        .arg(Arg::with_name("offline").long("offline").help(
            "Do not make any network requests to pull images or update scanning databases etc",
        ))
        .arg(
            Arg::with_name("tool-image-name")
                .long("tool-image-name")
                .takes_value(true)
                .help("Override the default docker image and tag for a tool."),
        )
        .arg(
            Arg::with_name("work-dir")
                .long("work-dir")
                .takes_value(true)
                .help("Path to be scanned. Defaults to current directory."),
        )
        .subcommand(
            SubCommand::with_name("ruby")
                .about("perform security testing of Ruby based projects")
                .setting(AppSettings::SubcommandRequired)
                .subcommand(SubCommand::with_name("dependencies").about(
                    "Use Bundler-audit to find known vulnerabilities in project dependencies",
                )),
        )
        .get_matches();

    let offline = matches.is_present("offline");

    let mut env_vec = Vec::new();
    let mut env_app_name = "APP_NAME=".to_string();
    let mut env_scan_env = "SCAN_ENV=".to_string();
    let mut env_df_url = "DATA_COLLECTOR_URL=".to_string();
    let env_offline = format!("OFFLINE={}", offline);

    match parse_kiln_toml_file() {
        Err(e) => {
            eprintln!("{}", e);
            process::exit(1);
        }
        Ok(config_info) => {
            env_app_name.push_str((config_info.app_name.unwrap()).as_ref());

            match config_info.scan_env {
                Some(scan_env) => env_scan_env.push_str((scan_env).to_string().as_ref()),
                None => env_scan_env.push_str("Local".to_string().as_ref()),
            };

            env_df_url.push_str((config_info.data_collector_url.unwrap()).as_ref());

            env_vec.push(env_df_url);
            env_vec.push(env_app_name);
            env_vec.push(env_scan_env);
            env_vec.push(env_offline);
        }
    };

    let docker = Docker::connect_with_local_defaults()?;

    match matches.subcommand() {
        ("ruby", Some(sub_m)) => match sub_m.subcommand_name() {
            Some("dependencies") => {
                let tool_image = match matches.value_of("tool-image-name") {
                    None => {
                        let tag = get_tag_for_image("kiln".into(), "bundler-audit".into())
                            .await
                            .expect("Could not get tag for bundler-audit image");
                        format!("kiln/bundler-audit:{}", tag)
                    }
                    Some(name) => name.into(),
                };
                let image_name_regex = Regex::new(r#"(?:(?P<r>[a-zA-Z0-9_-]+)/)?(?P<i>[a-zA-Z0-9_-]+)(?::(?P<t>[a-zA-Z0-9_.-]+))?"#).unwrap();
                let image_name_matches = image_name_regex.captures(&tool_image).expect(
                    "Error parsing tool image name, ensure name is in format REPO/IMAGE:TAG",
                );
                let tool_image_repo = image_name_matches
                    .name("r")
                    .map(|capture| capture.as_str())
                    .unwrap_or_else(|| "kiln");
                let tool_image_name = image_name_matches
                    .name("i")
                    .map(|capture| capture.as_str())
                    .unwrap_or_else(|| "bundler-audit");
                let tool_image_tag = image_name_matches
                    .name("t")
                    .map(|capture| capture.as_str())
                    .unwrap_or_else(|| "git-latest");

                let mut image_filters = HashMap::new();
                let reference_filter = format!("{}/{}", tool_image_repo, tool_image_name);
                image_filters.insert("reference", vec![reference_filter.as_str()]);
                let list_image_options = Some(ListImagesOptions {
                    filters: image_filters,
                    ..Default::default()
                });

                let pre_pull_images = docker.list_images(list_image_options.clone()).await?;

                prepare_tool_image(
                    tool_image_repo.to_owned(),
                    tool_image_name.to_owned(),
                    tool_image_tag.to_owned(),
                    offline,
                )
                .await?;

                let post_pull_images = docker.list_images(list_image_options).await?;

                let images_to_delete: Vec<_> = post_pull_images
                    .iter()
                    .filter(|item| {
                        pre_pull_images
                            .iter()
                            .any(|other_item| other_item.id == item.id)
                    })
                    .filter(|item| {
                        !item
                            .repo_tags
                            .as_ref()
                            .map(|tags| tags.iter().any(|tag| tag.as_str().contains("latest")))
                            .unwrap_or(false)
                    })
                    .map(|item| item.id.clone())
                    .collect();

                for item in images_to_delete.iter() {
                    if docker
                        .remove_image(item, None::<RemoveImageOptions>, None)
                        .await
                        .is_err()
                    {
                        eprintln!("Warning: Error occured while trying to clean up old Kiln tool images for {}/{}", tool_image_repo, tool_image_name);
                    }
                }

                let tool_image_name_full =
                    format!("{}/{}:{}", tool_image_repo, tool_image_name, tool_image_tag);

                let container_work_dir = matches.value_of("work-dir")
                    .map(|path| std::path::PathBuf::from(path))
                    .or_else(|| std::env::current_dir().ok())
                    .map(|path| path.to_str().unwrap().to_string())
                    .expect("Work directory not provided and current directory either does not exist or we do not have permission to access. EXITING!");

                let container_config = container::Config {
                    attach_stdout: Some(true),
                    attach_stderr: Some(true),
                    tty: Some(true),
                    image: Some(tool_image_name_full),
                    env: Some(env_vec),
                    host_config: Some(HostConfig {
                        auto_remove: Some(true),
                        mounts: Some(vec![MountPoint {
                            target: "/code".to_string(),
                            source: container_work_dir,
                            type_: "bind".to_string(),
                            ..Default::default()
                        }]),
                        ..Default::default()
                    }),
                    ..Default::default()
                };

                let create_container_result = docker
                    .create_container(None::<CreateContainerOptions<String>>, container_config)
                    .await;
                match &create_container_result {
                    Err(err) => {
                        eprintln!("Error creating tool container: {}", err);
                        panic!();
                    }
                    Ok(CreateContainerResults { warnings, .. }) if warnings.is_some() => {
                        warnings.as_ref().unwrap().iter().for_each(|item| {
                            println!("Warning occured while creating tool container: {}", item);
                        });
                    }
                    _ => (),
                };

                let container_id = create_container_result.unwrap().id;

                let container_start_result = docker
                    .start_container(&container_id, None::<StartContainerOptions<String>>)
                    .await;
                if let Err(err) = container_start_result {
                    eprintln!("Error start tool container: {}", err);
                    panic!();
                }

                let mut container_result =
                    docker.wait_container(&container_id, None::<WaitContainerOptions<String>>);

                let logs_options = Some(LogsOptions {
                    follow: true,
                    stdout: true,
                    stderr: true,
                    tail: "all".to_string(),
                    ..Default::default()
                });
                let mut logs_stream = docker.logs(&container_id, logs_options).fuse();

                loop {
                    if logs_stream.is_done() {
                        break;
                    }
                    let log_line = logs_stream.next().await;
                    if let Some(log_line) = log_line {
                        match log_line {
                            Ok(LogOutput::StdOut { message }) => println!("{}", message),
                            Ok(LogOutput::Console { message }) => println!("{}", message),
                            Ok(LogOutput::StdErr { message }) => eprintln!("{}", message),
                            Err(err) => eprintln!("Error getting tool logs: {}", err),
                            _ => (),
                        }
                    }
                }

                let container_exit_details = container_result.next().await.unwrap()?;
                if container_exit_details.status_code != 0 {
                    eprintln!(
                        "Tool container exited with code {}, {}",
                        container_exit_details.status_code,
                        container_exit_details
                            .error
                            .map(|e| e.message)
                            .unwrap_or_else(|| "No error message".to_string())
                    )
                }
            }
            _ => unreachable!(),
        },
        _ => unreachable!(),
    };
    Ok(())
}

fn parse_kiln_toml_file() -> Result<CliConfigOptions, ConfigFileError> {
    /* Read default kiln config file */
    let kiln_config_file_name = std::env::current_dir()
        .unwrap()
        .to_str()
        .unwrap()
        .to_string()
        + "/kiln.toml";
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
        }
        Err(e) => {
            eprintln!("Error reading kiln.toml file (Err: {})", e);
            process::exit(1);
        }
    }
}

fn validate_config_info(config_info: &CliConfigOptions) -> Result<(), ConfigFileError> {
    match &config_info.app_name {
        Some(app_name) => {
            if app_name.is_empty() {
                return Err(ConfigFileError::app_name_empty());
            }
        }
        None => return Err(ConfigFileError::app_name_unspecified()),
    };
    match &config_info.data_collector_url {
        Some(url) => {
            if url.is_empty() {
                return Err(ConfigFileError::data_collector_url_empty());
            }
        }
        None => return Err(ConfigFileError::data_collector_url_unspecified()),
    };

    Ok(())
}

static DOCKER_AUTH_URL: &str =
    "https://auth.docker.io/token?service=registry.docker.io&scope=repository";

static DOCKER_REGISTRY_URL: &str = "https://registry.hub.docker.com";

// This layer of indirection exists because I want to add support for using the latest semver
// compatible tag for a tool image when running a release build, but default to git-latest when
// running a debug build. This was planned for 0.2.0, but turned out to be more complex than I
// initially expected and I decided it shouldn't block the release
pub async fn get_tag_for_image(
    _repo_name: String,
    _image_name: String,
) -> Result<String, reqwest::Error> {
    if cfg!(debug_assertions) {
        Ok("git-latest".into())
    } else {
        Ok(env!("CARGO_PKG_VERSION").into())
    }
}

pub async fn get_fs_layers_for_docker_image(
    repo_name: String,
    image_name: String,
    tag: String,
) -> Result<HashSet<String>, reqwest::Error> {
    let client = Client::new();

    let docker_auth_url = format!("{}:{}/{}:pull", DOCKER_AUTH_URL, repo_name, image_name);
    let req = client.request(Method::GET, &docker_auth_url).build()?;
    let resp = client.execute(req).await?;
    let resp_body: Value = resp.json().await?;
    let token = resp_body["token"].as_str().unwrap();

    let docker_manifest_url = format!(
        "{}/v2/{}/{}/manifests/{}",
        DOCKER_REGISTRY_URL, repo_name, image_name, tag
    );
    let manifest_req = client
        .request(Method::GET, &docker_manifest_url)
        .bearer_auth(token)
        .build()?;
    let manifest_resp = client
        .execute(manifest_req)
        .await?
        .error_for_status()
        .expect(&format!(
            "Could not get information about docker image {}/{}:{}. Check that image exists",
            repo_name, image_name, tag
        ));
    let manifest_resp_body: Value = manifest_resp.json().await?;

    let layers = manifest_resp_body["fsLayers"]
        .as_array()
        .unwrap()
        .iter()
        .map(|hashval| {
            let hashstr: String = hashval["blobSum"].as_str().unwrap().to_string();
            let v: Vec<&str> = hashstr.split(':').collect();
            (&(v[1].to_string())[..12]).to_string()
        })
        .collect::<HashSet<_>>();
    Ok(layers)
}

type ProgressChannelUpdate = (String, Option<CreateImageProgressDetail>, String);

struct ProgressBarDisplay {
    prog_channels: HashMap<std::string::String, Arc<Mutex<mpsc::Sender<ProgressChannelUpdate>>>>,
    multibar_arc: Arc<MultiProgress>,
    pull_started: bool,
}

static PBAR_FMT: &str = "{msg} {percent}% [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} eta: {eta}";

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
            pull_started: false,
        }
    }

    pub fn create_threads_for_progress_bars(&mut self, layers: HashSet<String>) {
        for layer in layers {
            let pgbar = self.multibar_arc.add(Self::create_progress_bar(10));

            let (sender, receiver) = mpsc::channel();
            let sender = Arc::new(Mutex::new(sender));
            self.prog_channels
                .insert(layer.clone().to_string(), Arc::clone(&sender));
            thread::spawn(move || loop {
                let output_val = receiver.recv();
                match output_val {
                    Err(_e) => break,
                    Ok(update) => {
                        let (id, progress_detail, status) = update;
                        if status == "Pull complete" || status == "Already exists" {
                            pgbar.finish();
                        }
                        pgbar.set_message(format!("{}:{}", id, status).as_ref());
                        let total = progress_detail.map(|pd| pd.total).flatten();
                        let current = progress_detail.map(|pd| pd.current).flatten();
                        if let (Some(total), Some(current)) = (total, current) {
                            pgbar.set_length(total);
                            pgbar.set_position(current);
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

    pub fn update_progress_bar(
        &mut self,
        id: Option<String>,
        progress_detail: Option<CreateImageProgressDetail>,
        status: String,
    ) {
        if let Some(id) = id {
            if self.pull_started {
                match self.prog_channels.get(&id) {
                    Some(tx) => tx.lock().unwrap().send((id, progress_detail, status)).unwrap(),
                    None => eprintln!("Error: Cannot find channel for sending progress update message in kiln-cli for id {} and status {}", id, status)
                }
            } else {
                println!("{}", status);
                self.pull_started = true;
            }
        } else {
            println!("{}", status);
        }
    }
}

async fn prepare_tool_image(
    tool_image_repo: String,
    tool_image_name: String,
    tool_image_tag: String,
    offline: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let docker = Docker::connect_with_local_defaults()?;
    let tool_image_name_full =
        format!("{}/{}:{}", tool_image_repo, tool_image_name, tool_image_tag);

    if offline {
        let mut filters = HashMap::new();
        filters.insert("reference", vec![tool_image_name_full.as_ref()]);
        let options = Some(ListImagesOptions {
            filters,
            ..Default::default()
        });
        let images = docker.list_images(options).await?;
        if images.is_empty() {
            Err(err_msg(format!("Could not find {} locally.", tool_image_name_full)).into())
        } else {
            Ok(())
        }
    } else {
        let create_image_options = Some(CreateImageOptions {
            from_image: format!("{}/{}", tool_image_repo, tool_image_name),
            tag: tool_image_tag.clone(),
            ..Default::default()
        });

        let layers =
            get_fs_layers_for_docker_image(tool_image_repo, tool_image_name, tool_image_tag).await;
        let mut prog_bar_disp: Option<ProgressBarDisplay> = match layers {
            Ok(fslayers) => {
                let mut p = ProgressBarDisplay::new();
                p.create_threads_for_progress_bars(fslayers);
                Some(p)
            }
            Err(e) => {
                eprintln!("Error: Unable to get fs layers for tool image {}", e);
                None
            }
        };

        let mut status_stream = docker.create_image(create_image_options, None, None).fuse();
        loop {
            let item = status_stream.next().await;
            if item.is_none() {
                break;
            }
            match item.unwrap() {
                Ok(CreateImageResults::CreateImageProgressResponse {
                    status,
                    progress_detail,
                    id,
                    ..
                }) => {
                    if let Some(prog_bar_disp) = prog_bar_disp.as_mut() {
                        prog_bar_disp.update_progress_bar(id, progress_detail, status);
                    }
                    Ok(())
                }
                Ok(CreateImageResults::CreateImageError { error, .. }) => Err(error),
                Err(err) => Err(err.to_string()),
            }?
        }
        Ok(())
    }
}
