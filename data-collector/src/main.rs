use actix_web::{web, App, Error, HttpResponse, HttpServer};
use actix_web::middleware::Logger;
use addr::DomainName;
use avro_rs::{Schema, Writer};
use failure::err_msg;
use futures::{Future, Stream};
use kafka::client::{Compression, SecurityConfig};
use kafka::error::Error as KafkaError;
use kafka::producer::Producer;
use openssl::error::ErrorStack;
use openssl::ssl::{SslConnector, SslMethod, SslVerifyMode, SslVersion};
use std::convert::TryFrom;
use std::env;
use std::sync::Mutex;

use kiln_lib::avro_schema::TOOL_REPORT_SCHEMA;
use kiln_lib::tool_report::ToolReport;
use kiln_lib::validation::ValidationError;

fn main() -> Result<(), std::boxed::Box<dyn std::error::Error>> {
    std::env::set_var("RUST_LOG", "actix_web=info");
    env_logger::init();
    let config = get_configuration(&mut env::vars())
        .map_err(|err| failure::err_msg(format!("Configuration Error: {}", err)))?;

    let ssl_connector = build_ssl_connector().map_err(|err| {
        failure::err_msg(format!(
            "OpenSSL Error {}: {}",
            err.errors()[0].code(),
            err.errors()[0].reason().unwrap()
        ))
    })?;

    let producer = build_kafka_producer(config, ssl_connector)
        .map_err(|err| err_msg(format!("Kafka Error: {}", err.description())))?;

    let shared_producer = web::Data::new(Mutex::new(producer));

    HttpServer::new(move || {
        App::new()
            .register_data(shared_producer.clone())
            .service(web::resource("/").route(web::post().to_async(handler)))
            .wrap(Logger::default())
            .wrap(Logger::new("%a %{User-Agent}i"))
    })
    .bind("0.0.0.0:8080")?
    .run()
    .map_err(|err| err.into())
}

fn handler(
    payload: web::Payload,
    producer: web::Data<Mutex<Producer>>,
) -> impl Future<Item = HttpResponse, Error = Error> {
    payload.from_err().concat2().and_then(move |body| {
        let report_result = parse_payload(&body);

        if let Err(err) = report_result {
            return Ok(err.into());
        }

        let report = report_result.unwrap();

        let serialised_record = serialise_to_avro(report)?;

        producer
            .lock()
            .unwrap()
            .send(&kafka::producer::Record::from_value(
                "ToolReports",
                serialised_record,
            ))
            .map_err(|err| err_msg(format!("Error publishing to Kafka: {}", err.to_string())))?;

        Ok(HttpResponse::Ok().finish())
    })
}

pub fn parse_payload(body: &web::Bytes) -> Result<ToolReport, ValidationError> {
    if body.is_empty() {
        return Err(ValidationError::body_empty());
    }

    serde_json::from_slice(&body)
        .map_err(|_| ValidationError::body_media_type_incorrect())
        .and_then(|json| ToolReport::try_from(&json))
}

pub fn get_configuration<I>(vars: &mut I) -> Result<Config, String>
where
    I: Iterator<Item = (String, String)>,
{
    let local_vars: Vec<(String, String)> = vars.collect();
    let disable_kafka_domain_validation = match local_vars.iter().find(|var| var.0 == "DISABLE_KAFKA_DOMAIN_VALIDATION") {
        None => Ok(false),
        Some(var) => {
            if var.1.is_empty() {
                return Err(
                    "Optional environment variable present but empty: DISABLE_KAFKA_DOMAIN_VALIDATION"
                        .to_owned(),
                );
            } else {
                match var.1.as_ref() {
                    "true" => Ok(true),
                    "false" => Ok(false),
                    _ => Err("Optional environment variable did not pass validation: DISABLE_KAFKA_DOMAIN_VALIDATION".to_owned())
                }
            }
        }
    }?;

    let kafka_bootstrap_tls = match local_vars.iter().find(|var| var.0 == "KAFKA_BOOTSTRAP_TLS") {
        None => {
            Err("Required environment variable missing: KAFKA_BOOTSTRAP_TLS".to_owned())
        }
        Some(var) => {
            if var.1.is_empty() {
                return Err(
                    "Required environment variable present but empty: KAFKA_BOOTSTRAP_TLS"
                        .to_owned(),
                );
            } else {
                let raw_hosts: Vec<String> = var.1.split(',').map(|s| s.to_owned()).collect();
                let valid = raw_hosts.iter().all(|x| {
                    let parts: Vec<&str> = x.split(':').collect();
                    let domain_valid = if disable_kafka_domain_validation {
                        true
                    } else {
                        parts[0].parse::<DomainName>().is_ok()
                    };
                    let port_valid = u16::from_str_radix(parts[1], 10).is_ok();
                    domain_valid && port_valid
                });
                if valid {
                    Ok(raw_hosts)
                } else {
                    Err(
                        "KAFKA_BOOTSTRAP_TLS environment variable did not pass validation"
                            .to_owned(),
                    )
                }
            }
        }
    }?;

    Ok(Config {
        kafka_bootstrap_tls,
    })
}

pub fn build_ssl_connector() -> Result<SslConnector, ErrorStack> {
    let mut ssl_connector_builder = SslConnector::builder(SslMethod::tls())?;
    ssl_connector_builder.set_verify(SslVerifyMode::PEER);
    ssl_connector_builder.set_default_verify_paths()?;
    ssl_connector_builder.set_min_proto_version(Some(SslVersion::TLS1_2))?;
    ssl_connector_builder.set_cipher_list("ECDHE-ECDSA-AES128-GCM-SHA256:ECDHE-RSA-AES128-GCM-SHA256:ECDHE-ECDSA-AES256-GCM-SHA384:ECDHE-RSA-AES256-GCM-SHA384:ECDHE-ECDSA-CHACHA20-POLY1305:ECDHE-RSA-CHACHA20-POLY1305:DHE-RSA-AES128-GCM-SHA256:DHE-RSA-AES256-GCM-SHA384")?; //This cipher suite list is taken from the Mozilla Server Side TLS Version 5 recommendations, with the exception of support for TLS 1.3 as this is not supported by Apache Kafka yet
    Ok(ssl_connector_builder.build())
}

pub fn build_kafka_producer(
    config: Config,
    ssl_connector: SslConnector,
) -> Result<Producer, KafkaError> {
    let security_config = SecurityConfig::new(ssl_connector).with_hostname_verification(true);

    Producer::from_hosts(config.kafka_bootstrap_tls)
        .with_compression(Compression::GZIP)
        .with_security(security_config)
        .create()
}

pub fn serialise_to_avro(report: ToolReport) -> Result<Vec<u8>, failure::Error> {
    let schema = Schema::parse_str(TOOL_REPORT_SCHEMA)?;
    let mut writer = Writer::new(&schema, Vec::new());
    writer.append_ser(report)?;
    writer.flush()?;
    Ok(writer.into_inner())
}

#[derive(Debug)]
pub struct Config {
    kafka_bootstrap_tls: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    use actix_web::web::Bytes;

    use chrono::{DateTime, Utc};

    use serial_test_derive::serial;

    use kiln_lib::tool_report::{
        ApplicationName, EndTime, Environment, GitBranch, GitCommitHash, OutputFormat, StartTime,
        ToolName, ToolOutput, ToolVersion,
    };

    fn set_env_vars() {
        std::env::remove_var("KAFKA_BOOTSTRAP_TLS");
        std::env::set_var("KAFKA_BOOTSTRAP_TLS", "my.kafka.host.example.com:1234");
    }

    #[test]
    fn parse_payload_returns_error_when_body_empty() {
        let p = "".to_owned();
        let payload = p.as_bytes();
        let mut body = Bytes::new();
        body.extend_from_slice(payload);
        let expected = ValidationError::body_empty();
        let actual = parse_payload(&body).expect_err("expected Err(_) value");

        assert_eq!(expected, actual);
    }

    #[test]
    fn parse_payload_returns_error_when_body_contains_bytes() {
        let p = "\u{0000}".to_string();
        let payload = p.as_bytes();
        let mut body = Bytes::new();
        body.extend_from_slice(payload);
        let expected = ValidationError::body_media_type_incorrect();

        let actual = parse_payload(&body).expect_err("expected Ok(_) value");

        assert_eq!(expected, actual);
    }

    #[test]
    fn parse_payload_returns_error_when_body_is_not_json() {
        let p = "<report><title>Not a valid report</title></report>".to_owned();
        let payload = p.as_bytes();
        let mut body = Bytes::new();
        body.extend_from_slice(payload);
        let expected = ValidationError::body_media_type_incorrect();
        let response = parse_payload(&body).expect_err("expected Err(_) value");

        assert_eq!(expected, response);
    }

    #[test]
    fn parse_payload_returns_tool_report_when_request_valid() {
        let p = r#"{
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
                }"#
        .to_owned();
        let payload = p.as_bytes();
        let mut body = Bytes::new();
        body.extend_from_slice(payload);

        let expected = ToolReport {
            application_name: ApplicationName::try_from("Test application".to_owned()).unwrap(),
            git_branch: GitBranch::try_from("master".to_owned()).unwrap(),
            git_commit_hash: GitCommitHash::try_from(
                "e99f715d0fe787cd43de967b8a79b56960fed3e5".to_owned(),
            )
            .unwrap(),
            tool_name: ToolName::try_from("example tool".to_owned()).unwrap(),
            tool_output: ToolOutput::try_from("{}".to_owned()).unwrap(),
            output_format: OutputFormat::JSON,
            start_time: StartTime::from(DateTime::<Utc>::from(
                DateTime::parse_from_rfc3339("2019-09-13T19:35:38+00:00").unwrap(),
            )),
            end_time: EndTime::from(DateTime::<Utc>::from(
                DateTime::parse_from_rfc3339("2019-09-13T19:37:14+00:00").unwrap(),
            )),
            environment: Environment::Local,
            tool_version: ToolVersion::try_from(Some("1.0".to_owned())).unwrap(),
        };

        let actual = parse_payload(&body).expect("expected Ok(_) value");

        assert_eq!(expected, actual);
    }

    #[test]
    #[serial]
    fn main_returns_error_when_environment_vars_missing() {
        set_env_vars();
        std::env::remove_var("KAFKA_BOOTSTRAP_TLS");

        let actual = main();

        match actual {
            Ok(_) => panic!("expected Err(_) value"),
            Err(err) => assert_eq!("Configuration Error: Required environment variable missing: KAFKA_BOOTSTRAP_TLS", err.to_string()),
        }
    }

    #[test]
    fn get_configuration_returns_config_when_environment_vars_present_and_valid() {
        let hostname =
            "my.kafka.host.example.com:1234,my.second.kafka.host.example.com:1234".to_owned();
        let mut fake_vars = vec![("KAFKA_BOOTSTRAP_TLS".to_owned(), hostname)].into_iter();

        let expected = vec![
            "my.kafka.host.example.com:1234".to_owned(),
            "my.second.kafka.host.example.com:1234".to_owned(),
        ];

        let actual = get_configuration(&mut fake_vars).expect("expected Ok(_) value");

        assert_eq!(actual.kafka_bootstrap_tls, expected);
    }

    #[test]
    fn get_configuration_returns_error_when_environment_vars_missing() {
        let mut fake_vars = std::iter::empty::<(String, String)>();

        let actual = get_configuration(&mut fake_vars).expect_err("expected Err(_) value");

        assert_eq!(
            actual.to_string(),
            "Required environment variable missing: KAFKA_BOOTSTRAP_TLS"
        )
    }

    #[test]
    fn get_configuration_returns_error_when_environment_vars_present_but_empty() {
        let hostname = "".to_owned();
        let mut fake_vars = vec![("KAFKA_BOOTSTRAP_TLS".to_owned(), hostname.clone())].into_iter();

        let actual = get_configuration(&mut fake_vars).expect_err("expected Err(_) value");

        assert_eq!(
            actual.to_string(),
            "Required environment variable present but empty: KAFKA_BOOTSTRAP_TLS"
        )
    }

    #[test]
    fn get_configuration_returns_error_when_hostname_invalid_and_domain_validation_enabled() {
        let hostname = "kafka:1234".to_owned();
        let mut fake_vars = vec![("KAFKA_BOOTSTRAP_TLS".to_owned(), hostname.clone())].into_iter();

        let actual = get_configuration(&mut fake_vars).expect_err("expected Err(_) value");

        assert_eq!(
            actual.to_string(),
            "KAFKA_BOOTSTRAP_TLS environment variable did not pass validation"
        )
    }

    #[test]
    fn get_configuration_returns_configration_when_hostname_not_a_valid_domain_and_domain_validation_disabled() {
        let hostname = "kafka:1234".to_owned();
        let mut fake_vars = vec![("KAFKA_BOOTSTRAP_TLS".to_owned(), hostname.clone()), ("DISABLE_KAFKA_DOMAIN_VALIDATION".to_owned(), "true".to_owned())].into_iter();
        let expected = vec![hostname.clone()];

        let actual = get_configuration(&mut fake_vars).expect("expected Ok(_) value");

        assert_eq!(
            actual.kafka_bootstrap_tls,
            expected
        )
    }

    #[test]
    fn get_configuration_returns_error_when_port_number_invalid() {
        let hostname = "my.kafka.host.example.com:1234567".to_owned();
        let mut fake_vars = vec![("KAFKA_BOOTSTRAP_TLS".to_owned(), hostname.clone())].into_iter();

        let actual = get_configuration(&mut fake_vars).expect_err("expected Err(_) value");

        assert_eq!(
            actual.to_string(),
            "KAFKA_BOOTSTRAP_TLS environment variable did not pass validation"
        )
    }

    #[test]
    fn get_configuration_returns_error_when_disable_kafka_domain_validation_present_but_empty() {
        let mut fake_vars = vec![("DISABLE_KAFKA_DOMAIN_VALIDATION".to_owned(), "".to_owned())].into_iter();
        let actual = get_configuration(&mut fake_vars).expect_err("expected Err(_) value");

        assert_eq!(
            actual.to_string(),
            "Optional environment variable present but empty: DISABLE_KAFKA_DOMAIN_VALIDATION"
        )
    }

    #[test]
    fn get_configuration_returns_error_when_disable_kafka_domain_validation_present_but_invalid() {
        let mut fake_vars = vec![("DISABLE_KAFKA_DOMAIN_VALIDATION".to_owned(), "blah".to_owned())].into_iter();
        let actual = get_configuration(&mut fake_vars).expect_err("expected Err(_) value");

        assert_eq!(
            actual.to_string(),
            "Optional environment variable did not pass validation: DISABLE_KAFKA_DOMAIN_VALIDATION"
        )
    }
}
