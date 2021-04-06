use actix_web::{web, App, HttpResponse, HttpServer};
use avro_rs::{Schema, Writer};
use rdkafka::{
    error::KafkaError,
    producer::future_producer::{FutureProducer, FutureRecord},
};
use std::convert::TryFrom;
use std::env;
use std::str;
use std::sync::Arc;
pub mod lib;

use chrono::{SecondsFormat, Utc};
use slog::o;
use slog::Drain;
use slog::{FnValue, PushFnValue};

use crate::lib::StructuredLogger;

use kiln_lib::avro_schema::TOOL_REPORT_SCHEMA;
use kiln_lib::kafka::*;
use kiln_lib::log::NestedJsonFmt;
use kiln_lib::tool_report::ToolReport;
use kiln_lib::validation::ValidationError;


const SERVICE_NAME: &str = "data-collector";

#[actix_rt::main]
async fn main() -> Result<(), anyhow::Error> {
    let drain = NestedJsonFmt::new(std::io::stdout()).fuse();

    let drain = slog_async::Async::new(drain).build().fuse();

    let root_logger = slog::Logger::root(
        drain,
        o!(
            "@timestamp" => PushFnValue(move |_, ser| {
                ser.emit(Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true))
            }),
            "log.level" => FnValue(move |rinfo| {
                rinfo.level().as_str()
            }),
            "message" => PushFnValue(move |record, ser| {
                ser.emit(record.msg())
            }),
            "ecs.version" => "1.5",
            "service.version" => env!("CARGO_PKG_VERSION"),
            "service.name" => SERVICE_NAME,
            "service.type" => "kiln",
            "event.kind" => "event",
            "event.category" => "web",
        ),
    );

    let config = get_bootstrap_config(&mut env::vars())?;

    let producer = build_kafka_producer(config)?;

    let shared_producer = Arc::from(producer);

    HttpServer::new(move || {
        App::new()
            .data(shared_producer.clone())
            .wrap(StructuredLogger::new(root_logger.clone()).exclude("/health"))
            .route("/", web::post().to(handler))
            .route("/health", web::get().to(health_handler))
    })
    .bind("0.0.0.0:8080")?
    .run()
    .await
    .map_err(|err| err.into())
}
async fn health_handler() -> HttpResponse {
    HttpResponse::Ok().finish()
}

async fn handler(
    body: web::Bytes,
    producer: web::Data<Arc<FutureProducer>>,
) -> Result<HttpResponse, HandlerError> {
    let report = parse_payload(&body)?;

    let app_name = report.application_name.to_string();

    let serialised_record = serialise_to_avro(report)?;

    let kafka_payload = FutureRecord::to("ToolReports")
        .payload(&serialised_record)
        .key(&app_name);

    producer
        .send_result(kafka_payload)
        .map_err(|err| err.0.into())
        .and_then(|_| Ok(HttpResponse::Ok().finish()))
}

pub fn parse_payload(body: &web::Bytes) -> Result<ToolReport, ValidationError> {
    if body.is_empty() {
        return Err(ValidationError::body_empty());
    }

    serde_json::from_slice(&body)
        .map_err(|_| ValidationError::body_media_type_incorrect())
        .and_then(|json| ToolReport::try_from(&json))
}

pub fn serialise_to_avro(report: ToolReport) -> Result<Vec<u8>, HandlerError> {
    let schema = Schema::parse_str(TOOL_REPORT_SCHEMA)?;
    let mut writer = Writer::new(&schema, Vec::new());
    writer.append_ser(report)?;
    Ok(writer.into_inner()?)
}

#[derive(thiserror::Error, Debug)]
pub enum HandlerError {
    #[error("Something went wrong while communicating with Kafka")]
    KafkaError {
        #[from]
        source: KafkaError,
    },
    #[error(transparent)]
    ValidationError(#[from] ValidationError),
    #[error("Something went wrong while serialising payload to Apache Avro")]
    AvroError(#[from] avro_rs::Error),
}

impl From<HandlerError> for actix_web::error::Error {
    fn from(err: HandlerError) -> Self {
        match err {
            HandlerError::ValidationError(e) => actix_web::error::ErrorBadRequest(e),
            HandlerError::KafkaError { source } => {
                actix_web::error::ErrorInternalServerError(source)
            }
            HandlerError::AvroError(_) => actix_web::error::ErrorInternalServerError(err),
        }
    }
}

#[derive(Debug)]
pub struct Config {
    kafka_bootstrap_tls: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::iter::FromIterator;

    use actix_web::web::Bytes;

    use chrono::{DateTime, Utc};

    use serial_test_derive::serial;

    use kiln_lib::tool_report::{
        ApplicationName, EndTime, Environment, EventID, EventVersion, ExpiryDate, GitBranch,
        GitCommitHash, IssueHash, OutputFormat, StartTime, SuppressedBy, SuppressedIssue,
        SuppressionReason, ToolName, ToolOutput, ToolVersion,
    };

    fn set_env_vars() {
        std::env::remove_var("KAFKA_BOOTSTRAP_TLS");
        std::env::set_var("KAFKA_BOOTSTRAP_TLS", "my.kafka.host.example.com:1234");
    }

    #[test]
    fn parse_payload_returns_error_when_body_empty() {
        let p = "".to_owned();
        let payload = p.as_bytes().into_iter().cloned().collect::<Vec<u8>>();
        let body: Bytes = Bytes::from_iter(payload);
        let expected = ValidationError::body_empty();
        let actual = parse_payload(&body).expect_err("expected Err(_) value");

        assert_eq!(expected, actual);
    }

    #[test]
    fn parse_payload_returns_error_when_body_contains_bytes() {
        let p = "\u{0000}".to_string();
        let payload = p.as_bytes().into_iter().cloned().collect::<Vec<u8>>();
        let body: Bytes = Bytes::from_iter(payload);
        let expected = ValidationError::body_media_type_incorrect();

        let actual = parse_payload(&body).expect_err("expected Ok(_) value");

        assert_eq!(expected, actual);
    }

    #[test]
    fn parse_payload_returns_error_when_body_is_not_json() {
        let p = "<report><title>Not a valid report</title></report>".to_owned();
        let payload = p.as_bytes().into_iter().cloned().collect::<Vec<u8>>();
        let body: Bytes = Bytes::from_iter(payload);
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
                    "git_branch": "git",
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
                        "expiry_date": "2020-05-12T00:00:00+00:00",
                        "suppressed_by": "Dan Murphy"
                    }]
                }"#
        .to_owned();
        let payload = p.as_bytes().into_iter().cloned().collect::<Vec<u8>>();
        let body: Bytes = Bytes::from_iter(payload);

        let expected = ToolReport {
            event_version: EventVersion::try_from("1".to_owned()).unwrap(),
            event_id: EventID::try_from("95130bee-95ae-4dac-aecf-5650ff646ea1".to_owned()).unwrap(),
            application_name: ApplicationName::try_from("Test application".to_owned()).unwrap(),
            git_branch: GitBranch::try_from(Some("git".to_owned())).unwrap(),
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
            suppressed_issues: vec![SuppressedIssue {
                issue_hash: IssueHash::try_from(
                    "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_owned(),
                )
                .unwrap(),
                suppression_reason: SuppressionReason::try_from("Test issue".to_owned()).unwrap(),
                expiry_date: ExpiryDate::from(Some(DateTime::<Utc>::from(
                    DateTime::parse_from_rfc3339("2020-05-12T00:00:00+00:00").unwrap(),
                ))),
                suppressed_by: SuppressedBy::try_from("Dan Murphy".to_owned()).unwrap(),
            }],
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
            Err(err) => assert_eq!(
                "Required environment variable KAFKA_BOOTSTRAP_TLS failed validation because value is missing",
                err.to_string()
            ),
        }
    }
}
