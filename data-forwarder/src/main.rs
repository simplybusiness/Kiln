use chrono::{DateTime, Utc};
use clap::{App, Arg};
use git2::Repository;
use kiln_lib::tool_report::{
    ApplicationName, Environment, EventID, EventVersion, GitBranch, GitCommitHash, OutputFormat,
    SuppressedIssue, ToolName, ToolOutput, ToolReport, ToolVersion,
};
use kiln_lib::validation::ValidationError;
use reqwest::Client;
use reqwest::StatusCode;
use std::convert::TryFrom;
use std::env;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
use std::path::PathBuf;
use std::str::FromStr;
use toml;
use uuid::Uuid;

fn main() -> Result<(), std::boxed::Box<dyn std::error::Error>> {
    openssl_probe::init_ssl_cert_env_vars();

    let matches = App::new("Kiln data forwarder")
        .arg(
            Arg::with_name("tool_name")
                .help("Name of the security tool run")
                .long("tool-name")
                .required(true)
                .takes_value(true)
                .value_name("TOOL_NAME"),
        )
        .arg(
            Arg::with_name("tool_version")
                .help("Version of the security tool run")
                .long("tool-version")
                .takes_value(true)
                .value_name("TOOL_VERSION"),
        )
        .arg(
            Arg::with_name("tool_output_path")
                .help("Path to the output of the tool")
                .required(true)
                .long("tool-output-path")
                .takes_value(true)
                .value_name("PATH"),
        )
        .arg(
            Arg::with_name("endpoint_url")
                .help("kiln data collector URL")
                .long("endpoint-url")
                .required(true)
                .takes_value(true)
                .value_name("URL"),
        )
        .arg(
            Arg::with_name("start_time")
                .help("Start time of tool execution as a RFC3339 timestamp")
                .long("start-time")
                .required(true)
                .takes_value(true)
                .value_name("TIMESTAMP"),
        )
        .arg(
            Arg::with_name("end_time")
                .help("End time of tool execution as a RFC3339 timestamp")
                .long("end-time")
                .required(true)
                .takes_value(true)
                .value_name("TIMESTAMP"),
        )
        .arg(
            Arg::with_name("output_format")
                .help("Output format of the tool run")
                .long("output-format")
                .required(true)
                .takes_value(true)
                .value_name("OUTPUT-FORMAT")
                .possible_values(&["JSON", "PlainText"]),
        )
        .arg(
            Arg::with_name("scan_env")
                .help("Environment for the tool run")
                .long("scan-env")
                .required(true)
                .takes_value(true)
                .value_name("SCANENV")
                .possible_values(&["Local", "CI"]),
        )
        .arg(
            Arg::with_name("app_name")
                .help("Name of the application on which tool was run")
                .long("app-name")
                .required(true)
                .takes_value(true)
                .value_name("APPNAME"),
        )
        .get_matches();

    let tool_name = matches.value_of("tool_name").unwrap();
    let tool_version = matches.value_of("tool_version");
    let tool_output_path = matches.value_of("tool_output_path").unwrap();
    let endpoint_url = matches.value_of("endpoint_url").unwrap();
    let start_time = matches.value_of("start_time").unwrap();
    let parsed_start_time = DateTime::parse_from_rfc3339(&start_time)
        .map(DateTime::<Utc>::from)
        .map_err(|_| ValidationError::start_time_not_a_timestamp())?;

    let end_time = matches.value_of("end_time").unwrap();
    let parsed_end_time = DateTime::parse_from_rfc3339(&end_time)
        .map(DateTime::<Utc>::from)
        .map_err(|_| ValidationError::end_time_not_a_timestamp())?;

    let output_format = matches.value_of("output_format").unwrap();
    let scan_env = matches.value_of("scan_env").unwrap();
    let app_name = matches.value_of("app_name").unwrap();
    let path = Path::new(tool_output_path);
    let mut file = File::open(&path)?;
    let mut tool_output = String::new();
    file.read_to_string(&mut tool_output)?;

    let curr_dir = env::current_dir()?;
    let repo = Repository::discover(curr_dir)?;
    let head = repo.head()?;
    let git_branch_name = if head.is_branch() {
        head.shorthand().map(|t| t.to_string())
    } else {
        None
    };
    let git_commit = head.peel_to_commit()?.id().to_string();

    let kiln_cfg_path = PathBuf::from_str("./kiln.toml")?;
    let mut kiln_cfg_raw = String::new();

    File::open(kiln_cfg_path)?.read_to_string(&mut kiln_cfg_raw)?;
    let kiln_cfg: toml::value::Table = toml::de::from_str(&kiln_cfg_raw)?;
    let suppressed_issues = kiln_cfg
        .get("suppressed_issues")
        .and_then(|value| value.as_array())
        .and_then(|values| {
            Some(
                values
                    .into_iter()
                    .map(|value| SuppressedIssue::try_from(value.clone()))
                    .collect::<Result<Vec<_>, _>>(),
            )
        })
        .unwrap_or(Ok(vec![]));

    if let Err(e) = suppressed_issues {
        eprintln!(
            "Error while parsing suppressed issues in kiln.toml: {}",
            e.error_message
        );
        std::process::exit(1);
    }

    let tool_report = ToolReport {
        event_version: EventVersion::try_from("1".to_string())?,
        event_id: EventID::try_from(Uuid::new_v4().to_hyphenated().to_string())?,
        application_name: ApplicationName::try_from(app_name.to_string())?,
        git_branch: GitBranch::try_from(git_branch_name)?,
        git_commit_hash: GitCommitHash::try_from(git_commit)?,
        tool_name: ToolName::try_from(tool_name.to_string())?,
        tool_output: ToolOutput::try_from(tool_output.to_string())?,
        output_format: OutputFormat::try_from(output_format.to_string())?,
        start_time: parsed_start_time.into(),
        end_time: parsed_end_time.into(),
        environment: Environment::try_from(scan_env.to_string())?,
        tool_version: ToolVersion::try_from(tool_version.map(|s| s.to_string()))?,
        suppressed_issues: suppressed_issues.unwrap(),
    };

    let client = Client::new();
    let mut resp = client.post(endpoint_url).json(&tool_report).send()?;

    match resp.status() {
        StatusCode::OK => (),
        _ => eprintln!("Error received from data-collector: {}", resp.text()?),
    };

    Ok(())
}
