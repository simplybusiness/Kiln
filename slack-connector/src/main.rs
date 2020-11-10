#[macro_use]
extern crate slog;

use avro_rs::Reader;
use bytes::buf::ext::BufExt;
use bytes::Bytes;
use chrono::{SecondsFormat, Utc};
use failure::err_msg;
use futures::stream::{StreamExt, TryStreamExt};
use futures_util::sink::SinkExt;
use kiln_lib::dependency_event::DependencyEvent;
use kiln_lib::kafka::*;
use kiln_lib::log::NestedJsonFmt;
use kiln_lib::traits::Hashable;
use rdkafka::consumer::Consumer;
use rdkafka::message::Message;
use rdkafka::Offset;
use reqwest::Client;
use reqwest::Method;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::boxed::Box;
use std::convert::TryFrom;
use std::env;
use std::error::Error;
use std::sync::Arc;
use std::time::Duration;

use slog::o;
use slog::Drain;
use slog::{FnValue, PushFnValue};
use slog_derive::SerdeValue;
use uuid::Uuid;

const SERVICE_NAME: &str = "slack-connector";

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
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
            "event.id" => FnValue(move |_| {
                Uuid::new_v4().to_hyphenated().to_string()
            }),
        ),
    );

    let error_logger = root_logger.new(o!("event.type" => EventType(vec!("error".to_string()))));

    let oauth_token = env::var("OAUTH2_TOKEN").map_err(|err| {
        error!(error_logger, "Required Env Var OAUTH2_TOKEN not provided";
            o!(
                "error.message" => err.to_string()
            )
        );
        err
    })?;

    let channel_id = env::var("SLACK_CHANNEL_ID").map_err(|err| {
        error!(error_logger, "Required Env Var SLACK_CHANNEL_ID not provided";
            o!(
                "error.message" => err.to_string()
            )
        );
        err
    })?;

    let config = get_bootstrap_config(&mut env::vars()).map_err(|err| {
        error!(error_logger, "Error building Kafka configuration";
            o!(
                "error.message" => err.to_string()
            )
        );
        err
    })?;

    let consumer = Arc::new(
        build_kafka_consumer(config, "slack-connector".to_string()).map_err(|err| {
            error!(error_logger, "Could not build Kafka consumer";
                o!(
                    "error.message" => err.to_string()
                )
            );
            err
        })?,
    );

    let client = Client::new();
    let (queue_tx, queue_rx) =
        futures::channel::mpsc::channel::<(i64, Vec<Result<DependencyEvent, failure::Error>>)>(10);

    consumer.subscribe(&["DependencyEvents"]).map_err(|err| {
        error!(error_logger, "Could not subscribe to DependencyEvents Kafka topic";
            o!(
                "error.message" => err.to_string()
            )
        );
        err
    })?;

    let avro_bytes = consumer.start_with(Duration::from_secs(1), false);
    let consumer_error_logger = &error_logger;
    let mut events = avro_bytes
        .map_ok(|message| message.detach())
        .map_err(|err| failure::Error::from_boxed_compat(Box::new(err)))
        .and_then(move |message| async move {
            message
                .payload()
                .ok_or_else(|| {
                    warn!(consumer_error_logger, "Received empty payload from Kafka");
                    err_msg("Received empty payload from Kafka")
                })
                .map(|msg| (message.offset(), Bytes::copy_from_slice(msg)))
        })
        .and_then(|(offset, body_bytes)| async move {
            let reader_result = Reader::new(body_bytes.reader());
            match reader_result {
                Ok(reader) => Ok((offset, reader)),
                Err(err) => {
                    error!(consumer_error_logger, "Could not parse Avro value from message";
                        o!(
                            "error.message" => err.to_string()
                        )
                    );
                    Err(err_msg("Could not parse Avro value from message"))
                }
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
    let dispatch_error_logger = &error_logger;

    let slack_dispatch = queue_rx
        .then(|(offset, events)| async move {
            let trace_id = Uuid::new_v4().to_hyphenated().to_string();
            let dispatch_error_logger = dispatch_error_logger.new(o!("trace.id" => trace_id));
            for event in events.iter() {
                match event {
                    Err(err) => {
                        error!(dispatch_error_logger, "Error while processing events for offset {} in DependencyEvents topic", offset;
                            o!(
                                "error.message" => err.to_string(),
                            )
                        );
                    },
                    Ok(event) => {
                        if !event.suppressed {
                            while let Err(err) = try_send_slack_message(dispatch_channel_id, event, dispatch_client, dispatch_oauth_token).await {
                                match err {
                                    SlackSendError::RateLimited(delay) => {
                                        error!(dispatch_error_logger, "Rate limited by Slack while sending message for EventID {} (Parent EventID {}), waiting {} seconds before retrying", event.event_id, event.parent_event_id, delay.as_secs());
                                        futures_timer::Delay::new(delay).await;
                                    },
                                    SlackSendError::Unknown(err) => {
                                        error!(dispatch_error_logger, "Could not send Slack message for EventID {} (Parent EventID {})", event.event_id, event.parent_event_id;
                                            o!(
                                                "error.message" => err.to_string()
                                            ));
                                        futures_timer::Delay::new(Duration::from_secs(1)).await;
                                    }
                                }
                            }
                            futures_timer::Delay::new(Duration::from_secs(1)).await;
                        }
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

#[derive(Clone, SerdeValue, Serialize, Deserialize)]
struct EventType(Vec<String>);

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
        "text": event.to_slack_message()
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
