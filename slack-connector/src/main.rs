use avro_rs::Reader;
use failure::err_msg;
use futures_util::stream::StreamExt;
use kiln_lib::dependency_event::DependencyEvent;
use kiln_lib::kafka::*;
use kiln_lib::traits::Hashable;
use rdkafka::consumer::{CommitMode, Consumer};
use rdkafka::message::Message;
use reqwest::blocking::Client;
use reqwest::Method;
use serde_json::{json, Value};
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
        config.clone(),
        "slack-connector".to_string(),
        &tls_cert_path,
    )
    .map_err(|err| err_msg(format!("Kafka Consumer Error: {}", err.to_string())))?;

    consumer.subscribe(&["DependencyEvents"])?;
    let mut messages = consumer.start_with(std::time::Duration::from_secs(1), false);

    let client = Client::new();

    loop {
        if let Some(Ok(message)) = messages.next().await {
            if let Some(body) = message.payload() {
                let reader = Reader::new(body)?;
                for value in reader {
                    let event = DependencyEvent::try_from(value?)?;
                    if !event.suppressed {
                        let payload = json!({
                            "channel": channel_id,
                            "text": event.to_slack_message()
                        });
                        let req = client
                            .request(Method::POST, "https://slack.com/api/chat.postMessage")
                            .bearer_auth(&oauth_token)
                            .json(&payload)
                            .build()?;
                        let resp = client.execute(req)?;
                        let resp_body: Value = resp.json()?;
                        if !resp_body.get("ok").unwrap().as_bool().unwrap() {
                            let cause = resp_body.get("error").unwrap().as_str().unwrap();
                            eprintln!(
                                "Error sending message for event {}, parent {}, {}",
                                event.event_id.to_string(),
                                event.parent_event_id.to_string(),
                                cause
                            );
                        }
                    }
                }
            }
            consumer.commit_message(&message, CommitMode::Async)?;
        }
    }
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
