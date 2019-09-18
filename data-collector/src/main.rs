use chrono::{DateTime, Utc};
use http::status::StatusCode;
use lambda_http::{lambda, Body, IntoResponse, Request, Response};
use lambda_runtime::{error::HandlerError, Context};
use regex::Regex;
use serde::Serialize;
use serde_json::value::Value;

fn main() {
    lambda!(handler)
}

fn handler(req: Request, _: Context) -> Result<impl IntoResponse, HandlerError> {
    let body = req.body();
    if let Body::Empty = body {
        return Ok(validation_errors::BODY_EMPTY.into_response());
    };

    if let Body::Binary(_) = body {
        return Ok(validation_errors::BODY_MEDIA_TYPE_INCORRECT.into_response());
    };

    if let Body::Text(body_text) = body {
        let b = body_text.clone();
        let json: Value = serde_json::from_str(&b).unwrap();
        return match ToolReport::parse(&json) {
            Ok(_) => Ok(Response::builder()
                .status(StatusCode::OK)
                .body(Body::Empty)
                .unwrap()),
            Err(validation_error) => Ok(validation_error.into_response()),
        };
    };

    Ok(Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .body(Body::from("Unknown error"))
        .unwrap())
}

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

#[allow(dead_code)]
struct ToolReport {
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

impl ToolReport {
    pub fn parse(json_value: &Value) -> Result<Self, ValidationError> {
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

    fn parse_application_name(json_value: &Value) -> Result<String, ValidationError> {
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

    fn parse_git_branch(json_value: &Value) -> Result<String, ValidationError> {
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

    fn parse_git_commit_hash(json_value: &Value) -> Result<String, ValidationError> {
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

    fn parse_tool_name(json_value: &Value) -> Result<String, ValidationError> {
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

    fn parse_tool_output(json_value: &Value) -> Result<String, ValidationError> {
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

    fn parse_output_format(json_value: &Value) -> Result<OutputFormat, ValidationError> {
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

    fn parse_tool_start_time(json_value: &Value) -> Result<DateTime<Utc>, ValidationError> {
        let value = match &json_value["start_time"] {
            Value::Null => Err(validation_errors::START_TIME_MISSING),
            Value::String(value) => Ok(value),
            _ => Err(validation_errors::START_TIME_NOT_A_TIMESTAMP),
        }?;

        DateTime::parse_from_rfc3339(value)
            .map(DateTime::<Utc>::from)
            .map_err(|_| validation_errors::START_TIME_NOT_A_TIMESTAMP)
    }

    fn parse_tool_end_time(json_value: &Value) -> Result<DateTime<Utc>, ValidationError> {
        let value = match &json_value["end_time"] {
            Value::Null => Err(validation_errors::END_TIME_MISSING),
            Value::String(value) => Ok(value),
            _ => Err(validation_errors::END_TIME_NOT_A_TIMESTAMP),
        }?;

        DateTime::parse_from_rfc3339(value)
            .map(DateTime::<Utc>::from)
            .map_err(|_| validation_errors::END_TIME_NOT_A_TIMESTAMP)
    }

    fn parse_environment(json_value: &Value) -> Result<Environment, ValidationError> {
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

    fn parse_tool_version(json_value: &Value) -> Result<Option<String>, ValidationError> {
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

enum OutputFormat {
    JSON,
    PlainText,
}

enum Environment {
    Local,
    CI,
}

#[cfg(test)]
mod tests {
    use super::*;

    use http::status::StatusCode;
    use lambda_http::http::Request;
    use lambda_http::Body;
    use serde_json::json;

    #[test]
    fn handler_returns_error_when_body_empty() {
        let request = Request::default();
        let expected = json!({
            "error_code": 100,
            "error_message": "Request body empty"
        })
        .into_response();
        let response = handler(request, Context::default())
            .expect("expected Ok(_) value")
            .into_response();
        assert_eq!(response.body(), expected.body());
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn handler_returns_error_when_body_contains_bytes() {
        let mut builder = Request::builder();
        let request = builder.body(Body::from(r#"{}"#.as_bytes())).unwrap();
        let expected = json!({
            "error_code": 101,
            "error_message": "Request body not correct media type"
        })
        .into_response();
        let response = handler(request, Context::default())
            .expect("expected Ok(_) value")
            .into_response();
        assert_eq!(response.body(), expected.body());
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn handler_returns_error_when_application_name_missing() {
        let mut builder = Request::builder();
        let request = builder
            .body(Body::from(
                r#"{
            "git_branch": "master",
            "git_commit_hash": "e99f715d0fe787cd43de967b8a79b56960fed3e5",
            "tool_name": "example tool",
            "tool_output": "{}",
            "output_format": "Json",
            "start_time": "2019-09-13T19:35:38+00:00",
            "end_time": "2019-09-13T19:37:14+00:00",
            "environment": "Local",
            "tool_version": "1.0"
        }"#,
            ))
            .unwrap();
        let expected = json!({
            "error_code": 102,
            "error_message": "Application name required"
        })
        .into_response();
        let response = handler(request, Context::default())
            .expect("expected Ok(_) value")
            .into_response();
        assert_eq!(response.body(), expected.body());
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn handler_returns_error_when_git_branch_missing() {
        let mut builder = Request::builder();
        let request = builder
            .body(Body::from(
                r#"{
            "application_name": "Test application",
            "git_commit_hash": "e99f715d0fe787cd43de967b8a79b56960fed3e5",
            "tool_name": "example tool",
            "tool_output": "{}",
            "output_format": "Json",
            "start_time": "2019-09-13T19:35:38+00:00",
            "end_time": "2019-09-13T19:37:14+00:00",
            "environment": "Local",
            "tool_version": "1.0"
        }"#,
            ))
            .unwrap();
        let expected = json!({
            "error_code": 103,
            "error_message": "Git branch name required"
        })
        .into_response();
        let response = handler(request, Context::default())
            .expect("expected Ok(_) value")
            .into_response();
        assert_eq!(response.body(), expected.body());
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn handler_returns_error_when_git_commit_hash_missing() {
        let mut builder = Request::builder();
        let request = builder
            .body(Body::from(
                r#"{
            "application_name": "Test application",
            "git_branch": "master",
            "tool_name": "example tool",
            "tool_output": "{}",
            "output_format": "Json",
            "start_time": "2019-09-13T19:35:38+00:00",
            "end_time": "2019-09-13T19:37:14+00:00",
            "environment": "Local",
            "tool_version": "1.0"
        }"#,
            ))
            .unwrap();
        let expected = json!({
            "error_code": 104,
            "error_message": "Git commit hash required"
        })
        .into_response();
        let response = handler(request, Context::default())
            .expect("expected Ok(_) value")
            .into_response();
        assert_eq!(response.body(), expected.body());
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn handler_returns_error_when_tool_name_missing() {
        let mut builder = Request::builder();
        let request = builder
            .body(Body::from(
                r#"{
            "application_name": "Test application",
            "git_branch": "master",
            "git_commit_hash": "e99f715d0fe787cd43de967b8a79b56960fed3e5",
            "tool_output": "{}",
            "output_format": "Json",
            "start_time": "2019-09-13T19:35:38+00:00",
            "end_time": "2019-09-13T19:37:14+00:00",
            "environment": "Local",
            "tool_version": "1.0"
        }"#,
            ))
            .unwrap();
        let expected = json!({
            "error_code": 105,
            "error_message": "Tool name required"
        })
        .into_response();
        let response = handler(request, Context::default())
            .expect("expected Ok(_) value")
            .into_response();
        assert_eq!(response.body(), expected.body());
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn handler_returns_error_when_tool_output_missing() {
        let mut builder = Request::builder();
        let request = builder
            .body(Body::from(
                r#"{
            "application_name": "Test application",
            "git_branch": "master",
            "git_commit_hash": "e99f715d0fe787cd43de967b8a79b56960fed3e5",
            "tool_name": "example tool",
            "output_format": "Json",
            "start_time": "2019-09-13T19:35:38+00:00",
            "end_time": "2019-09-13T19:37:14+00:00",
            "environment": "Local",
            "tool_version": "1.0"
        }"#,
            ))
            .unwrap();
        let expected = json!({
            "error_code": 106,
            "error_message": "Tool output required"
        })
        .into_response();
        let response = handler(request, Context::default())
            .expect("expected Ok(_) value")
            .into_response();
        assert_eq!(response.body(), expected.body());
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn handler_returns_error_when_tool_output_format_missing() {
        let mut builder = Request::builder();
        let request = builder
            .body(Body::from(
                r#"{
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
            ))
            .unwrap();
        let expected = json!({
            "error_code": 107,
            "error_message": "Tool output format required"
        })
        .into_response();
        let response = handler(request, Context::default())
            .expect("expected Ok(_) value")
            .into_response();
        assert_eq!(response.body(), expected.body());
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn handler_returns_error_when_start_time_missing() {
        let mut builder = Request::builder();
        let request = builder
            .body(Body::from(
                r#"{
            "application_name": "Test application",
            "git_branch": "master",
            "git_commit_hash": "e99f715d0fe787cd43de967b8a79b56960fed3e5",
            "tool_name": "example tool",
            "tool_output": "{}",
            "output_format": "Json",
            "end_time": "2019-09-13T19:37:14+00:00",
            "environment": "Local",
            "tool_version": "1.0"
        }"#,
            ))
            .unwrap();
        let expected = json!({
            "error_code": 108,
            "error_message": "Start time required"
        })
        .into_response();
        let response = handler(request, Context::default())
            .expect("expected Ok(_) value")
            .into_response();
        assert_eq!(response.body(), expected.body());
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn handler_returns_error_when_end_time_missing() {
        let mut builder = Request::builder();
        let request = builder
            .body(Body::from(
                r#"{
            "application_name": "Test application",
            "git_branch": "master",
            "git_commit_hash": "e99f715d0fe787cd43de967b8a79b56960fed3e5",
            "tool_name": "example tool",
            "tool_output": "{}",
            "output_format": "Json",
            "start_time": "2019-09-13T19:35:38+00:00",
            "environment": "Local",
            "tool_version": "1.0"
        }"#,
            ))
            .unwrap();
        let expected = json!({
            "error_code": 109,
            "error_message": "End time required"
        })
        .into_response();
        let response = handler(request, Context::default())
            .expect("expected Ok(_) value")
            .into_response();
        assert_eq!(response.body(), expected.body());
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn handler_returns_error_when_environment_missing() {
        let mut builder = Request::builder();
        let request = builder
            .body(Body::from(
                r#"{
            "application_name": "Test application",
            "git_branch": "master",
            "git_commit_hash": "e99f715d0fe787cd43de967b8a79b56960fed3e5",
            "tool_name": "example tool",
            "tool_output": "{}",
            "output_format": "Json",
            "start_time": "2019-09-13T19:35:38+00:00",
            "end_time": "2019-09-13T19:37:14+00:00",
            "tool_version": "1.0"
        }"#,
            ))
            .unwrap();
        let expected = json!({
            "error_code": 110,
            "error_message": "Environment required"
        })
        .into_response();
        let response = handler(request, Context::default())
            .expect("expected Ok(_) value")
            .into_response();
        assert_eq!(response.body(), expected.body());
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn handler_returns_error_when_application_name_empty() {
        let mut builder = Request::builder();
        let request = builder
            .body(Body::from(
                r#"{
            "application_name": "",
            "git_branch": "master",
            "git_commit_hash": "e99f715d0fe787cd43de967b8a79b56960fed3e5",
            "tool_name": "example tool",
            "tool_output": "{}",
            "output_format": "Json",
            "start_time": "2019-09-13T19:35:38+00:00",
            "end_time": "2019-09-13T19:37:14+00:00",
            "environment": "Local",
            "tool_version": "1.0"
        }"#,
            ))
            .unwrap();
        let expected = json!({
            "error_code": 111,
            "error_message": "Application name present but empty"
        })
        .into_response();
        let response = handler(request, Context::default())
            .expect("expected Ok(_) value")
            .into_response();
        assert_eq!(response.body(), expected.body());
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn handler_returns_error_when_application_name_not_a_string() {
        let mut builder = Request::builder();
        let request = builder
            .body(Body::from(
                r#"{
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
        }"#,
            ))
            .unwrap();
        let expected = json!({
            "error_code": 112,
            "error_message": "Application name not a valid string"
        })
        .into_response();
        let response = handler(request, Context::default())
            .expect("expected Ok(_) value")
            .into_response();
        assert_eq!(response.body(), expected.body());
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn handler_returns_error_when_git_branch_name_empty() {
        let mut builder = Request::builder();
        let request = builder
            .body(Body::from(
                r#"{
            "application_name": "Test application",
            "git_branch": "",
            "git_commit_hash": "e99f715d0fe787cd43de967b8a79b56960fed3e5",
            "tool_name": "example tool",
            "tool_output": "{}",
            "output_format": "Json",
            "start_time": "2019-09-13T19:35:38+00:00",
            "end_time": "2019-09-13T19:37:14+00:00",
            "environment": "Local",
            "tool_version": "1.0"
        }"#,
            ))
            .unwrap();
        let expected = json!({
            "error_code": 113,
            "error_message": "Git branch name present but empty"
        })
        .into_response();
        let response = handler(request, Context::default())
            .expect("expected Ok(_) value")
            .into_response();
        assert_eq!(response.body(), expected.body());
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn handler_returns_error_when_git_branch_name_not_a_string() {
        let mut builder = Request::builder();
        let request = builder
            .body(Body::from(
                r#"{
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
        }"#,
            ))
            .unwrap();
        let expected = json!({
            "error_code": 114,
            "error_message": "Git branch name not a valid string"
        })
        .into_response();
        let response = handler(request, Context::default())
            .expect("expected Ok(_) value")
            .into_response();
        assert_eq!(response.body(), expected.body());
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn handler_returns_error_when_git_commit_hash_empty() {
        let mut builder = Request::builder();
        let request = builder
            .body(Body::from(
                r#"{
            "application_name": "Test application",
            "git_branch": "master",
            "git_commit_hash": "",
            "tool_name": "example tool",
            "tool_output": "{}",
            "output_format": "Json",
            "start_time": "2019-09-13T19:35:38+00:00",
            "end_time": "2019-09-13T19:37:14+00:00",
            "environment": "Local",
            "tool_version": "1.0"
        }"#,
            ))
            .unwrap();
        let expected = json!({
            "error_code": 115,
            "error_message": "Git commit hash present but empty"
        })
        .into_response();
        let response = handler(request, Context::default())
            .expect("expected Ok(_) value")
            .into_response();
        assert_eq!(response.body(), expected.body());
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn handler_returns_error_when_git_commit_hash_not_valid_string() {
        let mut builder = Request::builder();
        let request = builder
            .body(Body::from(
                r#"{
            "application_name": "Test application",
            "git_branch": "master",
            "git_commit_hash": "zzz",
            "tool_name": "example tool",
            "tool_output": "{}",
            "output_format": "Json",
            "start_time": "2019-09-13T19:35:38+00:00",
            "end_time": "2019-09-13T19:37:14+00:00",
            "environment": "Local",
            "tool_version": "1.0"
        }"#,
            ))
            .unwrap();
        let expected = json!({
            "error_code": 116,
            "error_message": "Git commit hash not valid"
        })
        .into_response();
        let response = handler(request, Context::default())
            .expect("expected Ok(_) value")
            .into_response();
        assert_eq!(response.body(), expected.body());
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn handler_returns_error_when_git_commit_hash_not_a_string() {
        let mut builder = Request::builder();
        let request = builder
            .body(Body::from(
                r#"{
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
        }"#,
            ))
            .unwrap();
        let expected = json!({
            "error_code": 117,
            "error_message": "Git commit hash not a valid string"
        })
        .into_response();
        let response = handler(request, Context::default())
            .expect("expected Ok(_) value")
            .into_response();
        assert_eq!(response.body(), expected.body());
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn handler_returns_error_when_tool_name_empty() {
        let mut builder = Request::builder();
        let request = builder
            .body(Body::from(
                r#"{
            "application_name": "Test application",
            "git_branch": "master",
            "git_commit_hash": "e99f715d0fe787cd43de967b8a79b56960fed3e5",
            "tool_name": "",
            "tool_output": "{}",
            "output_format": "Json",
            "start_time": "2019-09-13T19:35:38+00:00",
            "end_time": "2019-09-13T19:37:14+00:00",
            "environment": "Local",
            "tool_version": "1.0"
        }"#,
            ))
            .unwrap();
        let expected = json!({
            "error_code": 118,
            "error_message": "Tool name present but empty"
        })
        .into_response();
        let response = handler(request, Context::default())
            .expect("expected Ok(_) value")
            .into_response();
        assert_eq!(response.body(), expected.body());
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn handler_returns_error_when_tool_name_not_a_string() {
        let mut builder = Request::builder();
        let request = builder
            .body(Body::from(
                r#"{
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
        }"#,
            ))
            .unwrap();
        let expected = json!({
            "error_code": 119,
            "error_message": "Tool name not a valid string"
        })
        .into_response();
        let response = handler(request, Context::default())
            .expect("expected Ok(_) value")
            .into_response();
        assert_eq!(response.body(), expected.body());
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn handler_returns_error_when_tool_output_empty() {
        let mut builder = Request::builder();
        let request = builder
            .body(Body::from(
                r#"{
            "application_name": "Test application",
            "git_branch": "master",
            "git_commit_hash": "e99f715d0fe787cd43de967b8a79b56960fed3e5",
            "tool_name": "example tool",
            "tool_output": "",
            "output_format": "Json",
            "start_time": "2019-09-13T19:35:38+00:00",
            "end_time": "2019-09-13T19:37:14+00:00",
            "environment": "Local",
            "tool_version": "1.0"
        }"#,
            ))
            .unwrap();
        let expected = json!({
            "error_code": 120,
            "error_message": "Tool output present but empty"
        })
        .into_response();
        let response = handler(request, Context::default())
            .expect("expected Ok(_) value")
            .into_response();
        assert_eq!(response.body(), expected.body());
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn handler_returns_error_when_tool_output_not_a_string() {
        let mut builder = Request::builder();
        let request = builder
            .body(Body::from(
                r#"{
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
        }"#,
            ))
            .unwrap();
        let expected = json!({
            "error_code": 121,
            "error_message": "Tool output not a valid string"
        })
        .into_response();
        let response = handler(request, Context::default())
            .expect("expected Ok(_) value")
            .into_response();
        assert_eq!(response.body(), expected.body());
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn handler_returns_error_when_tool_output_format_empty() {
        let mut builder = Request::builder();
        let request = builder
            .body(Body::from(
                r#"{
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
        }"#,
            ))
            .unwrap();
        let expected = json!({
            "error_code": 122,
            "error_message": "Tool output format present but empty"
        })
        .into_response();
        let response = handler(request, Context::default())
            .expect("expected Ok(_) value")
            .into_response();
        assert_eq!(response.body(), expected.body());
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn handler_returns_error_when_tool_output_format_not_a_string() {
        let mut builder = Request::builder();
        let request = builder
            .body(Body::from(
                r#"{
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
            ))
            .unwrap();
        let expected = json!({
            "error_code": 123,
            "error_message": "Tool output format not a valid string"
        })
        .into_response();
        let response = handler(request, Context::default())
            .expect("expected Ok(_) value")
            .into_response();
        assert_eq!(response.body(), expected.body());
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn handler_returns_error_when_tool_output_format_not_a_valid_option() {
        let mut builder = Request::builder();
        let request = builder
            .body(Body::from(
                r#"{
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
        }"#,
            ))
            .unwrap();
        let expected = json!({
            "error_code": 124,
            "error_message": "Tool output format not acceptable"
        })
        .into_response();
        let response = handler(request, Context::default())
            .expect("expected Ok(_) value")
            .into_response();
        assert_eq!(response.body(), expected.body());
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn handler_returns_error_when_start_time_not_a_timestamp() {
        let mut builder = Request::builder();
        let request = builder
            .body(Body::from(
                r#"{
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
        }"#,
            ))
            .unwrap();
        let expected = json!({
            "error_code": 125,
            "error_message": "Start time not a valid timestamp"
        })
        .into_response();
        let response = handler(request, Context::default())
            .expect("expected Ok(_) value")
            .into_response();
        assert_eq!(response.body(), expected.body());
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn handler_returns_error_when_end_time_not_a_timestamp() {
        let mut builder = Request::builder();
        let request = builder
            .body(Body::from(
                r#"{
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
        }"#,
            ))
            .unwrap();
        let expected = json!({
            "error_code": 126,
            "error_message": "End time not a valid timestamp"
        })
        .into_response();
        let response = handler(request, Context::default())
            .expect("expected Ok(_) value")
            .into_response();
        assert_eq!(response.body(), expected.body());
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn handler_returns_error_when_environment_not_a_string() {
        let mut builder = Request::builder();
        let request = builder
            .body(Body::from(
                r#"{
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
        }"#,
            ))
            .unwrap();
        let expected = json!({
            "error_code": 127,
            "error_message": "Environment not a valid string"
        })
        .into_response();
        let response = handler(request, Context::default())
            .expect("expected Ok(_) value")
            .into_response();
        assert_eq!(response.body(), expected.body());
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn handler_returns_error_when_environment_not_a_valid_option() {
        let mut builder = Request::builder();
        let request = builder
            .body(Body::from(
                r#"{
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
        }"#,
            ))
            .unwrap();
        let expected = json!({
            "error_code": 128,
            "error_message": "Environment not a valid option"
        })
        .into_response();
        let response = handler(request, Context::default())
            .expect("expected Ok(_) value")
            .into_response();
        assert_eq!(response.body(), expected.body());
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn handler_returns_error_when_tool_version_present_but_empty() {
        let mut builder = Request::builder();
        let request = builder
            .body(Body::from(
                r#"{
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
        }"#,
            ))
            .unwrap();
        let expected = json!({
            "error_code": 129,
            "error_message": "Tool version present but empty"
        })
        .into_response();
        let response = handler(request, Context::default())
            .expect("expected Ok(_) value")
            .into_response();
        assert_eq!(response.body(), expected.body());
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn handler_returns_error_when_tool_version_present_but_not_a_string() {
        let mut builder = Request::builder();
        let request = builder
            .body(Body::from(
                r#"{
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
        }"#,
            ))
            .unwrap();
        let expected = json!({
            "error_code": 130,
            "error_message": "Tool version not a valid string"
        })
        .into_response();
        let response = handler(request, Context::default())
            .expect("expected Ok(_) value")
            .into_response();
        assert_eq!(response.body(), expected.body());
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn handler_returns_http_200_when_tool_report_valid() {
        let mut builder = Request::builder();
        let request = builder
            .body(Body::from(
                r#"{
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
        }"#,
            ))
            .unwrap();
        let expected = Request::default();
        let response = handler(request, Context::default())
            .expect("expected Ok(_) value")
            .into_response();
        assert_eq!(response.body(), expected.body())
    }
}
