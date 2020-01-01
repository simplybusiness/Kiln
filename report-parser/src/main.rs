use avro_rs::Reader;
use failure::err_msg;
use kiln_lib::kafka::*;
use kiln_lib::dependency_event::DependencyEvent;
use kiln_lib::tool_report::ToolReport;
use regex::Regex;
use std::convert::TryFrom;
use std::env;

fn main() -> Result<(), std::boxed::Box<dyn std::error::Error>> {
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
        config,
        "ToolReports".to_string(),
        "report-parser".to_string(),
        ssl_connector,
    )
        .map_err(|err| err_msg(format!("Kafka Consumer Error: {}", err.description())))?;


    loop {
        for ms in consumer.poll().unwrap().iter() {
            for m in ms.messages() {
                let reader = Reader::new(m.value)?;
                for value in reader {
                    let report = ToolReport::try_from(value?)?;
                    println!("{:?}", report);
                }
            }
            consumer.consume_messageset(ms)?;
        }
        consumer.commit_consumed()?;
    }
}
