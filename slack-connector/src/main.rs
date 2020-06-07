use avro_rs::Reader;
use bytes::buf::ext::BufExt;
use bytes::Bytes;
use failure::err_msg;
use futures::stream::{StreamExt, TryStreamExt};
use futures_util::sink::SinkExt;
use kiln_lib::dependency_event::DependencyEvent;
use kiln_lib::kafka::*;
use kiln_lib::traits::Hashable;
use rdkafka::consumer::Consumer;
use rdkafka::message::Message;
use reqwest::Client;
use std::boxed::Box;
use std::convert::TryFrom;
use std::env;
use std::error::Error;
use std::path::PathBuf;
use std::str::FromStr;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let oauth_token =
        env::var("OAUTH2_TOKEN").expect("Error: Required Env Var OAUTH2_TOKEN not provided");
    let channel_id = env::var("SLACK_CHANNEL_ID")
        .expect("Error: Required Env Var SLACK_CHANNEL_ID not provided");
    let config = get_bootstrap_config(&mut env::vars())
        .map_err(|err| failure::err_msg(format!("Configuration Error: {}", err)))?;

    let tls_cert_path = PathBuf::from_str("/etc/ssl/certs/ca-certificates.crt").unwrap();

    let consumer = build_kafka_consumer(
        config,
        "slack-connector".to_string(),
        &tls_cert_path,
    )
    .map_err(|err| err_msg(format!("Kafka Consumer Error: {}", err.to_string())))?;

    let client = Client::new();
    let (queue_tx, queue_rx) = futures::channel::mpsc::channel(10);

    consumer.subscribe(&["DependencyEvents"])?;
    let avro_bytes = consumer.start_with(std::time::Duration::from_secs(1), false);
    let mut events = avro_bytes
        .map_ok(|message| message.detach())
        .map_err(|err| failure::Error::from_boxed_compat(Box::new(err)))
        .and_then(move |message| async move {
            message
                .payload()
                .ok_or_else(|| err_msg("Received empty payload from Kafka"))
                .map(|msg| Bytes::copy_from_slice(msg))
        })
        .and_then(|body_bytes| async { Reader::new(body_bytes.reader()) })
        .map_ok(|unparsed_events| {
            futures::stream::iter(unparsed_events.map(|event| DependencyEvent::try_from(event?)))
        })
        .boxed();

    queue_tx
        .sink_map_err(|_| err_msg("Could not send event to stream"))
        .send_all(&mut events);

    Ok(())
}

trait ToSlackMessage {
    fn to_slack_message(&self) -> String;
}

impl ToSlackMessage for DependencyEvent {
    fn to_slack_message(&self) -> String {
        format!("Vulnerable package found in: {}\nWhat package is affected? {} {}\nWhere was this found? Commit {} on branch {}\nWhat is the problem? {}\nHow serious is it? CVSS {}\nWhere can I find out more? {}\nIssue hash if this should be suppressed: {}",
            self.application_name.to_string(),
            self.affected_package.to_string(),
            self.installed_version.to_string(),
            self.git_commit_hash.to_string(),
            self.git_branch.to_string(),
            self.advisory_description.to_string(),
            self.cvss.to_string(),
            self.advisory_url.to_string(),
            hex::encode(self.hash()),
        )
    }
}
