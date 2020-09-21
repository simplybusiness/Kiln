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
use rdkafka::Offset;
use reqwest::Client;
use reqwest::Method;
use serde_json::{json, Value};
use std::boxed::Box;
use std::convert::TryFrom;
use std::env;
use std::error::Error;
use std::sync::Arc;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    //env_logger::init();

    let oauth_token =
        env::var("OAUTH2_TOKEN").expect("Error: Required Env Var OAUTH2_TOKEN not provided");
    let channel_id = env::var("SLACK_CHANNEL_ID")
        .expect("Error: Required Env Var SLACK_CHANNEL_ID not provided");
    let config = get_bootstrap_config(&mut env::vars())
        .map_err(|err| failure::err_msg(format!("Configuration Error: {}", err)))?;

    let consumer = Arc::new(
        build_kafka_consumer(config, "slack-connector".to_string())
            .map_err(|err| err_msg(format!("Kafka Consumer Error: {}", err.to_string())))?,
    );

    let client = Client::new();
    let (queue_tx, queue_rx) =
        futures::channel::mpsc::channel::<(i64, Vec<Result<DependencyEvent, _>>)>(10);

    consumer.subscribe(&["DependencyEvents"])?;
    let avro_bytes = consumer.start_with(Duration::from_secs(1), false);
    let mut events = avro_bytes
        .map_ok(|message| message.detach())
        .map_err(|err| failure::Error::from_boxed_compat(Box::new(err)))
        .and_then(move |message| async move {
            message
                .payload()
                .ok_or_else(|| err_msg("Received empty payload from Kafka"))
                .map(|msg| (message.offset(), Bytes::copy_from_slice(msg)))
        })
        .and_then(|(offset, body_bytes)| async move {
            let reader_result = Reader::new(body_bytes.reader());
            match reader_result {
                Ok(reader) => Ok((offset, reader)),
                Err(_) => Err(err_msg("Could not parse avro value from bytes")),
            }
        })
        .map_ok(|(offset, reader)| {
            (
                offset,
                reader
                    .map(|event| DependencyEvent::try_from(event?))
                    .collect(),
            )
        })
        .boxed();

    let dispatch_consumer = &consumer;
    let dispatch_oauth_token = &oauth_token;
    let dispatch_client = &client;
    let dispatch_channel_id = &channel_id;

    let slack_dispatch = queue_rx
        .then(|(offset, events)| async move {
            for event in events.iter() {
                if let Err(err) = event {
                    eprintln!("Offset: {}, {}", offset, err);
                }
                if let Ok(event) = event {
                    if !event.suppressed {
                        while let Err(err) = try_send_slack_message(dispatch_channel_id, event, dispatch_client, dispatch_oauth_token).await {
                            match err {
                                SlackSendError::RateLimited(delay) => {
                                    eprintln!("Error sending slack message for EventID {}, Parent Event ID {}: Encountered rate limit, waiting {} seconds before trying again", event.event_id, event.parent_event_id, delay.as_secs());
                                    futures_timer::Delay::new(delay).await;
                                },
                                SlackSendError::Unknown(err) => {
                                    eprintln!("Error sending slack message for EventID {}, Parent Event ID {}: {}", event.event_id, event.parent_event_id, err);
                                    futures_timer::Delay::new(Duration::from_secs(1)).await;
                                }
                            }
                        }
                        futures_timer::Delay::new(Duration::from_secs(1)).await;
                    }
                }
            }
            dispatch_consumer.assignment().unwrap().set_all_offsets(Offset::from_raw(offset));
            Ok::<(), failure::Error>(())
        })
        .collect::<Vec<_>>();

    let mut queue_tx_mapped = queue_tx.sink_err_into();

    let queue_all = queue_tx_mapped.send_all(&mut events);

    let results = futures::join!(queue_all, slack_dispatch);
    results.0?;
    for result in results.1 {
        result?
    }
    Ok(())
}

trait ToSlackMessage {
    fn to_slack_message(&self) -> String;
}

impl ToSlackMessage for DependencyEvent {
    fn to_slack_message(&self) -> String {
        format!("Vulnerable package(s) found in: *{}*\n*Commit Scanned:* {}\n*Branch:* {}\n*Package Affected:* {} {}\n     *Issue*: {}\n    *Vulnerability Details:* {}\n    *CVSS v3 Score:* {} \n  *Hash:* {}\n",
            self.application_name.to_string(),
            self.git_commit_hash.to_string(),
            self.git_branch.to_string(),
            self.affected_package.to_string(),
            self.installed_version.to_string(),
            self.advisory_description.to_string(),
            self.advisory_url.to_string(),
            self.cvss.to_string(),
            hex::encode(self.hash()),
        )
    }
}

enum SlackSendError {
    RateLimited(std::time::Duration),
    Unknown(failure::Error),
}

impl From<reqwest::Error> for SlackSendError {
    fn from(val: reqwest::Error) -> Self {
        SlackSendError::Unknown(val.into())
    }
}

async fn try_send_slack_message<T: AsRef<str> + serde::ser::Serialize + std::fmt::Display>(
    channel_id: T,
    event: &DependencyEvent,
    client: &reqwest::Client,
    oauth_token: T,
) -> Result<(), SlackSendError> {
    let payload = json!({
        "channel": channel_id,
        "blocks": [
            { 
                "text": {
                    "type": "mrkdwn",
                    "text": event.to_slack_message()
            },
            "accessory": {
                "type": "button",
                "text": {
                    "type": "plain_text",
                    "text": "Surpress Issue",
                    "emoji": true
                    },
                "value": "click_me_123"
            }
        },
        {
            "text": {
                "type": "mrkdwn",
                "text": "hi"
            }
        }
        ]
        
    });
    let req = client
        .request(Method::POST, "https://slack.com/api/chat.postMessage")
        .bearer_auth(&oauth_token)
        .json(&payload)
        .build()
        .map_err(SlackSendError::from)?;
    let resp = client.execute(req).await?;
    let retry_delay: Option<std::time::Duration> = resp
        .headers()
        .get("Retry-After")
        .map(|val| Duration::from_secs(val.to_str().unwrap().parse().unwrap()));

    let resp_body: Value = resp.json().await?;
    if !resp_body.get("ok").unwrap().as_bool().unwrap() {
        let cause = resp_body.get("error").unwrap().as_str().unwrap();
        eprintln!(
            "Error sending message for event {}, parent {}, {}",
            event.event_id.to_string(),
            event.parent_event_id.to_string(),
            cause
        );
        return if cause == "rate_limited" {
            Err(SlackSendError::RateLimited(
                retry_delay.unwrap_or(Duration::from_secs(30)),
            ))
        } else {
            Err(SlackSendError::Unknown(err_msg(cause.to_owned())))
        };
    }
    Ok(())
}
