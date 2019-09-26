pub mod avro_schema {
    pub const TOOL_REPORT_SCHEMA: &'static str = r#"
        {
            "type": "record",
            "name": "ToolReport",
            "fields": [
                {"name": "application_name", "type": "string"},
                {"name": "git_branch", "type": "string"},
                {"name": "git_commit_hash", "type": "string"},
                {"name": "tool_name", "type": "string"},
                {"name": "tool_output", "type": "string"},
                {"name": "output_format", "type": "string"},
                {"name": "start_time", "type": "string"},
                {"name": "end_time", "type": "string"},
                {"name": "environment", "type": "string"},
                {"name": "tool_version", "type": ["null", "string"]}
            ]
        }
    "#;

    #[cfg(test)]
    mod tests {
        use avro_rs::Schema;
        use super::*;

        #[test]
        fn schema_is_valid() {
            Schema::parse_str(TOOL_REPORT_SCHEMA)
                .expect("expected Ok(_) value");
        }
    }
}

pub mod validation {
    use lambda_http::{Body, IntoResponse, Response};
    use http::status::StatusCode;
    use serde::Serialize;

    #[derive(Debug, PartialEq, Serialize)]
    pub struct ValidationError {
        pub error_code: u8,
        pub error_message: String,
    }

    impl ValidationError {
        pub fn body_empty() -> ValidationError {
            ValidationError {
                error_code: 100,
                error_message: "Request body empty".into(),
            }
        }

        pub fn body_media_type_incorrect() -> ValidationError {
            ValidationError {
                error_code: 101,
                error_message: "Request body not correct media type".into(),
            }
        }

        pub fn application_name_empty() -> ValidationError {
            ValidationError {
                error_code: 111,
                error_message: "Application name present but empty".into(),
            }
        }

        pub fn application_name_missing() -> ValidationError {
            ValidationError {
                error_code: 102,
                error_message: "Application name required".into(),
            }
        }

        pub fn application_name_not_a_string() -> ValidationError {
            ValidationError {
                error_code: 112,
                error_message: "Application name not a valid string".into(),
            }
        }

        pub fn git_branch_name_empty() -> ValidationError {
            ValidationError {
                error_code: 113,
                error_message: "Git branch name present but empty".into(),
            }
        }

        pub fn git_branch_name_missing() -> ValidationError{
            ValidationError {
                error_code: 103,
                error_message: "Git branch name required".into(),
            }
        }

        pub fn git_branch_name_not_a_string() -> ValidationError{
            ValidationError {
                error_code: 114,
                error_message: "Git branch name not a valid string".into(),
            }
        }

        pub fn git_commit_hash_empty() -> ValidationError {
            ValidationError {
                error_code: 115,
                error_message: "Git commit hash present but empty".into(),
            }
        }

        pub fn git_commit_hash_missing() -> ValidationError {
            ValidationError {
                error_code: 104,
                error_message: "Git commit hash required".into(),
            }
        }

        pub fn git_commit_hash_not_a_string() -> ValidationError {
            ValidationError {
                error_code: 117,
                error_message: "Git commit hash not a valid string".into(),
            }
        }

        pub fn git_commit_hash_not_valid() -> ValidationError {
            ValidationError {
                error_code: 116,
                error_message: "Git commit hash not valid".into(),
            }
        }

        pub fn tool_name_empty() -> ValidationError {
            ValidationError {
                error_code: 118,
                error_message: "Tool name present but empty".into(),
            }
        }

        pub fn tool_name_missing() -> ValidationError {
            ValidationError {
                error_code: 105,
                error_message: "Tool name required".into(),
            }
        }

        pub fn tool_name_not_a_string() -> ValidationError {
            ValidationError {
                error_code: 119,
                error_message: "Tool name not a valid string".into(),
            }
        }

        pub fn tool_output_empty() -> ValidationError {
            ValidationError {
                error_code: 120,
                error_message: "Tool output present but empty".into(),
            }
        }

        pub fn tool_output_missing() -> ValidationError {
            ValidationError {
                error_code: 106,
                error_message: "Tool output required".into(),
            }
        }

        pub fn tool_output_not_a_string() -> ValidationError {
            ValidationError {
                error_code: 121,
                error_message: "Tool output not a valid string".into(),
            }
        }

        pub fn tool_output_format_empty() -> ValidationError {
            ValidationError {
                error_code: 122,
                error_message: "Tool output format present but empty".into(),
            }
        }

        pub fn tool_output_format_missing() -> ValidationError {
            ValidationError {
                error_code: 107,
                error_message: "Tool output format required".into(),
            }
        }

        pub fn tool_output_format_not_a_string() -> ValidationError {
            ValidationError {
                error_code: 123,
                error_message: "Tool output format not a valid string".into(),
            }
        }

        pub fn tool_output_format_invalid() -> ValidationError {
            ValidationError {
                error_code: 124,
                error_message: "Tool output format not acceptable".into(),
            }
        }

        pub fn start_time_missing() -> ValidationError {
            ValidationError {
                error_code: 108,
                error_message: "Start time required".into(),
            }
        }

        pub fn start_time_not_a_timestamp() -> ValidationError {
            ValidationError {
                error_code: 125,
                error_message: "Start time not a valid timestamp".into(),
            }
        }

        pub fn end_time_missing() -> ValidationError {
            ValidationError {
                error_code: 109,
                error_message: "End time required".into(),
            }
        }

        pub fn end_time_not_a_timestamp() -> ValidationError {
            ValidationError {
                error_code: 126,
                error_message: "End time not a valid timestamp".into(),
            }
        }

        pub fn environment_not_a_valid_option() -> ValidationError {
            ValidationError {
                error_code: 128,
                error_message: "Environment not a valid option".into(),
            }
        }

        pub fn environment_missing() -> ValidationError {
            ValidationError {
                error_code: 110,
                error_message: "Environment required".into(),
            }
        }

        pub fn environment_not_a_string() -> ValidationError {
            ValidationError {
                error_code: 127,
                error_message: "Environment not a valid string".into(),
            }
        }

        pub fn tool_version_not_a_string() -> ValidationError {
            ValidationError {
                error_code: 130,
                error_message: "Tool version not a valid string".into(),
            }
        }

        pub fn tool_version_present_but_empty() -> ValidationError {
            ValidationError {
                error_code: 129,
                error_message: "Tool version present but empty".into(),
            }
        }

        pub fn avro_schema_validation_failed() -> ValidationError {
            ValidationError {
                error_code: 130,
                error_message: "Tried to deserialise a ToolReport from Avro but value didn't pass schema validation".into()
            }
        }
    }

    impl IntoResponse for ValidationError {
        fn into_response(self) -> Response<Body> {
            Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Body::from(serde_json::to_string(&self).unwrap()))
                .unwrap()
        }
    }
}

pub mod tool_report {
    use crate::avro_schema::TOOL_REPORT_SCHEMA;
    use crate::validation::ValidationError;

    use std::convert::TryFrom;

    use avro_rs::types::{Record, ToAvro};
    use avro_rs::schema::Schema;
    use chrono::{DateTime, Utc};
    use failure::err_msg;
    use serde_json::value::Value;
    use regex::Regex;

    #[allow(dead_code)]
    #[derive(Clone, Debug, PartialEq)]
    pub struct ToolReport {
        pub application_name: ApplicationName,
        pub git_branch: GitBranch,
        pub git_commit_hash: GitCommitHash,
        pub tool_name: ToolName,
        pub tool_output: ToolOutput,
        pub output_format: OutputFormat,
        pub start_time: StartTime,
        pub end_time: EndTime,
        pub environment: Environment,
        pub tool_version:ToolVersion
    }

    #[derive(Clone, Debug, PartialEq)]
    pub struct ApplicationName(String);

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

    #[derive(Clone, Debug, PartialEq)]
    pub struct GitBranch(String);
    
    impl TryFrom<String> for GitBranch{
        type Error = ValidationError;

        fn try_from(value: String) -> Result<Self, Self::Error> {
            if value.is_empty() {
                Err(ValidationError::git_branch_name_empty())
            } else {
                Ok(GitBranch(value))
            }
        }
    }

    impl std::fmt::Display for GitBranch {
        fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(f, "{}", self.0)
        }
    }

    #[derive(Clone, Debug, PartialEq)]
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

    #[derive(Clone, Debug, PartialEq)]
    pub struct ToolName(String);

    impl TryFrom<String> for ToolName{
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

    #[derive(Clone, Debug, PartialEq)]
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

    #[derive(Clone, Debug, PartialEq)]
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
                None => write!(f, "")
            }
        }
    }

    #[derive(Clone, Debug, PartialEq)]
    pub enum OutputFormat {
        JSON,
        PlainText,
    }

    impl std::fmt::Display for OutputFormat {
        fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            match self {
                OutputFormat::JSON => write!(f, "Json"),
                OutputFormat::PlainText => write!(f, "PlainText")
            }
        }
    }

    #[derive(Clone, Debug, PartialEq)]
    pub struct StartTime(DateTime<Utc>);
    
    impl std::fmt::Display for StartTime{
        fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(f, "{}", self.0.to_rfc3339())
        }
    }

    #[derive(Clone, Debug, PartialEq)]
    pub struct EndTime(DateTime<Utc>);
    
    impl std::fmt::Display for EndTime{
        fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(f, "{}", self.0.to_rfc3339())
        }
    }

    #[derive(Clone, Debug, PartialEq)]
    pub enum Environment {
        Local,
        CI,
    }

    impl std::fmt::Display for Environment {
        fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            match self {
                Environment::Local => write!(f, "Local"),
                Environment::CI => write!(f, "CI")
            }
        }
    }

    impl TryFrom<&Value> for ToolReport {
        type Error = ValidationError;

        fn try_from(json_value: &Value) -> Result<Self, Self::Error> {
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
            Ok(ToolReport {
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
            })
        }
    }

    impl<'a> TryFrom<avro_rs::types::Value> for ToolReport {
        type Error = failure::Error;

        fn try_from(value: avro_rs::types::Value) -> Result<Self, Self::Error> {
            let schema = Schema::parse_str(TOOL_REPORT_SCHEMA).unwrap();
            let resolved_value = value.resolve(&schema)
                .map_err(|err| err_msg(format!("Error resolving Avro schema: {}", err.name().unwrap())))?;

            if let avro_rs::types::Value::Record(record) = resolved_value {
                let mut fields = record.iter();
                let application_name = ApplicationName::try_from(fields.find(|&x| x.0 == "application_name").unwrap().1.clone())
                    .map_err(|err| err_msg(err.error_message))?;
                let git_branch = GitBranch::try_from(fields.find(|&x| x.0 == "git_branch").unwrap().1.clone())
                    .map_err(|err| err_msg(err.error_message))?;
                let git_commit_hash = GitCommitHash::try_from(fields.find(|&x| x.0 == "git_commit_hash").unwrap().1.clone())
                    .map_err(|err| err_msg(err.error_message))?;
                let tool_name = ToolName::try_from(fields.find(|&x| x.0 == "tool_name").unwrap().1.clone())
                    .map_err(|err| err_msg(err.error_message))?;
                let tool_output = ToolOutput::try_from(fields.find(|&x| x.0 == "tool_output").unwrap().1.clone())
                    .map_err(|err| err_msg(err.error_message))?;
                let output_format = OutputFormat::try_from(fields.find(|&x| x.0 == "output_format").unwrap().1.clone())
                    .map_err(|err| err_msg(err.error_message))?;
                let start_time = StartTime::try_from(fields.find(|&x| x.0 == "start_time").unwrap().1.clone())
                    .map_err(|err| err_msg(err.error_message))?;
                let end_time = EndTime::try_from(fields.find(|&x| x.0 == "end_time").unwrap().1.clone())
                    .map_err(|err| err_msg(err.error_message))?;
                let environment = Environment::try_from(fields.find(|&x| x.0 == "environment").unwrap().1.clone())
                    .map_err(|err| err_msg(err.error_message))?;
                let tool_version = ToolVersion::try_from(fields.find(|&x| x.0 == "tool_version").unwrap().1.clone())
                    .map_err(|err| err_msg(err.error_message))?;

                Ok(
                    ToolReport {
                        application_name,
                        git_branch,
                        git_commit_hash,
                        tool_name,
                        tool_output,
                        output_format,
                        start_time,
                        end_time,
                        environment,
                        tool_version
                    }
                )
            } else {
                Err(err_msg("Something went wrong decoding Avro record"))
            }
        }
    }

    impl TryFrom<avro_rs::types::Value> for ApplicationName {
        type Error = ValidationError;

        fn try_from(value: avro_rs::types::Value) -> Result<Self, Self::Error> {
            match value {
                avro_rs::types::Value::String(s) => ApplicationName::try_from(s),
                _ => Err(ValidationError::application_name_not_a_string())
            }
        }
    }

    impl TryFrom<avro_rs::types::Value> for GitBranch {
        type Error = ValidationError;

        fn try_from(value: avro_rs::types::Value) -> Result<Self, Self::Error> {
            match value {
                avro_rs::types::Value::String(s) => GitBranch::try_from(s),
                _ => Err(ValidationError::git_branch_name_not_a_string())
            }
        }
    }

    impl TryFrom<avro_rs::types::Value> for GitCommitHash {
        type Error = ValidationError;

        fn try_from(value: avro_rs::types::Value) -> Result<Self, Self::Error> {
            match value {
                avro_rs::types::Value::String(s) => GitCommitHash::try_from(s),
                _ => Err(ValidationError::git_commit_hash_not_a_string())
            }
        }
    }

    impl TryFrom<avro_rs::types::Value> for ToolName {
        type Error = ValidationError;

        fn try_from(value: avro_rs::types::Value) -> Result<Self, Self::Error> {
            match value {
                avro_rs::types::Value::String(s) => ToolName::try_from(s),
                _ => Err(ValidationError::tool_name_not_a_string())
            }
        }
    }

    impl TryFrom<avro_rs::types::Value> for ToolOutput {
        type Error = ValidationError;

        fn try_from(value: avro_rs::types::Value) -> Result<Self, Self::Error> {
            match value {
                avro_rs::types::Value::String(s) => ToolOutput::try_from(s),
                _ => Err(ValidationError::tool_output_not_a_string())
            }
        }
    }
    
    impl TryFrom<avro_rs::types::Value> for OutputFormat {
        type Error = ValidationError;

        fn try_from(value: avro_rs::types::Value) -> Result<Self, Self::Error> {
            let s = match value {
                avro_rs::types::Value::String(s) => Ok(s),
                _ => Err(ValidationError::tool_output_format_not_a_string())
            }?;

            if s.is_empty() {
                return Err(ValidationError::tool_output_format_empty())
            }

            match s.as_ref() {
                "Json" => Ok(OutputFormat::JSON),
                "PlainText" => Ok(OutputFormat::PlainText),
                _ => Err(ValidationError::tool_output_format_invalid()),
            }
        }
    }

    impl TryFrom<avro_rs::types::Value> for StartTime {
        type Error = ValidationError;

        fn try_from(value: avro_rs::types::Value) -> Result<Self, Self::Error> {
            match value {
                avro_rs::types::Value::String(s) =>
                    DateTime::parse_from_rfc3339(&s)
                        .map(|dt| StartTime(DateTime::<Utc>::from(dt)))
                        .map_err(|_| ValidationError::start_time_not_a_timestamp()),
                _ => Err(ValidationError::start_time_not_a_timestamp())
            }
        }
    }

    impl TryFrom<avro_rs::types::Value> for EndTime {
        type Error = ValidationError;

        fn try_from(value: avro_rs::types::Value) -> Result<Self, Self::Error> {
            match value {
                avro_rs::types::Value::String(s) =>
                    DateTime::parse_from_rfc3339(&s)
                        .map(|dt| EndTime(DateTime::<Utc>::from(dt)))
                        .map_err(|_| ValidationError::end_time_not_a_timestamp()),
                _ => Err(ValidationError::end_time_not_a_timestamp())
            }
        }
    }

    impl TryFrom<avro_rs::types::Value> for Environment {
        type Error = ValidationError;

        fn try_from(value: avro_rs::types::Value) -> Result<Self, Self::Error> {
            let s = match value {
                avro_rs::types::Value::String(s) => Ok(s),
                _ => Err(ValidationError::environment_not_a_string())
            }?;

            match s.as_ref() {
                "Local" => Ok(Environment::Local),
                "CI" => Ok(Environment::CI),
                _ => Err(ValidationError::environment_not_a_valid_option()),
            }
        }
    }

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

    impl ToAvro for ToolReport {
        fn avro(self) -> avro_rs::types::Value {
            let schema = Schema::parse_str(TOOL_REPORT_SCHEMA).unwrap();
            let mut record = Record::new(&schema).unwrap();
            record.put("application_name", self.application_name.to_string());
            record.put("git_branch", self.git_branch.to_string());
            record.put("git_commit_hash", self.git_commit_hash.to_string());
            record.put("tool_name", self.tool_name.to_string());
            record.put("tool_output", self.tool_output.to_string());
            record.put("output_format", self.output_format.to_string());
            record.put("start_time", self.start_time.to_string());
            record.put("end_time", self.end_time.to_string());
            record.put("environment", self.environment.to_string());
            let tool_version = self.tool_version.to_string();
            if tool_version.is_empty() {
                record.put("tool_version", avro_rs::types::Value::Null);
            } else {
                record.put("tool_version", tool_version);
            }
            avro_rs::types::Value::Record(record.fields)
        }
    }

    impl ToolReport {
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
                Value::Null => Err(ValidationError::git_branch_name_missing()),
                Value::String(value) => Ok(value),
                _ => Err(ValidationError::git_branch_name_not_a_string()),
            }?;
            GitBranch::try_from(value.to_owned())
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
                "Json" => Ok(OutputFormat::JSON),
                "PlainText" => Ok(OutputFormat::PlainText),
                _ => Err(ValidationError::tool_output_format_invalid()),
            }
        }

        fn parse_tool_start_time(
            json_value: &Value,
        ) -> Result<StartTime, ValidationError> {
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
                Value::String(value) => Ok(value),
                _ => Err(ValidationError::environment_not_a_string()),
            }?;

            match value.as_ref() {
                "Local" => Ok(Environment::Local),
                "CI" => Ok(Environment::CI),
                _ => Err(ValidationError::environment_not_a_valid_option()),
            }
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

    #[cfg(test)]
    pub mod tests {
        // TODO: Separate tests based on whether they test the JSON validation or the business logic
        // validation
        use super::*;

        pub mod tool_report {
            use super::*;

            #[test]
            fn try_from_returns_error_when_application_name_missing() {
                let message = serde_json::from_str(r#"{
                    "git_branch": "master",
                    "git_commit_hash": "e99f715d0fe787cd43de967b8a79b56960fed3e5",
                    "tool_name": "example tool",
                    "tool_output": "{}",
                    "output_format": "Json",
                    "start_time": "2019-09-13T19:35:38+00:00",
                    "end_time": "2019-09-13T19:37:14+00:00",
                    "environment": "Local",
                    "tool_version": "1.0"
                }"#).unwrap();

                let expected = ValidationError::application_name_missing();
                let actual = ToolReport::try_from(&message)
                    .expect_err("expected Err(_) value");

                assert_eq!(expected, actual);
            }

            #[test]
            fn try_from_returns_error_when_git_branch_missing() {
                let message = serde_json::from_str(r#"{
                    "application_name": "Test application",
                    "git_commit_hash": "e99f715d0fe787cd43de967b8a79b56960fed3e5",
                    "tool_name": "example tool",
                    "tool_output": "{}",
                    "output_format": "Json",
                    "start_time": "2019-09-13T19:35:38+00:00",
                    "end_time": "2019-09-13T19:37:14+00:00",
                    "environment": "Local",
                    "tool_version": "1.0"
                }"#).unwrap();
                let expected = ValidationError::git_branch_name_missing();
                let actual = ToolReport::try_from(&message)
                    .expect_err("expected Err(_) value");

                assert_eq!(expected, actual);
            }

            #[test]
            fn try_from_returns_error_when_git_commit_hash_missing() {
                let message = serde_json::from_str(r#"{
                    "application_name": "Test application",
                    "git_branch": "master",
                    "tool_name": "example tool",
                    "tool_output": "{}",
                    "output_format": "Json",
                    "start_time": "2019-09-13T19:35:38+00:00",
                    "end_time": "2019-09-13T19:37:14+00:00",
                    "environment": "Local",
                    "tool_version": "1.0"
                }"#).unwrap();

                let expected = ValidationError::git_commit_hash_missing();
                let actual = ToolReport::try_from(&message)
                    .expect_err("expected Err(_) value");

                assert_eq!(expected, actual);
            }

            #[test]
            fn try_from_returns_error_when_tool_name_missing() {
                let message =serde_json::from_str(r#"{
                    "application_name": "Test application",
                    "git_branch": "master",
                    "git_commit_hash": "e99f715d0fe787cd43de967b8a79b56960fed3e5",
                    "tool_output": "{}",
                    "output_format": "Json",
                    "start_time": "2019-09-13T19:35:38+00:00",
                    "end_time": "2019-09-13T19:37:14+00:00",
                    "environment": "Local",
                    "tool_version": "1.0"
                }"#).unwrap();

                let expected = ValidationError::tool_name_missing();
                let actual = ToolReport::try_from(&message)
                    .expect_err("expected Err(_) value");

                assert_eq!(expected, actual);
            }

            #[test]
            fn try_from_returns_error_when_tool_output_missing() {
                let message = serde_json::from_str(r#"{
                    "application_name": "Test application",
                    "git_branch": "master",
                    "git_commit_hash": "e99f715d0fe787cd43de967b8a79b56960fed3e5",
                    "tool_name": "example tool",
                    "output_format": "Json",
                    "start_time": "2019-09-13T19:35:38+00:00",
                    "end_time": "2019-09-13T19:37:14+00:00",
                    "environment": "Local",
                    "tool_version": "1.0"
                }"#).unwrap();

                let expected = ValidationError::tool_output_missing();
                let actual = ToolReport::try_from(&message)
                    .expect_err("expected Err(_) value");

                assert_eq!(expected, actual);
            }

            #[test]
            fn try_from_returns_error_when_tool_output_format_missing() {
                let message = serde_json::from_str(r#"{
                    "application_name": "Test application",
                    "git_branch": "master",
                    "git_commit_hash": "e99f715d0fe787cd43de967b8a79b56960fed3e5",
                    "tool_name": "example tool",
                    "tool_output": "{}",
                    "start_time": "2019-09-13T19:35:38+00:00",
                    "end_time": "2019-09-13T19:37:14+00:00",
                    "environment": "Local",
                    "tool_version": "1.0"
                }"#).unwrap();

                let expected = ValidationError::tool_output_format_missing();
                let actual = ToolReport::try_from(&message)
                    .expect_err("expected Err(_) value");

                assert_eq!(expected, actual);
            }

            #[test]
            fn try_from_returns_error_when_start_time_missing() {
                let message = serde_json::from_str(r#"{
                    "application_name": "Test application",
                    "git_branch": "master",
                    "git_commit_hash": "e99f715d0fe787cd43de967b8a79b56960fed3e5",
                    "tool_name": "example tool",
                    "tool_output": "{}",
                    "output_format": "Json",
                    "end_time": "2019-09-13T19:37:14+00:00",
                    "environment": "Local",
                    "tool_version": "1.0"
                }"#).unwrap();

                let expected = ValidationError::start_time_missing();
                let actual = ToolReport::try_from(&message)
                    .expect_err("expected Err(_) value");

                assert_eq!(expected, actual);
            }

            #[test]
            fn try_from_returns_error_when_end_time_missing() {
                let message = serde_json::from_str(r#"{
                    "application_name": "Test application",
                    "git_branch": "master",
                    "git_commit_hash": "e99f715d0fe787cd43de967b8a79b56960fed3e5",
                    "tool_name": "example tool",
                    "tool_output": "{}",
                    "output_format": "Json",
                    "start_time": "2019-09-13T19:35:38+00:00",
                    "environment": "Local",
                    "tool_version": "1.0"
                }"#).unwrap();

                let expected = ValidationError::end_time_missing();
                let actual = ToolReport::try_from(&message)
                    .expect_err("expected Err(_) value");

                assert_eq!(expected, actual);
            }

            #[test]
            fn try_from_returns_error_when_environment_missing() {
                let message = serde_json::from_str(r#"{
                    "application_name": "Test application",
                    "git_branch": "master",
                    "git_commit_hash": "e99f715d0fe787cd43de967b8a79b56960fed3e5",
                    "tool_name": "example tool",
                    "tool_output": "{}",
                    "output_format": "Json",
                    "start_time": "2019-09-13T19:35:38+00:00",
                    "end_time": "2019-09-13T19:37:14+00:00",
                    "tool_version": "1.0"
                }"#).unwrap();

                let expected = ValidationError::environment_missing();
                let actual = ToolReport::try_from(&message)
                    .expect_err("expected Err(_) value");

                assert_eq!(expected, actual);
            }

            #[test]
            fn try_from_returns_error_when_application_name_not_a_string() {
                let message = serde_json::from_str(r#"{
                    "application_name": false,
                    "git_branch": "master",
                    "git_commit_hash": "e99f715d0fe787cd43de967b8a79b56960fed3e5",
                    "tool_name": "example tool",
                    "tool_output": "{}",
                    "output_format": "Json",
                    "start_time": "2019-09-13T19:35:38+00:00",
                    "end_time": "2019-09-13T19:37:14+00:00",
                    "environment": "Local",
                    "tool_version": "1.0"
                }"#).unwrap();

                let expected = ValidationError::application_name_not_a_string();
                let actual = ToolReport::try_from(&message)
                    .expect_err("expected Err(_) value");

                assert_eq!(expected, actual);
            }

            #[test]
            fn try_from_returns_error_when_git_branch_name_not_a_string() {
                let message = serde_json::from_str(r#"{
                    "application_name": "Test application",
                    "git_branch": false,
                    "git_commit_hash": "e99f715d0fe787cd43de967b8a79b56960fed3e5",
                    "tool_name": "example tool",
                    "tool_output": "{}",
                    "output_format": "Json",
                    "start_time": "2019-09-13T19:35:38+00:00",
                    "end_time": "2019-09-13T19:37:14+00:00",
                    "environment": "Local",
                    "tool_version": "1.0"
                }"#).unwrap();

                let expected = ValidationError::git_branch_name_not_a_string();
                let actual = ToolReport::try_from(&message)
                    .expect_err("expected Err(_) value");

                assert_eq!(expected, actual);
            }

            #[test]
            fn try_from_returns_error_when_git_commit_hash_not_a_string() {
                let message = serde_json::from_str(r#"{
                    "application_name": "Test application",
                    "git_branch": "master",
                    "git_commit_hash": false,
                    "tool_name": "example tool",
                    "tool_output": "{}",
                    "output_format": "Json",
                    "start_time": "2019-09-13T19:35:38+00:00",
                    "end_time": "2019-09-13T19:37:14+00:00",
                    "environment": "Local",
                    "tool_version": "1.0"
                }"#).unwrap();

                let expected = ValidationError::git_commit_hash_not_a_string();
                let actual = ToolReport::try_from(&message)
                    .expect_err("expected Err(_) value");

                assert_eq!(expected, actual);
            }

            #[test]
            fn try_from_returns_error_when_tool_name_not_a_string() {
                let message = serde_json::from_str(r#"{
                    "application_name": "Test application",
                    "git_branch": "master",
                    "git_commit_hash": "e99f715d0fe787cd43de967b8a79b56960fed3e5",
                    "tool_name": false,
                    "tool_output": "{}",
                    "output_format": "Json",
                    "start_time": "2019-09-13T19:35:38+00:00",
                    "end_time": "2019-09-13T19:37:14+00:00",
                    "environment": "Local",
                    "tool_version": "1.0"
                }"#).unwrap();

                let expected = ValidationError::tool_name_not_a_string();
                let actual = ToolReport::try_from(&message)
                    .expect_err("expected Err(_) value");

                assert_eq!(expected, actual);
            }

            #[test]
            fn try_from_returns_error_when_tool_output_format_not_a_string() {
                let message = serde_json::from_str(r#"{
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
                }"#).unwrap();

                let expected = ValidationError::tool_output_format_not_a_string();
                let actual = ToolReport::try_from(&message)
                    .expect_err("expected Err(_) value");

                assert_eq!(expected, actual);
            }

            #[test]
            fn try_from_returns_error_when_environment_not_a_string() {
                let message = serde_json::from_str(r#"{
                    "application_name": "Test application",
                    "git_branch": "master",
                    "git_commit_hash": "e99f715d0fe787cd43de967b8a79b56960fed3e5",
                    "tool_name": "example tool",
                    "tool_output": "{}",
                    "output_format": "Json",
                    "start_time": "2019-09-13T19:35:38+00:00",
                    "end_time": "2019-09-13T19:37:14+00:00",
                    "environment": false,
                    "tool_version": "1.0"
                }"#).unwrap();

                let expected = ValidationError::environment_not_a_string();
                let actual = ToolReport::try_from(&message)
                    .expect_err("expected Err(_) value");

                assert_eq!(expected, actual);
            }

            #[test]
            fn try_from_returns_error_when_tool_version_present_but_not_a_string() {
                let message = serde_json::from_str(r#"{
                    "application_name": "Test application",
                    "git_branch": "master",
                    "git_commit_hash": "e99f715d0fe787cd43de967b8a79b56960fed3e5",
                    "tool_name": "example tool",
                    "tool_output": "{}",
                    "output_format": "Json",
                    "start_time": "2019-09-13T19:35:38+00:00",
                    "end_time": "2019-09-13T19:37:14+00:00",
                    "environment": "Local",
                    "tool_version": false
                }"#).unwrap();

                let expected = ValidationError::tool_version_not_a_string();
                let actual = ToolReport::try_from(&message)
                    .expect_err("expected Err(_) value");

                assert_eq!(expected, actual);
            }

            #[test]
            fn try_from_returns_error_when_tool_output_not_a_string() {
                let message = serde_json::from_str(r#"{
                    "application_name": "Test application",
                    "git_branch": "master",
                    "git_commit_hash": "e99f715d0fe787cd43de967b8a79b56960fed3e5",
                    "tool_name": "example tool",
                    "tool_output": false,
                    "output_format": "Json",
                    "start_time": "2019-09-13T19:35:38+00:00",
                    "end_time": "2019-09-13T19:37:14+00:00",
                    "environment": "Local",
                    "tool_version": "1.0"
                }"#).unwrap();

                let expected = ValidationError::tool_output_not_a_string();
                let actual = ToolReport::try_from(&message)
                    .expect_err("expected Err(_) value");

                assert_eq!(expected, actual);
            }
        }

        #[test]
        fn try_from_returns_error_when_application_name_empty() {
            let message = "".to_owned();
            let expected = ValidationError::application_name_empty();
            let actual = ApplicationName::try_from(message)
                .expect_err("expected Err(_) value");

            assert_eq!(expected, actual);
        }

        #[test]
        fn try_from_returns_error_when_git_branch_name_empty() {
            let message = "".to_owned();
            let expected = ValidationError::git_branch_name_empty();
            let actual = GitBranch::try_from(message)
                .expect_err("expected Err(_) value");

            assert_eq!(expected, actual);
        }

        #[test]
        fn try_from_returns_error_when_git_commit_hash_empty() {
            let message = "".to_owned();
            let expected = ValidationError::git_commit_hash_empty();
            let actual = GitCommitHash::try_from(message)
                .expect_err("expected Err(_) value");

            assert_eq!(expected, actual);
        }

        #[test]
        fn try_from_returns_error_when_git_commit_hash_not_valid_string() {
            let message = "zzz".to_owned();
            let expected = ValidationError::git_commit_hash_not_valid();
            let actual = GitCommitHash::try_from(message)
                .expect_err("expected Err(_) value");

            assert_eq!(expected, actual);
        }

        #[test]
        fn try_from_returns_error_when_tool_name_empty() {
            let message = "".to_owned();
            let expected = ValidationError::tool_name_empty();
            let actual = ToolName::try_from(message)
                .expect_err("expected Err(_) value");

            assert_eq!(expected, actual);
        }

        #[test]
        fn try_from_returns_error_when_tool_output_empty() {
            let message = "".to_owned();
            let expected = ValidationError::tool_output_empty();
            let actual = ToolOutput::try_from(message)
                .expect_err("expected Err(_) value");

            assert_eq!(expected, actual);
        }


        #[test]
        fn try_from_returns_error_when_tool_output_format_empty() {
            let message = serde_json::from_str(r#"{
                "application_name": "Test application",
                "git_branch": "master",
                "git_commit_hash": "e99f715d0fe787cd43de967b8a79b56960fed3e5",
                "tool_name": "example tool",
                "tool_output": "{}",
                "output_format": "",
                "start_time": "2019-09-13T19:35:38+00:00",
                "end_time": "2019-09-13T19:37:14+00:00",
                "environment": "Local",
                "tool_version": "1.0"
            }"#).unwrap();

            let expected = ValidationError::tool_output_format_empty();
            let actual = ToolReport::try_from(&message)
                .expect_err("expected Err(_) value");

            assert_eq!(expected, actual);
        }

        #[test]
        fn try_from_returns_error_when_tool_output_format_not_a_valid_option() {
            let message = serde_json::from_str(r#"{
                "application_name": "Test application",
                "git_branch": "master",
                "git_commit_hash": "e99f715d0fe787cd43de967b8a79b56960fed3e5",
                "tool_name": "example tool",
                "tool_output": "{}",
                "output_format": "msgpack",
                "start_time": "2019-09-13T19:35:38+00:00",
                "end_time": "2019-09-13T19:37:14+00:00",
                "environment": "Local",
                "tool_version": "1.0"
            }"#).unwrap();

            let expected = ValidationError::tool_output_format_invalid();
            let actual = ToolReport::try_from(&message)
                .expect_err("expected Err(_) value");

            assert_eq!(expected, actual);
        }

        #[test]
        fn try_from_returns_error_when_start_time_not_a_timestamp() {
            let message = serde_json::from_str(r#"{
                "application_name": "Test application",
                "git_branch": "master",
                "git_commit_hash": "e99f715d0fe787cd43de967b8a79b56960fed3e5",
                "tool_name": "example tool",
                "tool_output": "{}",
                "output_format": "Json",
                "start_time": "not a timestamp",
                "end_time": "2019-09-13T19:37:14+00:00",
                "environment": "Local",
                "tool_version": "1.0"
            }"#).unwrap();

            let expected = ValidationError::start_time_not_a_timestamp();
            let actual = ToolReport::try_from(&message)
                .expect_err("expected Err(_) value");

            assert_eq!(expected, actual);
        }

        #[test]
        fn try_from_returns_error_when_end_time_not_a_timestamp() {
            let message = serde_json::from_str(r#"{
                "application_name": "Test application",
                "git_branch": "master",
                "git_commit_hash": "e99f715d0fe787cd43de967b8a79b56960fed3e5",
                "tool_name": "example tool",
                "tool_output": "{}",
                "output_format": "Json",
                "start_time": "2019-09-13T19:35:38+00:00",
                "end_time": "not a timestamp",
                "environment": "Local",
                "tool_version": "1.0"
            }"#).unwrap();

            let expected = ValidationError::end_time_not_a_timestamp();
            let actual = ToolReport::try_from(&message)
                .expect_err("expected Err(_) value");

            assert_eq!(expected, actual);
        }
        
        #[test]
        fn try_from_returns_error_when_environment_not_a_valid_option() {
            let message = serde_json::from_str(r#"{
                "application_name": "Test application",
                "git_branch": "master",
                "git_commit_hash": "e99f715d0fe787cd43de967b8a79b56960fed3e5",
                "tool_name": "example tool",
                "tool_output": "{}",
                "output_format": "Json",
                "start_time": "2019-09-13T19:35:38+00:00",
                "end_time": "2019-09-13T19:37:14+00:00",
                "environment": "the moon",
                "tool_version": "1.0"
            }"#).unwrap();

            let expected = ValidationError::environment_not_a_valid_option();
            let actual = ToolReport::try_from(&message)
                .expect_err("expected Err(_) value");

            assert_eq!(expected, actual);
        }

        #[test]
        fn try_from_returns_error_when_tool_version_present_but_empty() {
            let message = serde_json::from_str(r#"{
                "application_name": "Test application",
                "git_branch": "master",
                "git_commit_hash": "e99f715d0fe787cd43de967b8a79b56960fed3e5",
                "tool_name": "example tool",
                "tool_output": "{}",
                "output_format": "Json",
                "start_time": "2019-09-13T19:35:38+00:00",
                "end_time": "2019-09-13T19:37:14+00:00",
                "environment": "Local",
                "tool_version": ""
            }"#).unwrap();

            let expected = ValidationError::tool_version_present_but_empty();
            let actual = ToolReport::try_from(&message)
                .expect_err("expected Err(_) value");

            assert_eq!(expected, actual);
        }

        #[test]
        fn tool_report_can_round_trip_to_avro_and_back() {
            let message = serde_json::from_str(r#"{
                "application_name": "Test application",
                "git_branch": "master",
                "git_commit_hash": "e99f715d0fe787cd43de967b8a79b56960fed3e5",
                "tool_name": "example tool",
                "tool_output": "{}",
                "output_format": "Json",
                "start_time": "2019-09-13T19:35:38+00:00",
                "end_time": "2019-09-13T19:37:14+00:00",
                "environment": "Local",
                "tool_version": "1.0"
            }"#).unwrap();

            let report = ToolReport::try_from(&message)
                .expect("expected Ok(_) value");

            let expected = ToolReport::try_from(&message)
                .expect("expected Ok(_) value");

            let avro = report.avro();

            let actual = ToolReport::try_from(avro)
                .expect("expected Ok(_) value");

            assert_eq!(expected, actual);
        }
    }
}

