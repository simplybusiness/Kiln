#[cfg(feature = "avro")]
use crate::avro_schema::TOOL_REPORT_SCHEMA;
#[cfg(feature = "avro")]
use avro_rs::schema::Schema;
#[cfg(feature = "avro")]
use failure::err_msg;

#[cfg(feature = "json")]
use serde_json::value::Value;

use crate::traits::Hashable;
use crate::validation::ValidationError;

use std::convert::TryFrom;

use chrono::{DateTime, Utc};
use hex;
use regex::Regex;
use ring::digest;
use serde::Serialize;

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ToolReport {
    pub event_version: EventVersion,
    pub event_id: EventID,
    pub application_name: ApplicationName,
    pub git_branch: GitBranch,
    pub git_commit_hash: GitCommitHash,
    pub tool_name: ToolName,
    pub tool_output: ToolOutput,
    pub output_format: OutputFormat,
    pub start_time: StartTime,
    pub end_time: EndTime,
    pub environment: Environment,
    pub tool_version: ToolVersion,
    pub suppressed_issues: Vec<SuppressedIssue>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct EventVersion(String);

impl TryFrom<String> for EventVersion {
    type Error = ValidationError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.is_empty() {
            Err(ValidationError::event_version_present_but_empty())
        } else if value != "1" {
            Err(ValidationError::event_version_unknown())
        } else {
            Ok(EventVersion(value))
        }
    }
}

impl std::fmt::Display for EventVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct EventID(uuid::Uuid);

impl TryFrom<String> for EventID {
    type Error = ValidationError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.is_empty() {
            Err(ValidationError::event_id_present_but_empty())
        } else {
            match uuid::Uuid::parse_str(value.as_ref()) {
                Ok(id) => Ok(EventID(id)),
                Err(_) => Err(ValidationError::event_id_not_a_uuid()),
            }
        }
    }
}

impl std::fmt::Display for EventID {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ApplicationName(String);

impl Hashable for ApplicationName {
    fn hash(&self) -> Vec<u8> {
        digest::digest(&digest::SHA256, &self.0.as_bytes()).as_ref().to_vec()
    }
}

impl TryFrom<String> for ApplicationName {
    type Error = ValidationError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.is_empty() {
            Err(ValidationError::application_name_empty())
        } else {
            Ok(ApplicationName(value))
        }
    }
}

impl std::fmt::Display for ApplicationName {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct GitBranch(Option<String>);

impl TryFrom<Option<String>> for GitBranch {
    type Error = ValidationError;

    fn try_from(value: Option<String>) -> Result<Self, Self::Error> {
        match value {
            None => Ok(GitBranch(None)),
            Some(value) => {
                if value.is_empty() {
                    Err(ValidationError::git_branch_name_empty())
                } else {
                    Ok(GitBranch(Some(value)))
                }
            }
        }
    }
}

impl std::fmt::Display for GitBranch {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match &self.0 {
            None => write!(f, "Not Provided"),
            Some(t) => write!(f, "{}", t),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct GitCommitHash(String);

impl TryFrom<String> for GitCommitHash {
    type Error = ValidationError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.is_empty() {
            return Err(ValidationError::git_commit_hash_empty());
        };

        let re = Regex::new(r"^[0-9a-fA-F]{40}$").unwrap();
        if re.is_match(&value) {
            Ok(GitCommitHash(value))
        } else {
            Err(ValidationError::git_commit_hash_not_valid())
        }
    }
}

impl std::fmt::Display for GitCommitHash {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ToolName(String);

impl TryFrom<String> for ToolName {
    type Error = ValidationError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.is_empty() {
            Err(ValidationError::tool_name_empty())
        } else {
            Ok(ToolName(value))
        }
    }
}

impl std::fmt::Display for ToolName {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl PartialEq<&str> for ToolName {
    fn eq(&self, other: &&str) -> bool {
        self.0 == *other
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ToolOutput(String);

impl TryFrom<String> for ToolOutput {
    type Error = ValidationError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.is_empty() {
            Err(ValidationError::tool_output_empty())
        } else {
            Ok(ToolOutput(value))
        }
    }
}

impl std::fmt::Display for ToolOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for ToolOutput {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ToolVersion(Option<String>);

impl TryFrom<Option<String>> for ToolVersion {
    type Error = ValidationError;

    fn try_from(value: Option<String>) -> Result<Self, Self::Error> {
        match value {
            None => Ok(ToolVersion(None)),
            Some(value) => {
                if value.is_empty() {
                    Err(ValidationError::tool_version_present_but_empty())
                } else {
                    Ok(ToolVersion(Some(value)))
                }
            }
        }
    }
}

impl std::fmt::Display for ToolVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match &self.0 {
            Some(x) => write!(f, "{}", x),
            None => write!(f, ""),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub enum OutputFormat {
    JSON,
    PlainText,
}

impl TryFrom<String> for OutputFormat {
    type Error = ValidationError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_ref() {
            "JSON" => Ok(OutputFormat::JSON),
            "PlainText" => Ok(OutputFormat::PlainText),
            "" => Err(ValidationError::tool_output_format_empty()),
            _ => Err(ValidationError::tool_output_format_invalid()),
        }
    }
}

impl TryFrom<&str> for OutputFormat {
    type Error = ValidationError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "JSON" => Ok(OutputFormat::JSON),
            "PlainText" => Ok(OutputFormat::PlainText),
            "" => Err(ValidationError::tool_output_format_empty()),
            _ => Err(ValidationError::tool_output_format_invalid()),
        }
    }
}

impl std::fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            OutputFormat::JSON => write!(f, "JSON"),
            OutputFormat::PlainText => write!(f, "PlainText"),
        }
    }
}

impl PartialEq<&str> for OutputFormat {
    fn eq(&self, other: &&str) -> bool {
        let parsed_other = OutputFormat::try_from(*other);
        match parsed_other {
            Ok(parsed_other) => parsed_other == *self,
            _ => false
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct StartTime(DateTime<Utc>);

impl std::fmt::Display for StartTime {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0.to_rfc3339())
    }
}

impl From<DateTime<Utc>> for StartTime {
    fn from(dt: DateTime<Utc>) -> Self {
        Self(dt)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct EndTime(DateTime<Utc>);

impl std::fmt::Display for EndTime {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0.to_rfc3339())
    }
}

impl From<DateTime<Utc>> for EndTime {
    fn from(dt: DateTime<Utc>) -> Self {
        Self(dt)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub enum Environment {
    Local,
    CI,
}

impl std::fmt::Display for Environment {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Environment::Local => write!(f, "Local"),
            Environment::CI => write!(f, "CI"),
        }
    }
}

#[cfg(feature = "json")]
impl TryFrom<&Value> for ToolReport {
    type Error = ValidationError;

    fn try_from(json_value: &Value) -> Result<Self, Self::Error> {
        let event_version = ToolReport::parse_event_version(json_value)?;
        let event_id = ToolReport::parse_event_id(json_value)?;
        let application_name = ToolReport::parse_application_name(json_value)?;
        let git_branch = ToolReport::parse_git_branch(json_value)?;
        let git_commit_hash = ToolReport::parse_git_commit_hash(json_value)?;
        let tool_name = ToolReport::parse_tool_name(json_value)?;
        let tool_output = ToolReport::parse_tool_output(json_value)?;
        let output_format = ToolReport::parse_output_format(json_value)?;
        let start_time = ToolReport::parse_tool_start_time(json_value)?;
        let end_time = ToolReport::parse_tool_end_time(json_value)?;
        let environment = ToolReport::parse_environment(json_value)?;
        let tool_version = ToolReport::parse_tool_version(json_value)?;

        let suppressed_issues = match json_value["suppressed_issues"].as_array() {
            None => Err(ValidationError::suppressed_issues_not_an_array()),
            Some(unparsed_issues) => Ok(unparsed_issues.into_iter().map(|unparsed_issue| SuppressedIssue::try_from(unparsed_issue)).collect::<Vec<Result<SuppressedIssue, Self::Error>>>())
        }?
            .into_iter().collect::<Result<Vec<SuppressedIssue>, _>>()?;

        Ok(ToolReport {
            event_version,
            event_id,
            application_name,
            git_branch,
            git_commit_hash,
            tool_name,
            tool_output,
            output_format,
            start_time,
            end_time,
            environment,
            tool_version,
            suppressed_issues,
        })
    }
}

#[cfg(feature = "avro")]
impl<'a> TryFrom<avro_rs::types::Value> for ToolReport {
    type Error = failure::Error;

    fn try_from(value: avro_rs::types::Value) -> Result<Self, Self::Error> {
        let schema = Schema::parse_str(TOOL_REPORT_SCHEMA).unwrap();
        let resolved_value = value.resolve(&schema).map_err(|err| {
            err_msg(format!(
                "Error resolving Avro schema: {}",
                err.name().unwrap()
            ))
        })?;

        if let avro_rs::types::Value::Record(record) = resolved_value {
            let mut fields = record.iter();
            let event_version =
                EventVersion::try_from(fields.find(|&x| x.0 == "event_version").unwrap().1.clone())
                    .map_err(|err| err_msg(err.error_message))?;
            let event_id =
                EventID::try_from(fields.find(|&x| x.0 == "event_id").unwrap().1.clone())
                    .map_err(|err| err_msg(err.error_message))?;
            let application_name = ApplicationName::try_from(
                fields
                    .find(|&x| x.0 == "application_name")
                    .unwrap()
                    .1
                    .clone(),
            )
            .map_err(|err| err_msg(err.error_message))?;
            let git_branch =
                GitBranch::try_from(fields.find(|&x| x.0 == "git_branch").unwrap().1.clone())
                    .map_err(|err| err_msg(err.error_message))?;
            let git_commit_hash = GitCommitHash::try_from(
                fields
                    .find(|&x| x.0 == "git_commit_hash")
                    .unwrap()
                    .1
                    .clone(),
            )
            .map_err(|err| err_msg(err.error_message))?;
            let tool_name =
                ToolName::try_from(fields.find(|&x| x.0 == "tool_name").unwrap().1.clone())
                    .map_err(|err| err_msg(err.error_message))?;
            let tool_output =
                ToolOutput::try_from(fields.find(|&x| x.0 == "tool_output").unwrap().1.clone())
                    .map_err(|err| err_msg(err.error_message))?;
            let output_format =
                OutputFormat::try_from(fields.find(|&x| x.0 == "output_format").unwrap().1.clone())
                    .map_err(|err| err_msg(err.error_message))?;
            let start_time =
                StartTime::try_from(fields.find(|&x| x.0 == "start_time").unwrap().1.clone())
                    .map_err(|err| err_msg(err.error_message))?;
            let end_time =
                EndTime::try_from(fields.find(|&x| x.0 == "end_time").unwrap().1.clone())
                    .map_err(|err| err_msg(err.error_message))?;
            let environment =
                Environment::try_from(fields.find(|&x| x.0 == "environment").unwrap().1.clone())
                    .map_err(|err| err_msg(err.error_message))?;
            let tool_version =
                ToolVersion::try_from(fields.find(|&x| x.0 == "tool_version").unwrap().1.clone())
                    .map_err(|err| err_msg(err.error_message))?;
            let suppressed_issues_avro =
                fields.find(|&x| x.0 == "suppressed_issues").unwrap().1.clone();

            let suppressed_issues = match suppressed_issues_avro {
                avro_rs::types::Value::Array(issues) => {
                    Ok(issues.into_iter().map(|unparsed_issue| SuppressedIssue::try_from(unparsed_issue).map_err(|err| err_msg(err.error_message))).collect::<Result<Vec<SuppressedIssue>, _>>())
                },
                _ => Err(ValidationError::suppressed_issues_not_an_array()),
            }??;

            Ok(ToolReport {
                event_version,
                event_id,
                application_name,
                git_branch,
                git_commit_hash,
                tool_name,
                tool_output,
                output_format,
                start_time,
                end_time,
                environment,
                tool_version,
                suppressed_issues,
            })
        } else {
            Err(err_msg("Something went wrong decoding Avro record"))
        }
    }
}

#[cfg(feature = "avro")]
impl TryFrom<avro_rs::types::Value> for EventVersion {
    type Error = ValidationError;

    fn try_from(value: avro_rs::types::Value) -> Result<Self, Self::Error> {
        match value {
            avro_rs::types::Value::String(s) => EventVersion::try_from(s),
            _ => Err(ValidationError::event_version_not_a_string()),
        }
    }
}

#[cfg(feature = "avro")]
impl TryFrom<avro_rs::types::Value> for EventID {
    type Error = ValidationError;

    fn try_from(value: avro_rs::types::Value) -> Result<Self, Self::Error> {
        match value {
            avro_rs::types::Value::String(s) => EventID::try_from(s),
            _ => Err(ValidationError::event_id_not_a_string()),
        }
    }
}

#[cfg(feature = "avro")]
impl TryFrom<avro_rs::types::Value> for ApplicationName {
    type Error = ValidationError;

    fn try_from(value: avro_rs::types::Value) -> Result<Self, Self::Error> {
        match value {
            avro_rs::types::Value::String(s) => ApplicationName::try_from(s),
            _ => Err(ValidationError::application_name_not_a_string()),
        }
    }
}

#[cfg(feature = "avro")]
impl TryFrom<avro_rs::types::Value> for GitBranch {
    type Error = ValidationError;

    fn try_from(value: avro_rs::types::Value) -> Result<Self, Self::Error> {
        match value {
            avro_rs::types::Value::String(s) => GitBranch::try_from(Some(s)),
            avro_rs::types::Value::Null => GitBranch::try_from(None),
            _ => Err(ValidationError::git_branch_name_not_a_string()),
        }
    }
}

#[cfg(feature = "avro")]
impl TryFrom<avro_rs::types::Value> for GitCommitHash {
    type Error = ValidationError;

    fn try_from(value: avro_rs::types::Value) -> Result<Self, Self::Error> {
        match value {
            avro_rs::types::Value::String(s) => GitCommitHash::try_from(s),
            _ => Err(ValidationError::git_commit_hash_not_a_string()),
        }
    }
}

#[cfg(feature = "avro")]
impl TryFrom<avro_rs::types::Value> for ToolName {
    type Error = ValidationError;

    fn try_from(value: avro_rs::types::Value) -> Result<Self, Self::Error> {
        match value {
            avro_rs::types::Value::String(s) => ToolName::try_from(s),
            _ => Err(ValidationError::tool_name_not_a_string()),
        }
    }
}

#[cfg(feature = "avro")]
impl TryFrom<avro_rs::types::Value> for ToolOutput {
    type Error = ValidationError;

    fn try_from(value: avro_rs::types::Value) -> Result<Self, Self::Error> {
        match value {
            avro_rs::types::Value::String(s) => ToolOutput::try_from(s),
            _ => Err(ValidationError::tool_output_not_a_string()),
        }
    }
}

#[cfg(feature = "avro")]
impl TryFrom<avro_rs::types::Value> for OutputFormat {
    type Error = ValidationError;

    fn try_from(value: avro_rs::types::Value) -> Result<Self, Self::Error> {
        let x = match value {
            avro_rs::types::Value::Enum(pos, val) => Ok((pos, val)),
            _ => Err(ValidationError::tool_output_format_not_an_enum()),
        }?;

        if x.1.is_empty() {
            return Err(ValidationError::tool_output_format_empty());
        }

        Self::try_from(x.1)
    }
}

#[cfg(feature = "avro")]
impl TryFrom<avro_rs::types::Value> for StartTime {
    type Error = ValidationError;

    fn try_from(value: avro_rs::types::Value) -> Result<Self, Self::Error> {
        match value {
            avro_rs::types::Value::String(s) => DateTime::parse_from_rfc3339(&s)
                .map(|dt| StartTime(DateTime::<Utc>::from(dt)))
                .map_err(|_| ValidationError::start_time_not_a_timestamp()),
            _ => Err(ValidationError::start_time_not_a_timestamp()),
        }
    }
}

#[cfg(feature = "avro")]
impl TryFrom<avro_rs::types::Value> for EndTime {
    type Error = ValidationError;

    fn try_from(value: avro_rs::types::Value) -> Result<Self, Self::Error> {
        match value {
            avro_rs::types::Value::String(s) => DateTime::parse_from_rfc3339(&s)
                .map(|dt| EndTime(DateTime::<Utc>::from(dt)))
                .map_err(|_| ValidationError::end_time_not_a_timestamp()),
            _ => Err(ValidationError::end_time_not_a_timestamp()),
        }
    }
}

impl TryFrom<String> for Environment {
    type Error = ValidationError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_ref() {
            "Local" => Ok(Environment::Local),
            "CI" => Ok(Environment::CI),
            "" => Err(ValidationError::environment_empty()),
            _ => Err(ValidationError::environment_not_a_valid_option()),
        }
    }
}

#[cfg(feature = "avro")]
impl TryFrom<avro_rs::types::Value> for Environment {
    type Error = ValidationError;

    fn try_from(value: avro_rs::types::Value) -> Result<Self, Self::Error> {
        let x = match value {
            avro_rs::types::Value::Enum(pos, val) => Ok((pos, val)),
            _ => Err(ValidationError::environment_not_an_enum()),
        }?;

        Self::try_from(x.1)
    }
}

#[cfg(feature = "avro")]
impl TryFrom<avro_rs::types::Value> for ToolVersion {
    type Error = ValidationError;

    fn try_from(value: avro_rs::types::Value) -> Result<Self, Self::Error> {
        match value {
            avro_rs::types::Value::Null => Ok(ToolVersion(None)),
            avro_rs::types::Value::String(s) => Ok(ToolVersion(Some(s))),
            _ => Err(ValidationError::tool_version_not_a_string()),
        }
    }
}

#[cfg(feature = "json")]
impl ToolReport {
    fn parse_event_version(json_value: &Value) -> Result<EventVersion, ValidationError> {
        let value = match &json_value["event_version"] {
            Value::Null => Err(ValidationError::event_version_missing()),
            Value::String(value) => Ok(value),
            _ => Err(ValidationError::event_version_not_a_string()),
        }?;
        EventVersion::try_from(value.to_owned())
    }

    fn parse_event_id(json_value: &Value) -> Result<EventID, ValidationError> {
        let value = match &json_value["event_id"] {
            Value::Null => Err(ValidationError::event_id_missing()),
            Value::String(value) => Ok(value),
            _ => Err(ValidationError::event_id_not_a_string()),
        }?;
        EventID::try_from(value.to_owned())
    }

    fn parse_application_name(json_value: &Value) -> Result<ApplicationName, ValidationError> {
        let value = match &json_value["application_name"] {
            Value::Null => Err(ValidationError::application_name_missing()),
            Value::String(value) => Ok(value),
            _ => Err(ValidationError::application_name_not_a_string()),
        }?;
        ApplicationName::try_from(value.to_owned())
    }

    fn parse_git_branch(json_value: &Value) -> Result<GitBranch, ValidationError> {
        let value = match &json_value["git_branch"] {
            Value::Null => Ok(None),
            Value::String(value) => Ok(Some(value.to_owned())),
            _ => Err(ValidationError::git_branch_name_not_a_string()),
        }?;
        GitBranch::try_from(value)
    }

    fn parse_git_commit_hash(json_value: &Value) -> Result<GitCommitHash, ValidationError> {
        let value = match &json_value["git_commit_hash"] {
            Value::Null => Err(ValidationError::git_commit_hash_missing()),
            Value::String(value) => Ok(value),
            _ => Err(ValidationError::git_commit_hash_not_a_string()),
        }?;
        GitCommitHash::try_from(value.to_owned())
    }

    fn parse_tool_name(json_value: &Value) -> Result<ToolName, ValidationError> {
        let value = match &json_value["tool_name"] {
            Value::Null => Err(ValidationError::tool_name_missing()),
            Value::String(value) => Ok(value),
            _ => Err(ValidationError::tool_name_not_a_string()),
        }?;
        ToolName::try_from(value.to_owned())
    }

    fn parse_tool_output(json_value: &Value) -> Result<ToolOutput, ValidationError> {
        let value = match &json_value["tool_output"] {
            Value::Null => Err(ValidationError::tool_output_missing()),
            Value::String(value) => Ok(value),
            _ => Err(ValidationError::tool_output_not_a_string()),
        }?;
        ToolOutput::try_from(value.to_owned())
    }

    fn parse_output_format(json_value: &Value) -> Result<OutputFormat, ValidationError> {
        let value = match &json_value["output_format"] {
            Value::Null => Err(ValidationError::tool_output_format_missing()),
            Value::String(value) => Ok(value),
            _ => Err(ValidationError::tool_output_format_not_a_string()),
        }?;
        if value.is_empty() {
            return Err(ValidationError::tool_output_format_empty());
        };

        match value.as_ref() {
            "JSON" => Ok(OutputFormat::JSON),
            "PlainText" => Ok(OutputFormat::PlainText),
            _ => Err(ValidationError::tool_output_format_invalid()),
        }
    }

    fn parse_tool_start_time(json_value: &Value) -> Result<StartTime, ValidationError> {
        let value = match &json_value["start_time"] {
            Value::Null => Err(ValidationError::start_time_missing()),
            Value::String(value) => Ok(value),
            _ => Err(ValidationError::start_time_not_a_timestamp()),
        }?;

        DateTime::parse_from_rfc3339(value)
            .map(|dt| StartTime(DateTime::<Utc>::from(dt)))
            .map_err(|_| ValidationError::start_time_not_a_timestamp())
    }

    fn parse_tool_end_time(json_value: &Value) -> Result<EndTime, ValidationError> {
        let value = match &json_value["end_time"] {
            Value::Null => Err(ValidationError::end_time_missing()),
            Value::String(value) => Ok(value),
            _ => Err(ValidationError::end_time_not_a_timestamp()),
        }?;

        DateTime::parse_from_rfc3339(value)
            .map(|dt| EndTime(DateTime::<Utc>::from(dt)))
            .map_err(|_| ValidationError::end_time_not_a_timestamp())
    }

    fn parse_environment(json_value: &Value) -> Result<Environment, ValidationError> {
        let value = match &json_value["environment"] {
            Value::Null => Err(ValidationError::environment_missing()),
            Value::String(value) => Ok(value.to_owned()),
            _ => Err(ValidationError::environment_not_a_string()),
        }?;

        Environment::try_from(value)
    }

    fn parse_tool_version(json_value: &Value) -> Result<ToolVersion, ValidationError> {
        let value = match &json_value["tool_version"] {
            Value::Null => Ok(None),
            Value::String(value) => Ok(Some(value.to_owned())),
            _ => Err(ValidationError::tool_version_not_a_string()),
        }?;
        ToolVersion::try_from(value)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SuppressedIssue {
    pub issue_hash: IssueHash,
    pub expiry_date: ExpiryDate,
    pub suppression_reason: SuppressionReason,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct IssueHash(
    #[serde(with = "hex")]
    Vec<u8>
);

impl TryFrom<String> for IssueHash {
    type Error = ValidationError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.chars().count() != 64 {
            Err(ValidationError::issue_hash_not_valid())
        } else {
            match hex::decode(value) {
                Ok(bytes) => Ok(IssueHash(bytes)),
                Err(_) => Err(ValidationError::issue_hash_not_valid())
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ExpiryDate(Option<DateTime<Utc>>);

impl TryFrom<Option<String>> for ExpiryDate {
    type Error = ValidationError;

    fn try_from(value: Option<String>) -> Result<Self, Self::Error> {
        match value {
            None => Ok(ExpiryDate(None)),
            Some(value) => DateTime::parse_from_rfc3339(&value)
                                .map(|dt| ExpiryDate(Some(DateTime::<Utc>::from(dt))))
                                .map_err(|_| ValidationError::expiry_date_not_a_valid_date())
        }
    }
}

impl std::cmp::PartialEq<DateTime<Utc>> for ExpiryDate {
    fn eq(&self, rhs: &DateTime<Utc>) -> bool {
        match self.0 {
            Some(dt) => {
                dt == *rhs
            },
            None => false
        }
    }
}

impl std::cmp::PartialOrd<DateTime<Utc>> for ExpiryDate {
    fn partial_cmp(&self, rhs: &DateTime<Utc>) -> Option<std::cmp::Ordering> {
        self.0.and_then(|x| x.partial_cmp(rhs))
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SuppressionReason(String);

impl TryFrom<String> for SuppressionReason {
    type Error = ValidationError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.is_empty() {
            Err(ValidationError::suppression_reason_empty())
        } else {
            Ok(SuppressionReason(value))
        }
    }
}

#[cfg(feature = "json")]
impl SuppressedIssue {
    fn parse_issue_hash(json_value: &Value) -> Result<IssueHash, ValidationError> {
        let value = match &json_value["issue_hash"] {
            Value::Null => Err(ValidationError::issue_hash_required()),
            Value::String(value) => Ok(value.to_owned()),
            _ => Err(ValidationError::issue_hash_not_a_string()),
        }?;
        IssueHash::try_from(value)
    }

    fn parse_expiry_date(json_value: &Value) -> Result<ExpiryDate, ValidationError> {
        let value = match &json_value["expiry_date"] {
            Value::Null => Ok(None),
            Value::String(value) => Ok(Some(value.to_owned())),
            _ => Err(ValidationError::expiry_date_not_a_string()),
        }?;
        ExpiryDate::try_from(value)
    }

    fn parse_suppression_reason(json_value: &Value) -> Result<SuppressionReason, ValidationError> {
        let value = match &json_value["suppression_reason"] {
            Value::Null => Err(ValidationError::suppression_reason_required()),
            Value::String(value) => Ok(value.to_owned()),
            _ => Err(ValidationError::suppression_reason_not_a_string()),
        }?;
        SuppressionReason::try_from(value)
    }
}

#[cfg(feature = "json")]
impl TryFrom<&Value> for SuppressedIssue {
    type Error = ValidationError;

    fn try_from(json_value: &Value) -> Result<Self, Self::Error> {
        Ok(SuppressedIssue {
            issue_hash: SuppressedIssue::parse_issue_hash(json_value)?,
            expiry_date: SuppressedIssue::parse_expiry_date(json_value)?,
            suppression_reason: SuppressedIssue::parse_suppression_reason(json_value)?,
        })
    }
}

#[cfg(feature = "avro")]
impl TryFrom<avro_rs::types::Value> for SuppressedIssue {
    type Error = ValidationError;

    fn try_from(value: avro_rs::types::Value) -> Result<Self, Self::Error> {
        match value {
            avro_rs::types::Value::Record(unparsed_issue) => {
                let mut fields = unparsed_issue.iter();
                println!("{:?}", fields);
                let issue_hash = IssueHash::try_from(
                    fields
                        .find(|&x| x.0 == "issue_hash")
                        .unwrap()
                        .1
                        .clone(),
                );
                
                let expiry_date = ExpiryDate::try_from(
                    fields
                        .find(|&x| x.0 == "expiry_date")
                        .unwrap()
                        .1
                        .clone(),
                );

                let suppression_reason = SuppressionReason::try_from(
                    fields
                        .find(|&x| x.0 == "suppression_reason")
                        .unwrap()
                        .1
                        .clone(),
                );

                Ok(SuppressedIssue {
                    issue_hash: issue_hash?,
                    suppression_reason: suppression_reason?,
                    expiry_date: expiry_date?,
                })
            }
            _ => Err(ValidationError::suppressed_issue_not_a_record()),
        }
    }
}

#[cfg(feature = "avro")]
impl TryFrom<avro_rs::types::Value> for IssueHash {
    type Error = ValidationError;

    fn try_from(value: avro_rs::types::Value) -> Result<Self, Self::Error> {
        match value {
            avro_rs::types::Value::String(s) => IssueHash::try_from(s),
            _ => Err(ValidationError::issue_hash_not_a_string()),
        }
    }
}

#[cfg(feature = "avro")]
impl TryFrom<avro_rs::types::Value> for ExpiryDate {
    type Error = ValidationError;

    fn try_from(value: avro_rs::types::Value) -> Result<Self, Self::Error> {
        match value {
            avro_rs::types::Value::String(s) => ExpiryDate::try_from(Some(s)),
            avro_rs::types::Value::Null => Ok(ExpiryDate::from(None)),
            _ => Err(ValidationError::expiry_date_not_a_string()),
        }
    }
}

impl From<Option<DateTime<Utc>>> for ExpiryDate {
    fn from(dt: Option<DateTime<Utc>>) -> Self {
        Self(dt)
    }
}

#[cfg(feature = "avro")]
impl TryFrom<avro_rs::types::Value> for SuppressionReason {
    type Error = ValidationError;

    fn try_from(value: avro_rs::types::Value) -> Result<Self, Self::Error> {
        match value {
            avro_rs::types::Value::String(s) => SuppressionReason::try_from(s),
            _ => Err(ValidationError::suppression_reason_not_a_string()),
        }
    }
}

#[cfg(test)]
#[cfg(feature = "all")]
pub mod tests {
    // TODO: Separate tests based on whether they test the JSON validation or the business logic
    // validation
    use super::*;

    use avro_rs::{Reader, Schema, Writer};

    pub mod tool_report_json {
        use super::*;

        #[test]
        fn try_from_returns_error_when_event_version_missing() {
            let message = serde_json::from_str(
                r#"{
                "event_id": "383bc5f5-d099-40a4-a1a9-8c8a97559479",
                "application_name": "Test application",
                "git_branch": "master",
                "git_commit_hash": "e99f715d0fe787cd43de967b8a79b56960fed3e5",
                "tool_name": "example tool",
                "tool_output": "{}",
                "output_format": "JSON",
                "start_time": "2019-09-13T19:35:38+00:00",
                "end_time": "2019-09-13T19:37:14+00:00",
                "environment": "Local",
                "tool_version": "1.0"
            }"#,
            )
            .unwrap();

            let expected = ValidationError::event_version_missing();
            let actual = ToolReport::try_from(&message).expect_err("expected Err(_) value");

            assert_eq!(expected, actual);
        }

        #[test]
        fn try_from_returns_error_when_event_version_present_but_empty() {
            let message = serde_json::from_str(
                r#"{
                "event_version": "",
                "event_id": "383bc5f5-d099-40a4-a1a9-8c8a97559479",
                "application_name": "Test application",
                "git_branch": "master",
                "git_commit_hash": "e99f715d0fe787cd43de967b8a79b56960fed3e5",
                "tool_name": "example tool",
                "tool_output": "{}",
                "output_format": "JSON",
                "start_time": "2019-09-13T19:35:38+00:00",
                "end_time": "2019-09-13T19:37:14+00:00",
                "environment": "Local",
                "tool_version": "1.0"
            }"#,
            )
            .unwrap();

            let expected = ValidationError::event_version_present_but_empty();
            let actual = ToolReport::try_from(&message).expect_err("expected Err(_) value");

            assert_eq!(expected, actual);
        }

        #[test]
        fn try_from_returns_error_when_event_version_unknown() {
            let message = serde_json::from_str(
                r#"{
                "event_version": "0",
                "event_id": "383bc5f5-d099-40a4-a1a9-8c8a97559479",
                "application_name": "Test application",
                "git_branch": "master",
                "git_commit_hash": "e99f715d0fe787cd43de967b8a79b56960fed3e5",
                "tool_name": "example tool",
                "tool_output": "{}",
                "output_format": "JSON",
                "start_time": "2019-09-13T19:35:38+00:00",
                "end_time": "2019-09-13T19:37:14+00:00",
                "environment": "Local",
                "tool_version": "1.0"
            }"#,
            )
            .unwrap();

            let expected = ValidationError::event_version_unknown();
            let actual = ToolReport::try_from(&message).expect_err("expected Err(_) value");

            assert_eq!(expected, actual);
        }

        #[test]
        fn try_from_returns_error_when_event_version_not_a_string() {
            let message = serde_json::from_str(
                r#"{
                "event_version": false,
                "event_id": "383bc5f5-d099-40a4-a1a9-8c8a97559479",
                "application_name": "Test application",
                "git_branch": "master",
                "git_commit_hash": "e99f715d0fe787cd43de967b8a79b56960fed3e5",
                "tool_name": "example tool",
                "tool_output": "{}",
                "output_format": "JSON",
                "start_time": "2019-09-13T19:35:38+00:00",
                "end_time": "2019-09-13T19:37:14+00:00",
                "environment": "Local",
                "tool_version": "1.0"
            }"#,
            )
            .unwrap();

            let expected = ValidationError::event_version_not_a_string();
            let actual = ToolReport::try_from(&message).expect_err("expected Err(_) value");

            assert_eq!(expected, actual);
        }

        #[test]
        fn try_from_returns_error_when_event_id_missing() {
            let message = serde_json::from_str(
                r#"{
                "event_version": "1",
                "application_name": "Test application",
                "git_branch": "master",
                "git_commit_hash": "e99f715d0fe787cd43de967b8a79b56960fed3e5",
                "tool_name": "example tool",
                "tool_output": "{}",
                "output_format": "JSON",
                "start_time": "2019-09-13T19:35:38+00:00",
                "end_time": "2019-09-13T19:37:14+00:00",
                "environment": "Local",
                "tool_version": "1.0"
            }"#,
            )
            .unwrap();

            let expected = ValidationError::event_id_missing();
            let actual = ToolReport::try_from(&message).expect_err("expected Err(_) value");

            assert_eq!(expected, actual);
        }

        #[test]
        fn try_from_returns_error_when_event_id_present_but_empty() {
            let message = serde_json::from_str(
                r#"{
                "event_version": "1",
                "event_id": "",
                "application_name": "Test application",
                "git_branch": "master",
                "git_commit_hash": "e99f715d0fe787cd43de967b8a79b56960fed3e5",
                "tool_name": "example tool",
                "tool_output": "{}",
                "output_format": "JSON",
                "start_time": "2019-09-13T19:35:38+00:00",
                "end_time": "2019-09-13T19:37:14+00:00",
                "environment": "Local",
                "tool_version": "1.0"
            }"#,
            )
            .unwrap();

            let expected = ValidationError::event_id_present_but_empty();
            let actual = ToolReport::try_from(&message).expect_err("expected Err(_) value");

            assert_eq!(expected, actual);
        }

        #[test]
        fn try_from_returns_error_when_event_id_not_a_uuid() {
            let message = serde_json::from_str(
                r#"{
                "event_version": "1",
                "event_id": "not a uuid",
                "application_name": "Test application",
                "git_branch": "master",
                "git_commit_hash": "e99f715d0fe787cd43de967b8a79b56960fed3e5",
                "tool_name": "example tool",
                "tool_output": "{}",
                "output_format": "JSON",
                "start_time": "2019-09-13T19:35:38+00:00",
                "end_time": "2019-09-13T19:37:14+00:00",
                "environment": "Local",
                "tool_version": "1.0"
            }"#,
            )
            .unwrap();

            let expected = ValidationError::event_id_not_a_uuid();
            let actual = ToolReport::try_from(&message).expect_err("expected Err(_) value");

            assert_eq!(expected, actual);
        }

        #[test]
        fn try_from_returns_error_when_event_id_not_a_string() {
            let message = serde_json::from_str(
                r#"{
                "event_version": "1",
                "event_id": false,
                "application_name": "Test application",
                "git_branch": "master",
                "git_commit_hash": "e99f715d0fe787cd43de967b8a79b56960fed3e5",
                "tool_name": "example tool",
                "tool_output": "{}",
                "output_format": "JSON",
                "start_time": "2019-09-13T19:35:38+00:00",
                "end_time": "2019-09-13T19:37:14+00:00",
                "environment": "Local",
                "tool_version": "1.0"
            }"#,
            )
            .unwrap();

            let expected = ValidationError::event_id_not_a_string();
            let actual = ToolReport::try_from(&message).expect_err("expected Err(_) value");

            assert_eq!(expected, actual);
        }

        #[test]
        fn try_from_returns_error_when_application_name_missing() {
            let message = serde_json::from_str(
                r#"{
                "event_version": "1",
                "event_id": "383bc5f5-d099-40a4-a1a9-8c8a97559479",
                "git_branch": "master",
                "git_commit_hash": "e99f715d0fe787cd43de967b8a79b56960fed3e5",
                "tool_name": "example tool",
                "tool_output": "{}",
                "output_format": "JSON",
                "start_time": "2019-09-13T19:35:38+00:00",
                "end_time": "2019-09-13T19:37:14+00:00",
                "environment": "Local",
                "tool_version": "1.0"
            }"#,
            )
            .unwrap();

            let expected = ValidationError::application_name_missing();
            let actual = ToolReport::try_from(&message).expect_err("expected Err(_) value");

            assert_eq!(expected, actual);
        }

        #[test]
        fn try_from_returns_error_when_git_commit_hash_missing() {
            let message = serde_json::from_str(
                r#"{
                "event_version": "1",
                "event_id": "383bc5f5-d099-40a4-a1a9-8c8a97559479",
                "application_name": "Test application",
                "git_branch": "master",
                "tool_name": "example tool",
                "tool_output": "{}",
                "output_format": "JSON",
                "start_time": "2019-09-13T19:35:38+00:00",
                "end_time": "2019-09-13T19:37:14+00:00",
                "environment": "Local",
                "tool_version": "1.0"
            }"#,
            )
            .unwrap();

            let expected = ValidationError::git_commit_hash_missing();
            let actual = ToolReport::try_from(&message).expect_err("expected Err(_) value");

            assert_eq!(expected, actual);
        }

        #[test]
        fn try_from_returns_error_when_tool_name_missing() {
            let message = serde_json::from_str(
                r#"{
                "event_version": "1",
                "event_id": "383bc5f5-d099-40a4-a1a9-8c8a97559479",
                "application_name": "Test application",
                "git_branch": "master",
                "git_commit_hash": "e99f715d0fe787cd43de967b8a79b56960fed3e5",
                "tool_output": "{}",
                "output_format": "JSON",
                "start_time": "2019-09-13T19:35:38+00:00",
                "end_time": "2019-09-13T19:37:14+00:00",
                "environment": "Local",
                "tool_version": "1.0"
            }"#,
            )
            .unwrap();

            let expected = ValidationError::tool_name_missing();
            let actual = ToolReport::try_from(&message).expect_err("expected Err(_) value");

            assert_eq!(expected, actual);
        }

        #[test]
        fn try_from_returns_error_when_tool_output_missing() {
            let message = serde_json::from_str(
                r#"{
                "event_version": "1",
                "event_id": "383bc5f5-d099-40a4-a1a9-8c8a97559479",
                "application_name": "Test application",
                "git_branch": "master",
                "git_commit_hash": "e99f715d0fe787cd43de967b8a79b56960fed3e5",
                "tool_name": "example tool",
                "output_format": "JSON",
                "start_time": "2019-09-13T19:35:38+00:00",
                "end_time": "2019-09-13T19:37:14+00:00",
                "environment": "Local",
                "tool_version": "1.0"
            }"#,
            )
            .unwrap();

            let expected = ValidationError::tool_output_missing();
            let actual = ToolReport::try_from(&message).expect_err("expected Err(_) value");

            assert_eq!(expected, actual);
        }

        #[test]
        fn try_from_returns_error_when_tool_output_format_missing() {
            let message = serde_json::from_str(
                r#"{
                "event_version": "1",
                "event_id": "383bc5f5-d099-40a4-a1a9-8c8a97559479",
                "application_name": "Test application",
                "git_branch": "master",
                "git_commit_hash": "e99f715d0fe787cd43de967b8a79b56960fed3e5",
                "tool_name": "example tool",
                "tool_output": "{}",
                "start_time": "2019-09-13T19:35:38+00:00",
                "end_time": "2019-09-13T19:37:14+00:00",
                "environment": "Local",
                "tool_version": "1.0"
            }"#,
            )
            .unwrap();

            let expected = ValidationError::tool_output_format_missing();
            let actual = ToolReport::try_from(&message).expect_err("expected Err(_) value");

            assert_eq!(expected, actual);
        }

        #[test]
        fn try_from_returns_error_when_start_time_missing() {
            let message = serde_json::from_str(
                r#"{
                "event_version": "1",
                "event_id": "383bc5f5-d099-40a4-a1a9-8c8a97559479",
                "application_name": "Test application",
                "git_branch": "master",
                "git_commit_hash": "e99f715d0fe787cd43de967b8a79b56960fed3e5",
                "tool_name": "example tool",
                "tool_output": "{}",
                "output_format": "JSON",
                "end_time": "2019-09-13T19:37:14+00:00",
                "environment": "Local",
                "tool_version": "1.0"
            }"#,
            )
            .unwrap();

            let expected = ValidationError::start_time_missing();
            let actual = ToolReport::try_from(&message).expect_err("expected Err(_) value");

            assert_eq!(expected, actual);
        }

        #[test]
        fn try_from_returns_error_when_end_time_missing() {
            let message = serde_json::from_str(
                r#"{
                "event_version": "1",
                "event_id": "383bc5f5-d099-40a4-a1a9-8c8a97559479",
                "application_name": "Test application",
                "git_branch": "master",
                "git_commit_hash": "e99f715d0fe787cd43de967b8a79b56960fed3e5",
                "tool_name": "example tool",
                "tool_output": "{}",
                "output_format": "JSON",
                "start_time": "2019-09-13T19:35:38+00:00",
                "environment": "Local",
                "tool_version": "1.0"
            }"#,
            )
            .unwrap();

            let expected = ValidationError::end_time_missing();
            let actual = ToolReport::try_from(&message).expect_err("expected Err(_) value");

            assert_eq!(expected, actual);
        }

        #[test]
        fn try_from_returns_error_when_environment_missing() {
            let message = serde_json::from_str(
                r#"{
                "event_version": "1",
                "event_id": "383bc5f5-d099-40a4-a1a9-8c8a97559479",
                "application_name": "Test application",
                "git_branch": "master",
                "git_commit_hash": "e99f715d0fe787cd43de967b8a79b56960fed3e5",
                "tool_name": "example tool",
                "tool_output": "{}",
                "output_format": "JSON",
                "start_time": "2019-09-13T19:35:38+00:00",
                "end_time": "2019-09-13T19:37:14+00:00",
                "tool_version": "1.0"
            }"#,
            )
            .unwrap();

            let expected = ValidationError::environment_missing();
            let actual = ToolReport::try_from(&message).expect_err("expected Err(_) value");

            assert_eq!(expected, actual);
        }

        #[test]
        fn try_from_returns_error_when_application_name_not_a_string() {
            let message = serde_json::from_str(
                r#"{
                "event_version": "1",
                "event_id": "383bc5f5-d099-40a4-a1a9-8c8a97559479",
                "application_name": false,
                "git_branch": "master",
                "git_commit_hash": "e99f715d0fe787cd43de967b8a79b56960fed3e5",
                "tool_name": "example tool",
                "tool_output": "{}",
                "output_format": "JSON",
                "start_time": "2019-09-13T19:35:38+00:00",
                "end_time": "2019-09-13T19:37:14+00:00",
                "environment": "Local",
                "tool_version": "1.0"
            }"#,
            )
            .unwrap();

            let expected = ValidationError::application_name_not_a_string();
            let actual = ToolReport::try_from(&message).expect_err("expected Err(_) value");

            assert_eq!(expected, actual);
        }

        #[test]
        fn try_from_returns_error_when_git_branch_name_not_a_string() {
            let message = serde_json::from_str(
                r#"{
                "event_version": "1",
                "event_id": "383bc5f5-d099-40a4-a1a9-8c8a97559479",
                "application_name": "Test application",
                "git_branch": false,
                "git_commit_hash": "e99f715d0fe787cd43de967b8a79b56960fed3e5",
                "tool_name": "example tool",
                "tool_output": "{}",
                "output_format": "JSON",
                "start_time": "2019-09-13T19:35:38+00:00",
                "end_time": "2019-09-13T19:37:14+00:00",
                "environment": "Local",
                "tool_version": "1.0"
            }"#,
            )
            .unwrap();

            let expected = ValidationError::git_branch_name_not_a_string();
            let actual = ToolReport::try_from(&message).expect_err("expected Err(_) value");

            assert_eq!(expected, actual);
        }

        #[test]
        fn try_from_returns_error_when_git_commit_hash_not_a_string() {
            let message = serde_json::from_str(
                r#"{
                "event_version": "1",
                "event_id": "383bc5f5-d099-40a4-a1a9-8c8a97559479",
                "application_name": "Test application",
                "git_branch": "master",
                "git_commit_hash": false,
                "tool_name": "example tool",
                "tool_output": "{}",
                "output_format": "JSON",
                "start_time": "2019-09-13T19:35:38+00:00",
                "end_time": "2019-09-13T19:37:14+00:00",
                "environment": "Local",
                "tool_version": "1.0"
            }"#,
            )
            .unwrap();

            let expected = ValidationError::git_commit_hash_not_a_string();
            let actual = ToolReport::try_from(&message).expect_err("expected Err(_) value");

            assert_eq!(expected, actual);
        }

        #[test]
        fn try_from_returns_error_when_tool_name_not_a_string() {
            let message = serde_json::from_str(
                r#"{
                "event_version": "1",
                "event_id": "383bc5f5-d099-40a4-a1a9-8c8a97559479",
                "application_name": "Test application",
                "git_branch": "master",
                "git_commit_hash": "e99f715d0fe787cd43de967b8a79b56960fed3e5",
                "tool_name": false,
                "tool_output": "{}",
                "output_format": "JSON",
                "start_time": "2019-09-13T19:35:38+00:00",
                "end_time": "2019-09-13T19:37:14+00:00",
                "environment": "Local",
                "tool_version": "1.0"
            }"#,
            )
            .unwrap();

            let expected = ValidationError::tool_name_not_a_string();
            let actual = ToolReport::try_from(&message).expect_err("expected Err(_) value");

            assert_eq!(expected, actual);
        }

        #[test]
        fn try_from_returns_error_when_tool_output_format_not_a_string() {
            let message = serde_json::from_str(
                r#"{
                "event_version": "1",
                "event_id": "383bc5f5-d099-40a4-a1a9-8c8a97559479",
                "application_name": "Test application",
                "git_branch": "master",
                "git_commit_hash": "e99f715d0fe787cd43de967b8a79b56960fed3e5",
                "tool_name": "example tool",
                "tool_output": "{}",
                "output_format": false,
                "start_time": "2019-09-13T19:35:38+00:00",
                "end_time": "2019-09-13T19:37:14+00:00",
                "environment": "Local",
                "tool_version": "1.0"
            }"#,
            )
            .unwrap();

            let expected = ValidationError::tool_output_format_not_a_string();
            let actual = ToolReport::try_from(&message).expect_err("expected Err(_) value");

            assert_eq!(expected, actual);
        }

        #[test]
        fn try_from_returns_error_when_environment_not_a_string() {
            let message = serde_json::from_str(
                r#"{
                "event_version": "1",
                "event_id": "383bc5f5-d099-40a4-a1a9-8c8a97559479",
                "application_name": "Test application",
                "git_branch": "master",
                "git_commit_hash": "e99f715d0fe787cd43de967b8a79b56960fed3e5",
                "tool_name": "example tool",
                "tool_output": "{}",
                "output_format": "JSON",
                "start_time": "2019-09-13T19:35:38+00:00",
                "end_time": "2019-09-13T19:37:14+00:00",
                "environment": false,
                "tool_version": "1.0"
            }"#,
            )
            .unwrap();

            let expected = ValidationError::environment_not_a_string();
            let actual = ToolReport::try_from(&message).expect_err("expected Err(_) value");

            assert_eq!(expected, actual);
        }

        #[test]
        fn try_from_returns_error_when_tool_version_present_but_not_a_string() {
            let message = serde_json::from_str(
                r#"{
                "event_version": "1",
                "event_id": "383bc5f5-d099-40a4-a1a9-8c8a97559479",
                "application_name": "Test application",
                "git_branch": "master",
                "git_commit_hash": "e99f715d0fe787cd43de967b8a79b56960fed3e5",
                "tool_name": "example tool",
                "tool_output": "{}",
                "output_format": "JSON",
                "start_time": "2019-09-13T19:35:38+00:00",
                "end_time": "2019-09-13T19:37:14+00:00",
                "environment": "Local",
                "tool_version": false
            }"#,
            )
            .unwrap();

            let expected = ValidationError::tool_version_not_a_string();
            let actual = ToolReport::try_from(&message).expect_err("expected Err(_) value");

            assert_eq!(expected, actual);
        }

        #[test]
        fn try_from_returns_error_when_tool_output_not_a_string() {
            let message = serde_json::from_str(
                r#"{
                "event_version": "1",
                "event_id": "383bc5f5-d099-40a4-a1a9-8c8a97559479",
                "application_name": "Test application",
                "git_branch": "master",
                "git_commit_hash": "e99f715d0fe787cd43de967b8a79b56960fed3e5",
                "tool_name": "example tool",
                "tool_output": false,
                "output_format": "JSON",
                "start_time": "2019-09-13T19:35:38+00:00",
                "end_time": "2019-09-13T19:37:14+00:00",
                "environment": "Local",
                "tool_version": "1.0"
            }"#,
            )
            .unwrap();

            let expected = ValidationError::tool_output_not_a_string();
            let actual = ToolReport::try_from(&message).expect_err("expected Err(_) value");

            assert_eq!(expected, actual);
        }

        #[test]
        fn suppressed_issue_collection_can_be_parsed_from_valid_json() {
            let message: Value = serde_json::from_str(
                r#"{
                "event_version": "1",
                "event_id": "383bc5f5-d099-40a4-a1a9-8c8a97559479",
                "application_name": "Test application",
                "git_branch": "master",
                "git_commit_hash": "e99f715d0fe787cd43de967b8a79b56960fed3e5",
                "tool_name": "example tool",
                "tool_output": "{}",
                "output_format": "JSON",
                "start_time": "2019-09-13T19:35:38+00:00",
                "end_time": "2019-09-13T19:37:14+00:00",
                "environment": "Local",
                "tool_version": "1.0",
                "suppressed_issues": [{
                    "issue_hash": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
                    "suppression_reason": "Test issue",
                    "expiry_date": "2020-05-12T00:00:00+00:00"
                }]
            }"#,
            )
            .unwrap();
            ToolReport::try_from(&message).expect("Expected Ok(_)");
        }

        #[test]
        fn suppressed_issue_collection_can_not_be_parsed_from_invalid_json_type() {
            let message: Value = serde_json::from_str(
                r#"{
                "event_version": "1",
                "event_id": "383bc5f5-d099-40a4-a1a9-8c8a97559479",
                "application_name": "Test application",
                "git_branch": "master",
                "git_commit_hash": "e99f715d0fe787cd43de967b8a79b56960fed3e5",
                "tool_name": "example tool",
                "tool_output": "{}",
                "output_format": "JSON",
                "start_time": "2019-09-13T19:35:38+00:00",
                "end_time": "2019-09-13T19:37:14+00:00",
                "environment": "Local",
                "tool_version": "1.0",
                "suppressed_issues": {}
            }"#,
            )
            .unwrap();
            let expected = ValidationError::suppressed_issues_not_an_array();
            let actual = ToolReport::try_from(&message).expect_err("Expected Err(_)");
            assert_eq!(expected, actual);
        }

        #[test]
        fn suppressed_issue_hash_can_not_be_parsed_from_invalid_json_type() {
            let message: Value = serde_json::from_str(
                r#"{
                "event_version": "1",
                "event_id": "383bc5f5-d099-40a4-a1a9-8c8a97559479",
                "application_name": "Test application",
                "git_branch": "master",
                "git_commit_hash": "e99f715d0fe787cd43de967b8a79b56960fed3e5",
                "tool_name": "example tool",
                "tool_output": "{}",
                "output_format": "JSON",
                "start_time": "2019-09-13T19:35:38+00:00",
                "end_time": "2019-09-13T19:37:14+00:00",
                "environment": "Local",
                "tool_version": "1.0",
                "suppressed_issues": [{
                    "issue_hash": true,
                    "suppression_reason": "Test issue",
                    "expiry_date": "2020-05-12T00:00:00+00:00"
                }]
            }"#,
            )
            .unwrap();
            let expected = ValidationError::issue_hash_not_a_string();
            let actual = ToolReport::try_from(&message).expect_err("Expected Err(_)");
            assert_eq!(expected, actual);
        }

        #[test]
        fn suppressed_issue_expiry_date_can_not_be_parsed_from_invalid_json_type() {
            let message: Value = serde_json::from_str(
                r#"{
                "event_version": "1",
                "event_id": "383bc5f5-d099-40a4-a1a9-8c8a97559479",
                "application_name": "Test application",
                "git_branch": "master",
                "git_commit_hash": "e99f715d0fe787cd43de967b8a79b56960fed3e5",
                "tool_name": "example tool",
                "tool_output": "{}",
                "output_format": "JSON",
                "start_time": "2019-09-13T19:35:38+00:00",
                "end_time": "2019-09-13T19:37:14+00:00",
                "environment": "Local",
                "tool_version": "1.0",
                "suppressed_issues": [{
                    "issue_hash": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
                    "suppression_reason": "Test issue",
                    "expiry_date": true
                }]
            }"#,
            )
            .unwrap();
            let expected = ValidationError::expiry_date_not_a_string();
            let actual = ToolReport::try_from(&message).expect_err("Expected Err(_)");
            assert_eq!(expected, actual);
        }

        #[test]
        fn suppressed_issue_suppression_reason_can_not_be_parsed_from_invalid_json_type() {
            let message: Value = serde_json::from_str(
                r#"{
                "event_version": "1",
                "event_id": "383bc5f5-d099-40a4-a1a9-8c8a97559479",
                "application_name": "Test application",
                "git_branch": "master",
                "git_commit_hash": "e99f715d0fe787cd43de967b8a79b56960fed3e5",
                "tool_name": "example tool",
                "tool_output": "{}",
                "output_format": "JSON",
                "start_time": "2019-09-13T19:35:38+00:00",
                "end_time": "2019-09-13T19:37:14+00:00",
                "environment": "Local",
                "tool_version": "1.0",
                "suppressed_issues": [{
                    "issue_hash": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
                    "suppression_reason": true,
                    "expiry_date": "2020-05-12T00:00:00+00:00"
                }]
            }"#,
            )
            .unwrap();

            let expected = ValidationError::suppression_reason_not_a_string();
            let actual = ToolReport::try_from(&message).expect_err("Expected Err(_)");
            assert_eq!(expected, actual);
        }

        #[test]
        fn suppressed_issue_expiry_date_can_be_parsed_from_json_when_null() {
            let message: Value = serde_json::from_str(
                r#"{
                "event_version": "1",
                "event_id": "383bc5f5-d099-40a4-a1a9-8c8a97559479",
                "application_name": "Test application",
                "git_branch": "master",
                "git_commit_hash": "e99f715d0fe787cd43de967b8a79b56960fed3e5",
                "tool_name": "example tool",
                "tool_output": "{}",
                "output_format": "JSON",
                "start_time": "2019-09-13T19:35:38+00:00",
                "end_time": "2019-09-13T19:37:14+00:00",
                "environment": "Local",
                "tool_version": "1.0",
                "suppressed_issues": [{
                    "issue_hash": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
                    "suppression_reason": "Test issue",
                    "expiry_date": null
                }]
            }"#,
            )
            .unwrap();

            ToolReport::try_from(&message).expect("Expected Ok(_)");
        }
    }

    #[test]
    fn try_from_returns_error_when_application_name_empty() {
        let message = "".to_owned();
        let expected = ValidationError::application_name_empty();
        let actual = ApplicationName::try_from(message).expect_err("expected Err(_) value");

        assert_eq!(expected, actual);
    }

    #[test]
    fn try_from_returns_error_when_git_branch_name_empty() {
        let message = "".to_owned();
        let expected = ValidationError::git_branch_name_empty();
        let actual = GitBranch::try_from(Some(message)).expect_err("expected Err(_) value");

        assert_eq!(expected, actual);
    }

    #[test]
    fn try_from_returns_error_when_git_commit_hash_empty() {
        let message = "".to_owned();
        let expected = ValidationError::git_commit_hash_empty();
        let actual = GitCommitHash::try_from(message).expect_err("expected Err(_) value");

        assert_eq!(expected, actual);
    }

    #[test]
    fn try_from_returns_error_when_git_commit_hash_not_valid_string() {
        let message = "zzz".to_owned();
        let expected = ValidationError::git_commit_hash_not_valid();
        let actual = GitCommitHash::try_from(message).expect_err("expected Err(_) value");

        assert_eq!(expected, actual);
    }

    #[test]
    fn try_from_returns_error_when_tool_name_empty() {
        let message = "".to_owned();
        let expected = ValidationError::tool_name_empty();
        let actual = ToolName::try_from(message).expect_err("expected Err(_) value");

        assert_eq!(expected, actual);
    }

    #[test]
    fn try_from_returns_error_when_tool_output_empty() {
        let message = "".to_owned();
        let expected = ValidationError::tool_output_empty();
        let actual = ToolOutput::try_from(message).expect_err("expected Err(_) value");

        assert_eq!(expected, actual);
    }

    #[test]
    fn try_from_returns_error_when_tool_output_format_empty() {
        let message = serde_json::from_str(
            r#"{
            "event_version": "1",
            "event_id": "383bc5f5-d099-40a4-a1a9-8c8a97559479",
            "application_name": "Test application",
            "git_branch": "master",
            "git_commit_hash": "e99f715d0fe787cd43de967b8a79b56960fed3e5",
            "tool_name": "example tool",
            "tool_output": "{}",
            "output_format": "",
            "start_time": "2019-09-13T19:35:38+00:00",
            "end_time": "2019-09-13T19:37:14+00:00",
            "environment": "Local",
            "tool_version": "1.0",
            "suppressed_issues": [{
                "issue_hash": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
                "suppression_reason": "Test issue",
                "expiry_date": "2020-05-12T00:00:00+00:00"
            }]
        }"#,
        )
        .unwrap();

        let expected = ValidationError::tool_output_format_empty();
        let actual = ToolReport::try_from(&message).expect_err("expected Err(_) value");

        assert_eq!(expected, actual);
    }

    #[test]
    fn try_from_returns_error_when_tool_output_format_not_a_valid_option() {
        let message = serde_json::from_str(
            r#"{
            "event_version": "1",
            "event_id": "383bc5f5-d099-40a4-a1a9-8c8a97559479",
            "application_name": "Test application",
            "git_branch": "master",
            "git_commit_hash": "e99f715d0fe787cd43de967b8a79b56960fed3e5",
            "tool_name": "example tool",
            "tool_output": "{}",
            "output_format": "msgpack",
            "start_time": "2019-09-13T19:35:38+00:00",
            "end_time": "2019-09-13T19:37:14+00:00",
            "environment": "Local",
            "tool_version": "1.0",
            "suppressed_issues": [{
                "issue_hash": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
                "suppression_reason": "Test issue",
                "expiry_date": "2020-05-12T00:00:00+00:00"
            }]
        }"#,
        )
        .unwrap();

        let expected = ValidationError::tool_output_format_invalid();
        let actual = ToolReport::try_from(&message).expect_err("expected Err(_) value");

        assert_eq!(expected, actual);
    }

    #[test]
    fn try_from_returns_error_when_start_time_not_a_timestamp() {
        let message = serde_json::from_str(
            r#"{
            "event_version": "1",
            "event_id": "383bc5f5-d099-40a4-a1a9-8c8a97559479",
            "application_name": "Test application",
            "git_branch": "master",
            "git_commit_hash": "e99f715d0fe787cd43de967b8a79b56960fed3e5",
            "tool_name": "example tool",
            "tool_output": "{}",
            "output_format": "JSON",
            "start_time": "not a timestamp",
            "end_time": "2019-09-13T19:37:14+00:00",
            "environment": "Local",
            "tool_version": "1.0",
            "suppressed_issues": [{
                "issue_hash": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
                "suppression_reason": "Test issue",
                "expiry_date": "2020-05-12T00:00:00+00:00"
            }]
        }"#,
        )
        .unwrap();

        let expected = ValidationError::start_time_not_a_timestamp();
        let actual = ToolReport::try_from(&message).expect_err("expected Err(_) value");

        assert_eq!(expected, actual);
    }

    #[test]
    fn try_from_returns_error_when_end_time_not_a_timestamp() {
        let message = serde_json::from_str(
            r#"{
            "event_version": "1",
            "event_id": "383bc5f5-d099-40a4-a1a9-8c8a97559479",
            "application_name": "Test application",
            "git_branch": "master",
            "git_commit_hash": "e99f715d0fe787cd43de967b8a79b56960fed3e5",
            "tool_name": "example tool",
            "tool_output": "{}",
            "output_format": "JSON",
            "start_time": "2019-09-13T19:35:38+00:00",
            "end_time": "not a timestamp",
            "environment": "Local",
            "tool_version": "1.0",
            "suppressed_issues": [{
                "issue_hash": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
                "suppression_reason": "Test issue",
                "expiry_date": "2020-05-12T00:00:00+00:00"
            }]
        }"#,
        )
        .unwrap();

        let expected = ValidationError::end_time_not_a_timestamp();
        let actual = ToolReport::try_from(&message).expect_err("expected Err(_) value");

        assert_eq!(expected, actual);
    }

    #[test]
    fn try_from_returns_error_when_environment_not_a_valid_option() {
        let message = serde_json::from_str(
            r#"{
            "event_version": "1",
            "event_id": "383bc5f5-d099-40a4-a1a9-8c8a97559479",
            "application_name": "Test application",
            "git_branch": "master",
            "git_commit_hash": "e99f715d0fe787cd43de967b8a79b56960fed3e5",
            "tool_name": "example tool",
            "tool_output": "{}",
            "output_format": "JSON",
            "start_time": "2019-09-13T19:35:38+00:00",
            "end_time": "2019-09-13T19:37:14+00:00",
            "environment": "the moon",
            "tool_version": "1.0",
            "suppressed_issues": [{
                "issue_hash": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
                "suppression_reason": "Test issue",
                "expiry_date": "2020-05-12T00:00:00+00:00"
            }]
        }"#,
        )
        .unwrap();

        let expected = ValidationError::environment_not_a_valid_option();
        let actual = ToolReport::try_from(&message).expect_err("expected Err(_) value");

        assert_eq!(expected, actual);
    }

    #[test]
    fn try_from_returns_error_when_environment_empty() {
        let message = serde_json::from_str(
            r#"{
            "event_version": "1",
            "event_id": "383bc5f5-d099-40a4-a1a9-8c8a97559479",
            "application_name": "Test application",
            "git_branch": "master",
            "git_commit_hash": "e99f715d0fe787cd43de967b8a79b56960fed3e5",
            "tool_name": "example tool",
            "tool_output": "{}",
            "output_format": "JSON",
            "start_time": "2019-09-13T19:35:38+00:00",
            "end_time": "2019-09-13T19:37:14+00:00",
            "environment": "",
            "tool_version": "1.0",
            "suppressed_issues": [{
                "issue_hash": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
                "suppression_reason": "Test issue",
                "expiry_date": "2020-05-12T00:00:00+00:00"
            }]
        }"#,
        )
        .unwrap();

        let expected = ValidationError::environment_empty();
        let actual = ToolReport::try_from(&message).expect_err("expected Err(_) value");

        assert_eq!(expected, actual);
    }

    #[test]
    fn try_from_returns_error_when_tool_version_present_but_empty() {
        let message = serde_json::from_str(
            r#"{
            "event_version": "1",
            "event_id": "383bc5f5-d099-40a4-a1a9-8c8a97559479",
            "application_name": "Test application",
            "git_branch": "master",
            "git_commit_hash": "e99f715d0fe787cd43de967b8a79b56960fed3e5",
            "tool_name": "example tool",
            "tool_output": "{}",
            "output_format": "JSON",
            "start_time": "2019-09-13T19:35:38+00:00",
            "end_time": "2019-09-13T19:37:14+00:00",
            "environment": "Local",
            "tool_version": "",
            "suppressed_issues": [{
                "issue_hash": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
                "suppression_reason": "Test issue",
                "expiry_date": "2020-05-12T00:00:00+00:00"
            }]
        }"#,
        )
        .unwrap();

        let expected = ValidationError::tool_version_present_but_empty();
        let actual = ToolReport::try_from(&message).expect_err("expected Err(_) value");

        assert_eq!(expected, actual);
    }

    #[test]
    fn tool_report_can_round_trip_to_avro_and_back() {
        let message = serde_json::from_str(
            r#"{
            "event_version": "1",
            "event_id": "383bc5f5-d099-40a4-a1a9-8c8a97559479",
            "application_name": "Test application",
            "git_branch": "master",
            "git_commit_hash": "e99f715d0fe787cd43de967b8a79b56960fed3e5",
            "tool_name": "example tool",
            "tool_output": "{}",
            "output_format": "JSON",
            "start_time": "2019-09-13T19:35:38+00:00",
            "end_time": "2019-09-13T19:37:14+00:00",
            "environment": "Local",
            "tool_version": "1.0",
            "suppressed_issues": [{
                "issue_hash": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
                "suppression_reason": "Test issue",
                "expiry_date": "2020-05-12T00:00:00+00:00"
            }]
        }"#,
        )
        .unwrap();

        let report = ToolReport::try_from(&message).expect("expected Ok(_) value");

        let expected = ToolReport::try_from(&message).expect("expected Ok(_) value");

        let schema = Schema::parse_str(TOOL_REPORT_SCHEMA).unwrap();
        let mut writer = Writer::new(&schema, Vec::new());
        writer.append_ser(report).unwrap();
        writer.flush().unwrap();

        let input = writer.into_inner();

        let reader = Reader::with_schema(&schema, &input[..]).unwrap();
        let mut input_records = reader.into_iter().collect::<Vec<_>>();
        let parsed_record = input_records.remove(0).unwrap();

        let actual = ToolReport::try_from(parsed_record).expect("expected Ok(_) value");

        assert_eq!(expected, actual);
    }

    #[test]
    fn valid_issue_hash_can_be_parsed_from_string() {
        let issue_hash = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_owned();
        IssueHash::try_from(issue_hash).expect("Expected Ok(_) value");
    }

    #[test]
    fn invalid_issue_hash_length_can_not_be_parsed_from_string() {
        let issue_hash = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b".to_owned();
        let expected = ValidationError::issue_hash_not_valid();
        let actual = IssueHash::try_from(issue_hash).expect_err("Expected Err(_)");
        assert_eq!(expected, actual);
    }

    #[test]
    fn invalid_issue_hash_characters_can_not_be_parsed_from_string() {
        let issue_hash = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852B!^?".to_owned();
        let expected = ValidationError::issue_hash_not_valid();
        let actual = IssueHash::try_from(issue_hash).expect_err("Expected Err(_)");
        assert_eq!(expected, actual);
    }

    #[test]
    fn suppression_reason_can_not_be_parsed_from_empty_string() {
        let suppression_reason = "".to_owned();
        let expected = ValidationError::suppression_reason_empty();
        let actual = SuppressionReason::try_from(suppression_reason).expect_err("Expected Err(_)");
        assert_eq!(expected, actual);
    }

    #[test]
    fn suppression_expiry_date_can_not_be_parsed_from_invalid_string() {
        let expiry_date = Some("42/17/3000 not a date".to_owned());
        let expected = ValidationError::expiry_date_not_a_valid_date();
        let actual = ExpiryDate::try_from(expiry_date).expect_err("Expected Err(_)");
        assert_eq!(expected, actual);
    }
}
