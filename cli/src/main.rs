use base64;
use bollard::{
    auth::DockerCredentials as BollardDockerCredentials,
    container::{
        self, CreateContainerOptions, LogOutput, LogsOptions, StartContainerOptions,
        WaitContainerOptions,
    },
    image::{CreateImageOptions, ListImagesOptions, RemoveImageOptions},
    models::{BuildInfo, HostConfig},
    service::{ContainerCreateResponse, Mount, MountTypeEnum, ProgressDetail},
    Docker,
};
use clap::{App, AppSettings, Arg, SubCommand};
use failure::err_msg;
use futures::stream::StreamExt;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use path_clean::PathClean;

#[cfg(target_os = "linux")]
use bollard::container::InspectContainerOptions;
#[cfg(target_os = "linux")]
use procfs::process::Process;

use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(test)] {
        use mockall::mock;
        use self::tests::MockClient as Client;
    } else {
        use reqwest::Client;
    }
}

use reqwest::{Request, StatusCode};
use serde::Deserialize;
use serde_json::Value;
use std::boxed::Box;
use std::collections::HashMap;
use std::collections::HashSet;
use std::convert::{TryInto, TryFrom};
use std::error::Error;
use std::fmt::{self, Debug, Display};
use std::fs::File;
use std::io::Read;
use std::process;
use std::sync::mpsc;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use url::Url;

static DOCKER_AUTH_URL: &str =
    "https://auth.docker.io/token?service=registry.docker.io&scope=repository";

static DOCKER_REGISTRY_URL: &str = "https://registry.hub.docker.com";

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

enum Tool {
    BundlerAudit,
}

#[derive(Clone, Debug, Default)]
pub struct DockerImage {
    registry: Option<Url>,
    repo: String,
    image: String,
    tag: String,
    credentials: Option<DockerCredentials>,
}

impl DockerImage {
    fn new() -> DockerImageBuilder {
        DockerImageBuilder {
            ..Default::default()
        }
    }

    fn manifest_url(&self) -> Url {
        let mut url = self
            .registry
            .clone()
            .unwrap_or_else(|| Url::parse(DOCKER_REGISTRY_URL).unwrap());
        url.set_path(format!("/v2/{}/{}/manifests/{}", self.repo, self.image, self.tag).as_ref());
        url
    }
}

impl Display for DockerImage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.registry.as_ref() {
            Some(registry) => {
                match registry.port() {
                    Some(port) => write!(f, "{}:{}/{}/{}:{}", registry.host().unwrap(), port, self.repo, self.image, self.tag),
                    None => write!(f, "{}/{}/{}:{}", registry.host().unwrap(), self.repo, self.image, self.tag),
                }
            }
            None => write!(f, "{}/{}:{}", self.repo, self.image, self.tag),
        }
    }
}

impl From<&DockerImage> for CreateImageOptions<String> {
    fn from(image: &DockerImage) -> Self {
        let image_name = match image.registry.as_ref() {
            Some(registry) => format!("{}/{}/{}", registry.host().unwrap(), image.repo, image.image),
            None => format!("{}/{}", image.repo, image.image),
        };
        CreateImageOptions {
            from_image: image_name,
            tag: image.tag.clone(),
            ..Default::default()
        }
    }
}

impl From<&DockerImage> for Option<BollardDockerCredentials> {
    fn from(image: &DockerImage) -> Self {
        image
            .credentials
            .clone()
            .map(|creds| BollardDockerCredentials {
                username: Some(creds.username),
                password: Some(creds.password),
                serveraddress: image.registry.clone().map(|url| url.to_string()),
                ..Default::default()
            })
    }
}

#[derive(Clone, Debug)]
struct DockerCredentials {
    username: String,
    password: String,
}

#[derive(Clone, Debug, Default)]
struct DockerImageBuilder {
    registry: Option<String>,
    repo: Option<String>,
    image: Option<String>,
    tag: Option<String>,
    credentials: Option<DockerCredentials>,
}

impl DockerImageBuilder {
    fn with_registry<S: Into<String>>(&mut self, registry_url: S) -> &mut DockerImageBuilder {
        self.registry = Some(registry_url.into());
        self
    }

    fn with_repo<S: Into<String>>(&mut self, repo: S) -> &mut DockerImageBuilder {
        self.repo = Some(repo.into());
        self
    }

    fn with_image<S: Into<String>>(&mut self, image: S) -> &mut DockerImageBuilder {
        self.image = Some(image.into());
        self
    }

    fn with_tag<S: Into<String>>(&mut self, tag: S) -> &mut DockerImageBuilder {
        self.tag = Some(tag.into());
        self
    }

    fn with_credentials(&mut self, credentials: DockerCredentials) -> &mut DockerImageBuilder {
        self.credentials = Some(credentials);
        self
    }

    async fn build(self) -> Result<DockerImage, Box<dyn Error>> {
        let parsed_registry = match self.registry {
            None => Ok(None),
            Some(registry) => Url::parse(&registry).and_then(|url| Ok(Some(url))),
        }?;

        let repo = self.repo.ok_or_else(|| {
            err_msg("Tried to build a DockerImage but image repo wasn't provided")
        })?;

        let image = self.image.ok_or_else(|| {
            err_msg("Tried to build a DockerImage but image name wasn't provided")
        })?;

        let default_tag = get_tag_for_image(repo.clone(), image.clone()).await?;
        let tag = self.tag.unwrap_or(default_tag);

        Ok(DockerImage {
            registry: parsed_registry,
            repo,
            image,
            tag,
            credentials: self.credentials,
        })
    }
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

    let tool_work_dir = matches.value_of("work-dir")
        .map(|path| std::path::PathBuf::from(path).clean())
        .or_else(|| std::env::current_dir().ok())
        .map(|path| path.to_str().unwrap().to_string())
        .expect("Work directory not provided and current directory either does not exist or we do not have permission to access. EXITING!");
    let mapped_tool_work_dir = get_mapped_tool_work_dir(&tool_work_dir)
        .await
        .unwrap_or(tool_work_dir.clone());

    let offline = matches.is_present("offline");

    let env_var_scan_env = std::env::var("KILN_SCAN_ENV").ok();

    let mut env_vec = Vec::new();
    let mut env_app_name = "APP_NAME=".to_string();
    let mut env_scan_env = "SCAN_ENV=".to_string();
    let mut env_df_url = "DATA_COLLECTOR_URL=".to_string();
    let env_offline = format!("OFFLINE={}", offline);

    match parse_kiln_toml_file(&mapped_tool_work_dir) {
        Err(e) => {
            eprintln!("{}", e);
            process::exit(1);
        }
        Ok(mut config_info) => {
            env_app_name.push_str((config_info.app_name.take().unwrap()).as_ref());

            let scan_env_val = env_var_scan_env.unwrap_or_else(|| {
                config_info
                    .scan_env
                    .take()
                    .map_or_else(|| "Local".to_string(), |x| x.to_string())
            });
            env_scan_env.push_str(&scan_env_val);

            env_df_url.push_str((config_info.data_collector_url.unwrap()).as_ref());

            env_vec.push(env_df_url);
            env_vec.push(env_app_name);
            env_vec.push(env_scan_env);
            env_vec.push(env_offline);
        }
    };

    let docker = Docker::connect_with_local_defaults()?;
    let client = Client::new();

    let env_var_docker_registry_username = std::env::var("KILN_DOCKER_USERNAME").ok();
    let env_var_docker_registry_password = std::env::var("KILN_DOCKER_PASSWORD").ok();
    let env_var_docker_registry_registry = std::env::var("KILN_DOCKER_REGISTRY").ok(); //Registry and Repo

    let mut image_builder = DockerImage::new();

    match (
        env_var_docker_registry_username,
        env_var_docker_registry_password,
    ) {
        (Some(username), Some(password)) => {
            image_builder.with_credentials(DockerCredentials { username, password });
        }
        (None, None) => (),
        _ => panic!("Provided only one of username and password"),
    };

    match env_var_docker_registry_registry {
        Some(registry) => {
            let mut url = Url::parse(&registry)?;
            url
                .host_str()
                .expect("Docker registry does not contain a valid hostname");
            let path = url.path().trim_start_matches("/");
            if !path.is_empty() {
                image_builder.with_repo(path);
            } else {
                image_builder.with_repo("kiln");
            }
            url.set_path("");
            image_builder.with_registry(url.to_string());
        }
        None => {
            image_builder.with_repo("kiln");
        }
    };

    let (tool, custom_image_name) = match matches.subcommand() {
        ("ruby", Some(sub_m)) => match sub_m.subcommand_name() {
            Some("dependencies") => (Tool::BundlerAudit, matches.value_of("tool-image-name")), //Image and tag
            _ => unreachable!(),
        },
        _ => unreachable!(),
    };

    match custom_image_name {
        Some(image_name) => {
            let parts: Vec<&str> = image_name.splitn(2, ":").collect();
            image_builder.with_image(parts[0]);
            if parts.len() == 2 {
                image_builder.with_tag(parts[1]);
            }
        }
        None => {
            match tool {
                Tool::BundlerAudit => {
                    image_builder.with_image("bundler-audit");
                }
            };
        }
    };

    let docker_image = image_builder.build().await?;

    let mut image_filters = HashMap::new();
    let reference_filter = match docker_image.registry.as_ref() {
        Some(registry) => format!("{}/{}/{}", registry, docker_image.repo, docker_image.image),
        None => format!("{}/{}", docker_image.repo, docker_image.image),
    };

    image_filters.insert("reference", vec![reference_filter.as_str()]);
    let list_image_options = Some(ListImagesOptions {
        filters: image_filters,
        ..Default::default()
    });

    let pre_pull_images = docker.list_images(list_image_options.clone()).await?;

    prepare_tool_image(&client, &docker_image, offline).await?;

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
                .iter()
                .any(|tag| tag.as_str().contains("git-latest") || tag.as_str().contains("latest"))
        })
        .map(|item| item.id.clone())
        .collect();

    for item in images_to_delete.iter() {
        if docker
            .remove_image(item, None::<RemoveImageOptions>, None)
            .await
            .is_err()
        {
            eprintln!(
                "Warning: Error occured while trying to clean up old Kiln tool images for {}",
                reference_filter
            );
        }
    }

    let container_config = container::Config {
        attach_stdout: Some(true),
        attach_stderr: Some(true),
        tty: Some(true),
        image: Some(docker_image.to_string()),
        env: Some(env_vec),
        host_config: Some(HostConfig {
            auto_remove: Some(true),
            mounts: Some(vec![Mount {
                target: Some("/code".to_string()),
                source: Some(tool_work_dir),
                typ: Some(MountTypeEnum::BIND),
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
        Ok(ContainerCreateResponse { warnings, .. }) if !warnings.is_empty() => {
            warnings.iter().for_each(|item| {
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
                Ok(LogOutput::StdOut { message }) => {
                    println!("{}", String::from_utf8_lossy(&message))
                }
                Ok(LogOutput::Console { message }) => {
                    println!("{}", String::from_utf8_lossy(&message))
                }
                Ok(LogOutput::StdErr { message }) => {
                    eprintln!("{}", String::from_utf8_lossy(&message))
                }
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
                .and_then(|e| e.message)
                .unwrap_or_else(|| "No error message".to_string())
        )
    }
    Ok(())
}

fn parse_kiln_toml_file(tool_work_dir: &str) -> Result<CliConfigOptions, ConfigFileError> {
    /* Read default kiln config file */
    let kiln_config_file_name =
        std::path::PathBuf::from(tool_work_dir.to_owned()).join("kiln.toml");
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
    client: &Client,
    docker_image: &DockerImage,
) -> Result<HashSet<String>, Box<dyn Error>> {
    let token = if docker_image.registry.is_none() {
        let docker_auth_url = format!(
            "{}:{}/{}:pull",
            DOCKER_AUTH_URL, docker_image.repo, docker_image.image
        );
        let req = http::Request::get(&docker_auth_url)
            .body("")
            .map(|req| Request::try_from(req))??;
        let resp = client.execute(req).await?;
        let resp_body: Value = resp.json().await?;
        Some(resp_body["token"].as_str().unwrap().to_owned())
    } else {
        None
    };

    let mut req_builder = http::Request::get(docker_image.manifest_url().to_string())
            .header("Accept", "application/vnd.docker.distribution.manifest.v2+json");

    req_builder = if let Some(token) = token {
            req_builder.header("Authorization", format!("Bearer {}", token))
        } else if let Some(creds) = docker_image.credentials.clone() {
            let header_value = format!("Basic {}:{}", base64::encode(creds.username), base64::encode(creds.password));
            req_builder.header("Authorization", header_value)
        } else {
            req_builder
        };

    let manifest_req = Request::try_from(req_builder.body("")?)?;
    let manifest_resp_result = client
        .execute(manifest_req)
        .await;

    let manifest_resp = match manifest_resp_result {
        Err(err) => {
            let msg = if err.is_timeout() {
                err_msg(format!("Error: Timed out connecting to Docker reigstry {}", docker_image.manifest_url().to_string()))
            } else if err.is_connect() {
                err_msg(format!("Error: Could not connect to Docker reigstry {}", docker_image.manifest_url().to_string()))
            } else {
                err_msg(format!("Error: Something went wrong while connecting to Docker reigstry {} ({})", docker_image.manifest_url().to_string(), err))
            };
            Err(msg)
        },
        Ok(resp) => Ok(resp)
    }?;

    if let Err(err) = manifest_resp.error_for_status_ref() {
        match err.status().unwrap() {
            StatusCode::NOT_FOUND => {
                Err(err_msg(format!("Error: Repo {} is not known to Docker registry at {}", docker_image.repo, docker_image.registry.clone().map_or_else(|| "registry.hub.docker.com".to_string(), |reg| reg.to_string()))))
            },
            StatusCode::BAD_REQUEST => {
                Err(err_msg(format!("Error: Repo name \"{}\" is invalid.", docker_image.repo)))
            },
            StatusCode::UNAUTHORIZED => {
                Err(err_msg(format!("Error: Authentication is required to access {}. Ensure you have provided valid credentials using the KILN_DOCKER_USERNAME and KILN_DOCKER_PASSWORD environment variables.", docker_image.to_string())))
            },
            StatusCode::FORBIDDEN => {
                Err(err_msg(format!("Error: You are not authorized to access {} with the credentials provided.", docker_image.to_string())))
            },
            StatusCode::TOO_MANY_REQUESTS => {
                Err(err_msg(format!("Error: You have exceeded the rate limit set by {}.", docker_image.registry.clone().map_or_else(|| "registry.hub.docker.com".to_string(), |reg| reg.to_string()))))
            },
            _ => {
                Err(err_msg(format!("Error: Something went wrong while trying to fetch Docker Image manifest. {}.", err)))
            }
        }?
    }

    let manifest_resp_body: Value = manifest_resp.json().await?;

    let layers = manifest_resp_body["layers"]
        .as_array()
        .unwrap()
        .iter()
        .map(|hashval| {
            let hashstr: String = hashval["digest"].as_str().unwrap().to_string();
            let v: Vec<&str> = hashstr.split(':').collect();
            (&(v[1].to_string())[..12]).to_string()
        })
        .collect::<HashSet<_>>();
    Ok(layers)
}

type ProgressChannelUpdate = (String, Option<ProgressDetail>, String);

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
                        let total = progress_detail.as_ref().and_then(|pd| pd.total);
                        let current = progress_detail.as_ref().and_then(|pd| pd.current);
                        if let (Some(total), Some(current)) = (total, current) {
                            pgbar.set_length(total.try_into().unwrap());
                            pgbar.set_position(current.try_into().unwrap());
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
        progress_detail: Option<ProgressDetail>,
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
    client: &Client,
    docker_image: &DockerImage,
    offline: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let docker = Docker::connect_with_local_defaults()?;

    let mut filters = HashMap::new();
    filters.insert("reference".to_string(), vec![docker_image.to_string()]);
    let options = Some(ListImagesOptions {
        filters,
        ..Default::default()
    });
    let images = docker.list_images(options).await?;

    if offline && images.is_empty() {
        Err(err_msg(format!(
            "Could not find {} locally.",
            docker_image.to_string()
        ))
        .into())
    } else if cfg!(debug_assertions) || images.is_empty() {
        let create_image_options = Some(docker_image.into());

        let layers = get_fs_layers_for_docker_image(client, docker_image).await;
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

        let mut status_stream = docker
            .create_image(create_image_options, None, docker_image.into())
            .fuse();
        loop {
            let item = status_stream.next().await;
            if item.is_none() {
                break;
            }
            match item.unwrap() {
                Ok(BuildInfo {
                    status,
                    progress_detail,
                    id,
                    ..
                }) => {
                    if let Some(prog_bar_disp) = prog_bar_disp.as_mut() {
                        prog_bar_disp.update_progress_bar(
                            id,
                            progress_detail,
                            status.unwrap_or_default(),
                        );
                    }
                    Ok(())
                }
                Err(err) => Err(err.to_string()),
            }?
        }
        Ok(())
    } else {
        println!("Tool image exists locally, skipping pull");
        Ok(())
    }
}

// If we're running inside a docker container and the tool work dir provided is a path on the host
// that is the source of a bind mount into the container we're running in, return the destination
// of that bind mount. Otherwise, return None. Used to help read kiln.toml before launching a tool
// container when we're running in a container with docker.sock bind mounted into it, which will
// result in the tool container being launched as a sibling to the container we're in.
#[cfg(target_os = "linux")]
async fn get_mapped_tool_work_dir(tool_work_dir: &str) -> Option<String> {
    let container_id = Process::myself()
        .and_then(|proc| proc.cgroups())
        .map(|mut cgroups| cgroups.pop().unwrap())
        .map(|cgroup| cgroup.pathname)
        .ok()
        .and_then(|path| {
            let path_regex = Regex::new(r"/docker/([a-f0-9]+)").unwrap();
            path_regex
                .captures(&path)
                .and_then(|regex_captures| regex_captures.get(1))
                .and_then(|capture| Some(capture.as_str().to_owned()))
        });

    match container_id {
        Some(id) => {
            let docker = Docker::connect_with_local_defaults();
            if docker.is_err() {
                return None;
            }
            let inspection = docker
                .unwrap()
                .inspect_container(&id, None::<InspectContainerOptions>)
                .await;
            inspection
                .ok()
                .and_then(|container| container.mounts)
                .and_then(|mounts: Vec<MountPoint>| {
                    mounts
                        .into_iter()
                        .find(|item| {
                            item.source.as_deref().unwrap_or_default() == tool_work_dir
                                && item.typ.as_deref().unwrap_or_default() == "bind"
                        })
                        .map(|item| item.destination)
                })
                .flatten()
        }
        None => None,
    }
}

#[cfg(not(target_os = "linux"))]
async fn get_mapped_tool_work_dir(_tool_work_dir: &str) -> Option<String> {
    None
}
#[cfg(test)]
pub mod tests {

    use super::*;
    use mockall::Sequence;
    use reqwest::{Request, Response};
    use serde_json::json;
    use std::boxed::Box;
    use std::pin::Pin;
    use std::future::Future;

    mock! {
        pub Client {
            fn execute(&self, request: Request) -> Pin<Box<dyn Future<Output = Result<Response, reqwest::Error>>>>;
        }
    }

    fn docker_image() -> DockerImage {
        DockerImage {
            registry: None,
            repo: "kiln".to_string(),
            image: "bundler-audit".to_string(),
            tag: "git-latest".to_string(),
            credentials: None
        }
    }

    #[tokio::test]
    async fn docker_registry_returns_unauthorised_while_fetching_image_manifest() {
        let mut call_sequence = Sequence::new();
        let mut client_mock = MockClient::new();
        client_mock.expect_execute()
            .times(1)
            .withf(|req| req.url().to_string().starts_with(DOCKER_AUTH_URL))
            .returning(|_| Box::pin(async {
                let body_payload = json!({
                    "token": "some token"
                });
                let response = Response::from(http::response::Response::builder()
                    .status(200)
                    .body(body_payload.to_string())
                    .unwrap());
                Ok(response)
            }))
            .in_sequence(&mut call_sequence);

        client_mock.expect_execute()
            .times(1)
            .withf(|req| req.url().to_string().starts_with(DOCKER_REGISTRY_URL))
            .returning(|_| Box::pin(async {
                let body_payload = json!({
                    "errors": [{
                        "code": "UNAUTHORIZED",
                        "message": "authentication required"
                    }]
                });
                let response = Response::from(http::response::Response::builder()
                    .status(401)
                    .body(body_payload.to_string())
                    .unwrap());
                Ok(response)
            }))
            .in_sequence(&mut call_sequence);

        let expected = "Error: ";
        let actual = get_fs_layers_for_docker_image(&client_mock, &docker_image()).await.expect_err("Expected Err(err), got Ok(_)");
        assert_eq!(expected, actual.to_string())
    }

    #[tokio::test]
    async fn docker_registry_returns_forbidden_while_fetching_image_manifest() {
        let mut call_sequence = Sequence::new();
        let mut client_mock = MockClient::new();
        client_mock.expect_execute()
            .times(1)
            .withf(|req| req.url().to_string().starts_with(DOCKER_AUTH_URL))
            .returning(|_| Box::pin(async {
                let body_payload = json!({
                    "token": "some token"
                });
                let response = Response::from(http::response::Response::builder()
                    .status(200)
                    .body(body_payload.to_string())
                    .unwrap());
                Ok(response)
            }))
            .in_sequence(&mut call_sequence);

        client_mock.expect_execute()
            .times(1)
            .withf(|req| req.url().to_string().starts_with(DOCKER_REGISTRY_URL))
            .returning(|_| Box::pin(async {
                let body_payload = json!({
                    "errors": [{
                        "code": "DENIED",
                        "message": "requested access to the resource is denied"
                    }]
                });
                let response = Response::from(http::response::Response::builder()
                    .status(403)
                    .body(body_payload.to_string())
                    .unwrap());
                Ok(response)
            }))
            .in_sequence(&mut call_sequence);

        let expected = "Error: ";
        let actual = get_fs_layers_for_docker_image(&client_mock, &docker_image()).await.expect_err("Expected Err(err), got Ok(_)");
        assert_eq!(expected, actual.to_string())
    }

    #[tokio::test]
    async fn docker_registry_returns_not_found_while_fetching_image_manifest() {
        let mut call_sequence = Sequence::new();
        let mut client_mock = MockClient::new();
        client_mock.expect_execute()
            .times(1)
            .withf(|req| req.url().to_string().starts_with(DOCKER_AUTH_URL))
            .returning(|_| Box::pin(async {
                let body_payload = json!({
                    "token": "some token"
                });
                let response = Response::from(http::response::Response::builder()
                    .status(200)
                    .body(body_payload.to_string())
                    .unwrap());
                Ok(response)
            }))
            .in_sequence(&mut call_sequence);

        client_mock.expect_execute()
            .times(1)
            .withf(|req| req.url().to_string().starts_with(DOCKER_REGISTRY_URL))
            .returning(|_| Box::pin(async {
                let body_payload = json!({
                    "errors": [{
                        "code": "NAME_UNKNOWN",
                        "message": "repository name not known to registry"
                    }]
                });
                let response = Response::from(http::response::Response::builder()
                    .status(404)
                    .body(body_payload.to_string())
                    .unwrap());
                Ok(response)
            }))
            .in_sequence(&mut call_sequence);

        let expected = "Error: ";
        let actual = get_fs_layers_for_docker_image(&client_mock, &docker_image()).await.expect_err("Expected Err(err), got Ok(_)");
        assert_eq!(expected, actual.to_string())
    }

    #[tokio::test]
    async fn docker_registry_returns_bad_request_for_image_name_while_fetching_image_manifest() {
        let mut call_sequence = Sequence::new();
        let mut client_mock = MockClient::new();
        client_mock.expect_execute()
            .times(1)
            .withf(|req| req.url().to_string().starts_with(DOCKER_AUTH_URL))
            .returning(|_| Box::pin(async {
                let body_payload = json!({
                    "token": "some token"
                });
                let response = Response::from(http::response::Response::builder()
                    .status(200)
                    .body(body_payload.to_string())
                    .unwrap());
                Ok(response)
            }))
            .in_sequence(&mut call_sequence);

        client_mock.expect_execute()
            .times(1)
            .withf(|req| req.url().to_string().starts_with(DOCKER_REGISTRY_URL))
            .returning(|_| Box::pin(async {
                let body_payload = json!({
                    "errors": [{
                        "code": "NAME_INVALID",
                        "message": "invalid repository name"
                    }]
                });
                let response = Response::from(http::response::Response::builder()
                    .status(400)
                    .body(body_payload.to_string())
                    .unwrap());
                Ok(response)
            }))
            .in_sequence(&mut call_sequence);

        let expected = "Error: ";
        let actual = get_fs_layers_for_docker_image(&client_mock, &docker_image()).await.expect_err("Expected Err(err), got Ok(_)");
        assert_eq!(expected, actual.to_string())
    }

    #[tokio::test]
    async fn docker_registry_returns_bad_request_for_tag_while_fetching_image_manifest() {
        let mut call_sequence = Sequence::new();
        let mut client_mock = MockClient::new();
        client_mock.expect_execute()
            .times(1)
            .withf(|req| req.url().to_string().starts_with(DOCKER_AUTH_URL))
            .returning(|_| Box::pin(async {
                let body_payload = json!({
                    "token": "some token"
                });
                let response = Response::from(http::response::Response::builder()
                    .status(200)
                    .body(body_payload.to_string())
                    .unwrap());
                Ok(response)
            }))
            .in_sequence(&mut call_sequence);

        client_mock.expect_execute()
            .times(1)
            .withf(|req| req.url().to_string().starts_with(DOCKER_REGISTRY_URL))
            .returning(|_| Box::pin(async {
                let body_payload = json!({
                    "errors": [{
                        "code": "TAG_INVALID",
                        "message": "manifest tag did not match URI"
                    }]
                });
                let response = Response::from(http::response::Response::builder()
                    .status(400)
                    .body(body_payload.to_string())
                    .unwrap());
                Ok(response)
            }))
            .in_sequence(&mut call_sequence);

        let expected = "Error: ";
        let actual = get_fs_layers_for_docker_image(&client_mock, &docker_image()).await.expect_err("Expected Err(err), got Ok(_)");
        assert_eq!(expected, actual.to_string())
    }

    #[tokio::test]
    async fn docker_registry_returns_too_many_requests_while_fetching_image_manifest() {
        let mut call_sequence = Sequence::new();
        let mut client_mock = MockClient::new();
        client_mock.expect_execute()
            .times(1)
            .withf(|req| req.url().to_string().starts_with(DOCKER_AUTH_URL))
            .returning(|_| Box::pin(async {
                let body_payload = json!({
                    "token": "some token"
                });
                let response = Response::from(http::response::Response::builder()
                    .status(200)
                    .body(body_payload.to_string())
                    .unwrap());
                Ok(response)
            }))
            .in_sequence(&mut call_sequence);

        client_mock.expect_execute()
            .times(1)
            .withf(|req| req.url().to_string().starts_with(DOCKER_REGISTRY_URL))
            .returning(|_| Box::pin(async {
                let body_payload = json!({
                    "errors": [{
                        "code": "TOOMANYREQUESTS",
                        "message": "too many requests"
                    }]
                });
                let response = Response::from(http::response::Response::builder()
                    .status(429)
                    .body(body_payload.to_string())
                    .unwrap());
                Ok(response)
            }))
            .in_sequence(&mut call_sequence);

        let expected = "Error: ";
        let actual = get_fs_layers_for_docker_image(&client_mock, &docker_image()).await.expect_err("Expected Err(err), got Ok(_)");
        assert_eq!(expected, actual.to_string())
    }
}
