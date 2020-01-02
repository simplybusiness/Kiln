#[macro_use] extern crate lazy_static;

use avro_rs::{Reader, Schema, Writer};
use failure::err_msg;
use kiln_lib::avro_schema::DEPENDENCY_EVENT_SCHEMA;
use kiln_lib::kafka::*;
use kiln_lib::dependency_event::{DependencyEvent, Timestamp, AdvisoryDescription, AdvisoryId, AdvisoryUrl, InstalledVersion, AffectedPackage};
use kiln_lib::tool_report::{ToolReport, EventVersion, EventID};
use regex::Regex;
use serde::Serialize;
use std::boxed::Box;
use std::collections::HashMap;
use std::convert::TryFrom;
use std::env;
use std::error::Error;
use uuid::Uuid;

fn main() -> Result<(), Box<dyn Error>> {
    let config = get_bootstrap_config(&mut env::vars())
        .map_err(|err| failure::err_msg(format!("Configuration Error: {}", err)))?;

    let ssl_connector = build_ssl_connector().map_err(|err| {
        failure::err_msg(format!(
            "OpenSSL Error {}: {}",
            err.errors()[0].code(),
            err.errors()[0].reason().unwrap()
        ))
    })?;

    let mut consumer = build_kafka_consumer(
        config.clone(),
        "ToolReports".to_string(),
        "report-parser".to_string(),
        ssl_connector.clone(),
    )
        .map_err(|err| err_msg(format!("Kafka Consumer Error: {}", err.description())))?;

    let mut producer = build_kafka_producer(
        config.clone(),
        ssl_connector.clone(),
    )
        .map_err(|err| err_msg(format!("Kafka Producer Error: {}", err.description())))?;

    loop {
        for ms in consumer.poll().unwrap().iter() {
            for m in ms.messages() {
                let reader = Reader::new(m.value)?;
                for value in reader {
                    let report = ToolReport::try_from(value?)?;
                    let records = parse_tool_report(&report)?;
                    for record in records.into_iter() {
                        producer
                            .send(&kafka::producer::Record::from_value(
                                "DependencyEvents",
                                record,
                            ))
                            .map_err(|err| err_msg(format!("Error publishing to Kafka: {}", err.to_string())))?;
                    }
                }
            }
            consumer.consume_messageset(ms)?;
        }
        consumer.commit_consumed()?;
    }
}

fn parse_tool_report(report: &ToolReport) -> Result<Vec<Vec<u8>>, Box<dyn Error>> {
    let events = if report.tool_name == "bundler-audit" {
        if report.output_format == "PlainText" {
            parse_bundler_audit_plaintext(&report)
        } else {
            Err(Box::new(err_msg(format!("Unknown output format for Bundler-audit in ToolReport: {:?}", report)).compat()).into())
        }
    } else {
        Err(Box::new(err_msg(format!("Unknown tool in ToolReport: {:?}", report)).compat()).into())
    }?;

    Ok(events
        .iter()
        .map(|event| {
            let schema = Schema::parse_str(DEPENDENCY_EVENT_SCHEMA).unwrap();
            let mut writer = Writer::new(&schema, Vec::new());
            writer.append_ser(event).unwrap();
            writer.flush().unwrap();
            writer.into_inner()
        })
        .collect::<Vec<Vec<u8>>>())
}

fn parse_bundler_audit_plaintext(report: &ToolReport) -> Result<Vec<DependencyEvent>, Box<dyn Error>> {
    lazy_static! {
        static ref BLOCK_RE: Regex = Regex::new("(Name: .*\nVersion: .*\nAdvisory: .*\nCriticality: .*\nURL: .*\nTitle: .*\nSolution:.*\n)").unwrap();
    }
    let mut events = Vec::new();
    for block in BLOCK_RE.captures_iter(report.tool_output.as_ref()) {
        let block = block.get(0).unwrap().as_str();
        let fields = block
            .trim_end()
            .split('\n')
            .map(|line| line.split(": ").collect::<Vec<_>>())
            .map(|fields| (fields[0].to_string(), fields[1].to_string()))
            .collect::<HashMap<_, _>>();
       let event = DependencyEvent {
            event_version: EventVersion::try_from("1".to_string())?,
            event_id: EventID::try_from(Uuid::new_v4().to_hyphenated().to_string())?,
            parent_event_id: report.event_id.clone(),
            application_name: report.application_name.clone(),
            git_branch: report.git_branch.clone(),
            git_commit_hash: report.git_commit_hash.clone(),
            timestamp: Timestamp::try_from(report.end_time.to_string())?,
            affected_package: AffectedPackage::try_from(fields.get("Name").or(Some(&"".to_string())).unwrap().to_owned())?,
            installed_version: InstalledVersion::try_from(fields.get("Version").or(Some(&"".to_string())).unwrap().to_owned())?,
            advisory_id: AdvisoryId::try_from(fields.get("Advisory").or(Some(&"".to_string())).unwrap().to_owned())?,
            advisory_url: AdvisoryUrl::try_from(fields.get("URL").or(Some(&"".to_string())).unwrap().to_owned())?,
            advisory_description: AdvisoryDescription::try_from(fields.get("Title").or(Some(&"".to_string())).unwrap().to_owned())?,
       };
       events.push(event);
    }
    Ok(events)
}
