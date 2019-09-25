use addr::DomainName;
use http::status::StatusCode;
use lambda_http::{lambda, Body, IntoResponse, Request, Response};
use lambda_runtime::{error::HandlerError, Context};
use kafka::producer::Producer;
use kafka::client::{Compression, SecurityConfig};
use kafka::error::Error as KafkaError;
use openssl::error::ErrorStack;
use openssl::ssl::{SslConnector, SslVerifyMode, SslMethod, SslVersion};
use std::convert::TryFrom;
use std::env;

use kiln_lib::validation::ValidationError;
use kiln_lib::tool_report::ToolReport;

fn main() {
    lambda!(handler)
}

fn handler(req: Request, _: Context) -> Result<impl IntoResponse, HandlerError> {
    let config = get_configuration(&mut env::vars())
        .map_err(|err| {
            failure::err_msg(format!("Configuration Error: {}", err))
        })?;

    let ssl_connector = build_ssl_connector()
        .map_err(|err| {
            failure::err_msg(format!("OpenSSL Error {}: {}", err.errors()[0].code(), err.errors()[0].reason().unwrap()))
        })?;

    let producer = build_kafka_producer(config, ssl_connector)
        .map_err(|err| {
            failure::err_msg(format!("Kafka Error: {}", err.description()))
        })?;

    let report = parse_request(&req);
    match report {
        Ok(_) => Ok(Response::builder()
            .status(StatusCode::OK)
            .body(Body::Empty)
            .unwrap()),
        Err(validation_error) => Ok(validation_error.into_response()),
    }
}

pub fn parse_request(req: &Request) -> Result<ToolReport, ValidationError> {
    let body = req.body();
    match body {
        Body::Empty => Err(ValidationError::body_empty()),
        Body::Binary(_) => Err(ValidationError::body_media_type_incorrect()),
        Body::Text(body_text) => Ok(body_text),
    }
    .and_then(|body_text| {
        serde_json::from_str(&body_text).map_err(|_| ValidationError::body_media_type_incorrect())
    })
    .and_then(|json| ToolReport::try_from(&json))
}

pub fn get_configuration<I>(vars: &mut I) -> Result<Config, String> where I: Iterator<Item=(String, String)> {
    let kafka_bootstrap_tls = match vars.find(|var| var.0 == "KAFKA_BOOTSTRAP_TLS") {
        None => Err("Required environment variable missing or empty: KAFKA_BOOTSTRAP_TLS".to_owned() ),
        Some(var) => {
            if var.1.is_empty() {
                return Err("Required environment variable missing or empty: KAFKA_BOOTSTRAP_TLS".to_owned())
            } else {
                let raw_hosts: Vec<String> = var.1.split(",").map(|s| s.to_owned()).collect();
                let valid = raw_hosts.iter().all(|x| {
                    let parts: Vec<&str> = x.split(":").collect();
                    parts[0].parse::<DomainName>().is_ok() && u16::from_str_radix(parts[1], 10).is_ok()
                });
                if valid { Ok(raw_hosts) } else { Err("KAFKA_BOOTSTRAP_TLS environment variable did not pass validation".to_owned()) }
            }
        }
    }?;

    Ok(Config { kafka_bootstrap_tls })
}

pub fn build_ssl_connector() -> Result<SslConnector, ErrorStack> {
    let mut ssl_connector_builder = SslConnector::builder(SslMethod::tls())?;
    ssl_connector_builder.set_verify(SslVerifyMode::PEER);
    ssl_connector_builder.set_default_verify_paths()?;
    ssl_connector_builder.set_min_proto_version(Some(SslVersion::TLS1_2))?;
    ssl_connector_builder.set_cipher_list("ECDHE-ECDSA-AES128-GCM-SHA256:ECDHE-RSA-AES128-GCM-SHA256:ECDHE-ECDSA-AES256-GCM-SHA384:ECDHE-RSA-AES256-GCM-SHA384:ECDHE-ECDSA-CHACHA20-POLY1305:ECDHE-RSA-CHACHA20-POLY1305:DHE-RSA-AES128-GCM-SHA256:DHE-RSA-AES256-GCM-SHA384")?; //This cipher suite list is taken from the Mozilla Server Side TLS Version 5 recommendations, with the exception of support for TLS 1.3 as this is not supported by Apache Kafka yet
    Ok(ssl_connector_builder.build())
}

pub fn build_kafka_producer(config: Config, ssl_connector: SslConnector) -> Result<Producer, KafkaError> {

    let security_config = SecurityConfig::new(ssl_connector)
        .with_hostname_verification(true);

    Producer::from_hosts(config.kafka_bootstrap_tls)
        .with_compression(Compression::GZIP)
        .with_security(security_config)
        .create()
}

#[derive(Debug)]
pub struct Config {
    kafka_bootstrap_tls: Vec<String>
}

#[cfg(test)]
mod tests {
    use super::*;

    use chrono::{DateTime, Utc};
    use http::status::StatusCode;
    use lambda_http::http::Request;
    use lambda_http::Body;
    use serde_json::json;

    use serial_test_derive::serial;
    use lambda_runtime_errors::LambdaErrorExt;

    use kiln_lib::tool_report::{ApplicationName, GitCommitHash, GitBranch, ToolName, ToolOutput, ToolVersion, Environment, OutputFormat};

    fn set_env_vars() {
        std::env::remove_var("KAFKA_BOOTSTRAP_TLS");
        std::env::set_var("KAFKA_BOOTSTRAP_TLS", "my.kafka.host.example.com:1234");
    }

    #[test]
    fn parse_request_returns_error_when_body_empty() {
        let request = Request::default();
        let expected = ValidationError::body_empty();
        let actual = parse_request(&request)
            .expect_err("expected Err(_) value");
        
        assert_eq!(expected, actual);
    }

    #[test]
    fn parse_request_returns_error_when_body_contains_bytes() {
        let mut builder = Request::builder();
        let request = builder.body(Body::from(r#"{}"#.as_bytes())).unwrap();
        let expected = ValidationError::body_media_type_incorrect();

        let actual = parse_request(&request)
            .expect_err("expected Ok(_) value");

        assert_eq!(expected, actual);
    }

    #[test]
    fn parse_request_returns_error_when_body_is_not_json() {
        let mut builder = Request::builder();
        let request = builder
            .body(Body::from(
                "<report><title>Not a valid report</title></report>",
            ))
            .unwrap();
        let expected = ValidationError::body_media_type_incorrect();
        let response = parse_request(&request)
            .expect_err("expected Err(_) value");

        assert_eq!(expected, response);
    }

    #[test]
    fn parse_request_returns_tool_report_when_request_valid() {
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

        let expected = ToolReport {
            application_name: ApplicationName::try_from("Test application".to_owned()).unwrap(),
            git_branch: GitBranch::try_from("master".to_owned()).unwrap(),
            git_commit_hash: GitCommitHash::try_from("e99f715d0fe787cd43de967b8a79b56960fed3e5".to_owned()).unwrap(),
            tool_name: ToolName::try_from("example tool".to_owned()).unwrap(),
            tool_output: ToolOutput::try_from("{}".to_owned()).unwrap(),
            output_format: OutputFormat::JSON,
            start_time: DateTime::<Utc>::from(DateTime::parse_from_rfc3339("2019-09-13T19:35:38+00:00").unwrap()),
            end_time: DateTime::<Utc>::from(DateTime::parse_from_rfc3339("2019-09-13T19:37:14+00:00").unwrap()),
            environment: Environment::Local,
            tool_version: ToolVersion::try_from(Some("1.0".to_owned())).unwrap()
        };

        let actual = parse_request(&request)
            .expect("expected Ok(_) value");
        
        assert_eq!(expected, actual);
    }

    #[test]
    #[serial]
    fn handler_returns_error_when_environment_vars_missing() {
        set_env_vars();
        std::env::remove_var("KAFKA_BOOTSTRAP_TLS");

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

        let actual = handler(request, Context::default());

        match actual {
            Ok(_) => panic!("expected Err(_) value"),
            Err(err) => assert_eq!("failure::ErrorMessage", err.error_type())
        }
    }

    #[test]
    fn get_configuration_returns_config_when_environment_vars_present_and_valid() {
        let hostname = "my.kafka.host.example.com:1234,my.second.kafka.host.example.com:1234".to_owned();
        let mut fake_vars = vec![("KAFKA_BOOTSTRAP_TLS".to_owned(), hostname)]
            .into_iter();

        let expected = vec!["my.kafka.host.example.com:1234".to_owned(), "my.second.kafka.host.example.com:1234".to_owned()];

        let actual = get_configuration(&mut fake_vars)
            .expect("expected Ok(_) value");

        assert_eq!(actual.kafka_bootstrap_tls, expected);
    }

    #[test]
    fn get_configuration_returns_error_when_environment_vars_missing() {
        let mut fake_vars = std::iter::empty::<(String, String)>();

        let actual = get_configuration(&mut fake_vars)
            .expect_err("expected Err(_) value");

        assert_eq!(actual.to_string(), "Required environment variable missing or empty: KAFKA_BOOTSTRAP_TLS")
    }

    #[test]
    fn get_configuration_returns_error_when_environment_vars_present_but_empty() {
        let hostname = "".to_owned();
        let mut fake_vars = vec![("KAFKA_BOOTSTRAP_TLS".to_owned(), hostname.clone())]
            .into_iter();

        let actual = get_configuration(&mut fake_vars)
            .expect_err("expected Err(_) value");

        assert_eq!(actual.to_string(), "Required environment variable missing or empty: KAFKA_BOOTSTRAP_TLS")
    }

    #[test]
    fn get_configuration_returns_error_when_hostname_invalid() {
        let hostname = "!!!:1234".to_owned();
        let mut fake_vars = vec![("KAFKA_BOOTSTRAP_TLS".to_owned(), hostname.clone())]
            .into_iter();

        let actual = get_configuration(&mut fake_vars)
            .expect_err("expected Err(_) value");

        assert_eq!(actual.to_string(), "KAFKA_BOOTSTRAP_TLS environment variable did not pass validation")
    }

    #[test]
    fn get_configuration_returns_error_when_post_number_invalid() {
        let hostname = "my.kafka.host.example.com:1234567".to_owned();
        let mut fake_vars = vec![("KAFKA_BOOTSTRAP_TLS".to_owned(), hostname.clone())]
            .into_iter();

        let actual = get_configuration(&mut fake_vars)
            .expect_err("expected Err(_) value");

        assert_eq!(actual.to_string(), "KAFKA_BOOTSTRAP_TLS environment variable did not pass validation")

    }
}
