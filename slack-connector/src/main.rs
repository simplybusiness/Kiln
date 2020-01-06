use avro_rs::Reader;
use failure::err_msg;
use kiln_lib::kafka::*;
use kiln_lib::dependency_event::DependencyEvent;
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

    loop {
        for ms in consumer.poll().unwrap().iter() {
            for m in ms.messages() {
                let reader = Reader::new(m.value)?;
                for value in reader {
                    let event = DependencyEvent::try_from(value?)?;
                    println!("{:?}", event);
                }
            }
            consumer.consume_messageset(ms)?;
        }
        consumer.commit_consumed()?;
    }
}
