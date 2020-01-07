use avro_rs::Reader;
use failure::err_msg;
use kiln_lib::kafka::*;
use kiln_lib::dependency_event::DependencyEvent;
use reqwest::blocking::Client;
use reqwest::Method;
use serde_json::{json, Value};
use std::boxed::Box;
use std::env;
use std::error::Error;
use std::convert::TryFrom;

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    openssl_probe::init_ssl_cert_env_vars();

    let oauth_token = env::var("OAUTH2_TOKEN").expect("Error: Required Env Var OAUTH2_TOKEN not provided");
    let channel_id = env::var("SLACK_CHANNEL_ID").expect("Error: Required Env Var SLACK_CHANNEL_ID not provided");
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
        "DependencyEvents".to_string(),
        "slack-connector".to_string(),
        ssl_connector.clone(),
    )
        .map_err(|err| err_msg(format!("Kafka Consumer Error: {}", err.description())))?;

    let client = Client::new();

    loop {
        for ms in consumer.poll().unwrap().iter() {
            for m in ms.messages() {
                let reader = Reader::new(m.value)?;
                for value in reader {
                    let event = DependencyEvent::try_from(value?)?;
                    let payload = json!({
                        "channel": channel_id,
                        "text": event.to_slack_message()
                    });
                    let req = client.request(Method::POST, "https://slack.com/api/chat.postMessage")
                        .bearer_auth(&oauth_token)
                        .json(&payload)
                        .build()?;
                    let resp = client.execute(req);
                }
            }
            consumer.consume_messageset(ms)?;
        }
        consumer.commit_consumed()?;
    }
}

trait ToSlackMessage {
    fn to_slack_message(&self) -> String;
}

impl ToSlackMessage for DependencyEvent {
    fn to_slack_message(&self) -> String {
        format!("Vulnerable package found in: {}\nWhat package is affected? {} {}\nWhere was this found? Commit {} on branch {}\nWhat is the problem? {}\nHow serious is it? CVSS {}\nWhere can I find out more? {}",
            self.application_name.to_string(),
            self.affected_package.to_string(),
            self.installed_version.to_string(),
            self.git_commit_hash.to_string(),
            self.git_branch.to_string(),
            self.advisory_description.to_string(),
            self.cvss.to_string(),
            self.advisory_url.to_string()
        )
    }
}
