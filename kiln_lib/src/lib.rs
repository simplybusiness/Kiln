pub mod validation {
    use lambda_http::{Body, IntoResponse, Response};
    use http::status::StatusCode;
    use serde::Serialize;

    #[derive(Debug, Serialize)]
    pub struct ValidationError<'a> {
        pub error_code: u8,
        pub error_message: &'a str,
    }

    pub mod validation_errors {
        use super::*;

        pub const BODY_EMPTY: ValidationError = ValidationError {
            error_code: 100,
            error_message: "Request body empty",
        };
        pub const BODY_MEDIA_TYPE_INCORRECT: ValidationError = ValidationError {
            error_code: 101,
            error_message: "Request body not correct media type",
        };
        pub const APPLICATION_NAME_EMPTY: ValidationError = ValidationError {
            error_code: 111,
            error_message: "Application name present but empty",
        };
        pub const APPLICATION_NAME_MISSING: ValidationError = ValidationError {
            error_code: 102,
            error_message: "Application name required",
        };
        pub const APPLICATION_NAME_NOT_A_STRING: ValidationError = ValidationError {
            error_code: 112,
            error_message: "Application name not a valid string",
        };
        pub const GIT_BRANCH_NAME_EMPTY: ValidationError = ValidationError {
            error_code: 113,
            error_message: "Git branch name present but empty",
        };
        pub const GIT_BRANCH_NAME_MISSING: ValidationError = ValidationError {
            error_code: 103,
            error_message: "Git branch name required",
        };
        pub const GIT_BRANCH_NAME_NOT_A_STRING: ValidationError = ValidationError {
            error_code: 114,
            error_message: "Git branch name not a valid string",
        };
        pub const GIT_COMMIT_HASH_EMPTY: ValidationError = ValidationError {
            error_code: 115,
            error_message: "Git commit hash present but empty",
        };
        pub const GIT_COMMIT_HASH_MISSING: ValidationError = ValidationError {
            error_code: 104,
            error_message: "Git commit hash required",
        };
        pub const GIT_COMMIT_HASH_NOT_A_STRING: ValidationError = ValidationError {
            error_code: 117,
            error_message: "Git commit hash not a valid string",
        };
        pub const GIT_COMMIT_HASH_NOT_VALID: ValidationError = ValidationError {
            error_code: 116,
            error_message: "Git commit hash not valid",
        };
        pub const TOOL_NAME_EMPTY: ValidationError = ValidationError {
            error_code: 118,
            error_message: "Tool name present but empty",
        };
        pub const TOOL_NAME_MISSING: ValidationError = ValidationError {
            error_code: 105,
            error_message: "Tool name required",
        };
        pub const TOOL_NAME_NOT_A_STRING: ValidationError = ValidationError {
            error_code: 119,
            error_message: "Tool name not a valid string",
        };
        pub const TOOL_OUTPUT_EMPTY: ValidationError = ValidationError {
            error_code: 120,
            error_message: "Tool output present but empty",
        };
        pub const TOOL_OUTPUT_MISSING: ValidationError = ValidationError {
            error_code: 106,
            error_message: "Tool output required",
        };
        pub const TOOL_OUTPUT_NOT_A_STRING: ValidationError = ValidationError {
            error_code: 121,
            error_message: "Tool output not a valid string",
        };
        pub const TOOL_OUTPUT_FORMAT_EMPTY: ValidationError = ValidationError {
            error_code: 122,
            error_message: "Tool output format present but empty",
        };
        pub const TOOL_OUTPUT_FORMAT_MISSING: ValidationError = ValidationError {
            error_code: 107,
            error_message: "Tool output format required",
        };
        pub const TOOL_OUTPUT_FORMAT_NOT_A_STRING: ValidationError = ValidationError {
            error_code: 123,
            error_message: "Tool output format not a valid string",
        };
        pub const TOOL_OUTPUT_FORMAT_INVALID: ValidationError = ValidationError {
            error_code: 124,
            error_message: "Tool output format not acceptable",
        };
        pub const START_TIME_MISSING: ValidationError = ValidationError {
            error_code: 108,
            error_message: "Start time required",
        };
        pub const START_TIME_NOT_A_TIMESTAMP: ValidationError = ValidationError {
            error_code: 125,
            error_message: "Start time not a valid timestamp",
        };
        pub const END_TIME_MISSING: ValidationError = ValidationError {
            error_code: 109,
            error_message: "End time required",
        };
        pub const END_TIME_NOT_A_TIMESTAMP: ValidationError = ValidationError {
            error_code: 126,
            error_message: "End time not a valid timestamp",
        };
        pub const ENVIRONMENT_NOT_A_VALID_OPTION: ValidationError = ValidationError {
            error_code: 128,
            error_message: "Environment not a valid option",
        };
        pub const ENVIRONMENT_MISSING: ValidationError = ValidationError {
            error_code: 110,
            error_message: "Environment required",
        };
        pub const ENVIRONMENT_NOT_A_STRING: ValidationError = ValidationError {
            error_code: 127,
            error_message: "Environment not a valid string",
        };
        pub const TOOL_VERSION_NOT_A_STRING: ValidationError = ValidationError {
            error_code: 130,
            error_message: "Tool version not a valid string",
        };
        pub const TOOL_VERSION_PRESENT_BUT_EMPTY: ValidationError = ValidationError {
            error_code: 129,
            error_message: "Tool version present but empty",
        };
    }

    impl IntoResponse for ValidationError<'_> {
        fn into_response(self) -> Response<Body> {
            Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Body::from(serde_json::to_string(&self).unwrap()))
                .unwrap()
        }
    }
}

pub mod tool_report {
    use crate::validation::{ValidationError, validation_errors};

    use std::convert::TryFrom;

    
    use chrono::{DateTime, Utc};
    use serde_json::value::Value;
    use regex::Regex;

    #[allow(dead_code)]
    pub struct ToolReport {
        application_name: String,
        git_branch: String,
        git_commit_hash: String,
        tool_name: String,
        tool_output: String,
        output_format: OutputFormat,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
        environment: Environment,
        tool_version: Option<String>,
    }

    pub enum OutputFormat {
        JSON,
        PlainText,
    }

    pub enum Environment {
        Local,
        CI,
    }

    impl TryFrom<&Value> for ToolReport {
        type Error = ValidationError<'static>;

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

    impl ToolReport {
        fn parse_application_name(json_value: &Value) -> Result<String, ValidationError<'static>> {
            let value = match &json_value["application_name"] {
                Value::Null => Err(validation_errors::APPLICATION_NAME_MISSING),
                Value::String(value) => Ok(value),
                _ => Err(validation_errors::APPLICATION_NAME_NOT_A_STRING),
            }?;
            if value.is_empty() {
                Err(validation_errors::APPLICATION_NAME_EMPTY)
            } else {
                Ok(value.into())
            }
        }

        fn parse_git_branch(json_value: &Value) -> Result<String, ValidationError<'static>> {
            let value = match &json_value["git_branch"] {
                Value::Null => Err(validation_errors::GIT_BRANCH_NAME_MISSING),
                Value::String(value) => Ok(value),
                _ => Err(validation_errors::GIT_BRANCH_NAME_NOT_A_STRING),
            }?;
            if value.is_empty() {
                Err(validation_errors::GIT_BRANCH_NAME_EMPTY)
            } else {
                Ok(value.into())
            }
        }

        fn parse_git_commit_hash(json_value: &Value) -> Result<String, ValidationError<'static>> {
            let value = match &json_value["git_commit_hash"] {
                Value::Null => Err(validation_errors::GIT_COMMIT_HASH_MISSING),
                Value::String(value) => Ok(value),
                _ => Err(validation_errors::GIT_COMMIT_HASH_NOT_A_STRING),
            }?;
            if value.is_empty() {
                return Err(validation_errors::GIT_COMMIT_HASH_EMPTY);
            };

            let re = Regex::new(r"^[0-9a-fA-F]{40}$").unwrap();
            if re.is_match(value) {
                Ok(value.into())
            } else {
                Err(validation_errors::GIT_COMMIT_HASH_NOT_VALID)
            }
        }

        fn parse_tool_name(json_value: &Value) -> Result<String, ValidationError<'static>> {
            let value = match &json_value["tool_name"] {
                Value::Null => Err(validation_errors::TOOL_NAME_MISSING),
                Value::String(value) => Ok(value),
                _ => Err(validation_errors::TOOL_NAME_NOT_A_STRING),
            }?;
            if value.is_empty() {
                Err(validation_errors::TOOL_NAME_EMPTY)
            } else {
                Ok(value.into())
            }
        }

        fn parse_tool_output(json_value: &Value) -> Result<String, ValidationError<'static>> {
            let value = match &json_value["tool_output"] {
                Value::Null => Err(validation_errors::TOOL_OUTPUT_MISSING),
                Value::String(value) => Ok(value),
                _ => Err(validation_errors::TOOL_OUTPUT_NOT_A_STRING),
            }?;
            if value.is_empty() {
                Err(validation_errors::TOOL_OUTPUT_EMPTY)
            } else {
                Ok(value.into())
            }
        }

        fn parse_output_format(json_value: &Value) -> Result<OutputFormat, ValidationError<'static>> {
            let value = match &json_value["output_format"] {
                Value::Null => Err(validation_errors::TOOL_OUTPUT_FORMAT_MISSING),
                Value::String(value) => Ok(value),
                _ => Err(validation_errors::TOOL_OUTPUT_FORMAT_NOT_A_STRING),
            }?;
            if value.is_empty() {
                return Err(validation_errors::TOOL_OUTPUT_FORMAT_EMPTY);
            };

            match value.as_ref() {
                "Json" => Ok(OutputFormat::JSON),
                "PlainText" => Ok(OutputFormat::PlainText),
                _ => Err(validation_errors::TOOL_OUTPUT_FORMAT_INVALID),
            }
        }

        fn parse_tool_start_time(
            json_value: &Value,
        ) -> Result<DateTime<Utc>, ValidationError<'static>> {
            let value = match &json_value["start_time"] {
                Value::Null => Err(validation_errors::START_TIME_MISSING),
                Value::String(value) => Ok(value),
                _ => Err(validation_errors::START_TIME_NOT_A_TIMESTAMP),
            }?;

            DateTime::parse_from_rfc3339(value)
                .map(DateTime::<Utc>::from)
                .map_err(|_| validation_errors::START_TIME_NOT_A_TIMESTAMP)
        }

        fn parse_tool_end_time(json_value: &Value) -> Result<DateTime<Utc>, ValidationError<'static>> {
            let value = match &json_value["end_time"] {
                Value::Null => Err(validation_errors::END_TIME_MISSING),
                Value::String(value) => Ok(value),
                _ => Err(validation_errors::END_TIME_NOT_A_TIMESTAMP),
            }?;

            DateTime::parse_from_rfc3339(value)
                .map(DateTime::<Utc>::from)
                .map_err(|_| validation_errors::END_TIME_NOT_A_TIMESTAMP)
        }

        fn parse_environment(json_value: &Value) -> Result<Environment, ValidationError<'static>> {
            let value = match &json_value["environment"] {
                Value::Null => Err(validation_errors::ENVIRONMENT_MISSING),
                Value::String(value) => Ok(value),
                _ => Err(validation_errors::ENVIRONMENT_NOT_A_STRING),
            }?;

            match value.as_ref() {
                "Local" => Ok(Environment::Local),
                "CI" => Ok(Environment::CI),
                _ => Err(validation_errors::ENVIRONMENT_NOT_A_VALID_OPTION),
            }
        }

        fn parse_tool_version(json_value: &Value) -> Result<Option<String>, ValidationError<'static>> {
            let value = match &json_value["tool_version"] {
                Value::Null => Ok(None),
                Value::String(value) => Ok(Some(value.to_owned())),
                _ => Err(validation_errors::TOOL_VERSION_NOT_A_STRING),
            }?;

            match value {
                None => Ok(None),
                Some(value) => {
                    if value.is_empty() {
                        Err(validation_errors::TOOL_VERSION_PRESENT_BUT_EMPTY)
                    } else {
                        Ok(Some(value))
                    }
                }
            }
        }
    }
}

