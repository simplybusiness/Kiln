use actix_web::middleware::Logger;
use actix_web::{web, App, HttpResponse, HttpServer};
use actix_web::Error as ActixError;
use avro_rs::{Schema, Writer};
use failure::err_msg;
use rdkafka::producer::future_producer::{FutureProducer, FutureRecord};
use std::boxed::Box;
use std::convert::TryFrom;
use std::env;
use std::error::Error;
use std::str;

use log::warn;

use kiln_lib::avro_schema::TOOL_REPORT_SCHEMA;
use kiln_lib::kafka::*;
use kiln_lib::tool_report::ToolReport;
use kiln_lib::validation::ValidationError;

#[actix_rt::main]
async fn main() -> Result<(), Box<dyn Error>> {
    std::env::set_var("RUST_LOG", "info");
    env_logger::init();
    let config = get_bootstrap_config(&mut env::vars())
        .map_err(|err| failure::err_msg(format!("Configuration Error: {}", err)))?;

    let producer = build_kafka_producer(config)
        .map_err(|err| err_msg(format!("Kafka Error: {}", err.description())))?;

    HttpServer::new(move || {
        App::new()
            .data(producer.clone())
            .route("/", web::post().to(handler))
            .wrap(Logger::default())
            .wrap(Logger::new("%a %{User-Agent}i"))
    })
    .bind("0.0.0.0:8080")?
    .run()
    .await
    .map_err(|err| err.into())
}

async fn handler(
    body: web::Bytes,
    producer: web::Data<FutureProducer>
) -> Result<HttpResponse, ActixError> {
    let report_result = parse_payload(&body);

    if let Err(err) = report_result {
        if let Some(field_name) = &err.json_field_name {
            warn!("Request did not pass validation. Error message: {}. JSON field name: {}. Request body: {}\n", err.error_message, field_name, str::from_utf8(&body).unwrap());
        } else {
            warn!("Request did not pass validation. Error message: {}. Request body: {}\n", err.error_message, str::from_utf8(&body).unwrap());
        }
        return Ok(err.into());
    }

    let report = report_result.unwrap();
    let app_name = report.application_name.to_string();

    let serialised_record = serialise_to_avro(report)?;

    let kafka_payload = FutureRecord::to("ToolReports")
        .payload(&serialised_record)
        .key(&app_name);

    let delivery_result = producer
        .send(kafka_payload, 5000)
        .await?;

    match delivery_result {
        Ok(_) => Ok(HttpResponse::Ok().finish()),
        Err(err) => Err(err_msg(format!("Error publishing to Kafka: {}", err.0)).into())

    }

}

pub fn parse_payload(body: &web::Bytes) -> Result<ToolReport, ValidationError> {
    if body.is_empty() {
        return Err(ValidationError::body_empty());
    }

    serde_json::from_slice(&body)
        .map_err(|_| ValidationError::body_media_type_incorrect())
        .and_then(|json| ToolReport::try_from(&json))
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
        ApplicationName, EndTime, Environment, EventID, EventVersion, GitBranch, GitCommitHash, OutputFormat, StartTime,
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
                    "event_version": "1",
                    "event_id": "95130bee-95ae-4dac-aecf-5650ff646ea1",
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
            event_version: EventVersion::try_from("1".to_owned()).unwrap(),
            event_id: EventID::try_from("95130bee-95ae-4dac-aecf-5650ff646ea1".to_owned()).unwrap(),
            application_name: ApplicationName::try_from("Test application".to_owned()).unwrap(),
            git_branch: GitBranch::try_from(Some("master".to_owned())).unwrap(),
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
}
