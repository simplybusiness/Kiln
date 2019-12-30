use addr::DomainName;
use kafka::client::{Compression, GroupOffsetStorage, SecurityConfig};
use kafka::consumer::Consumer;
use kafka::error::Error as KafkaError;
use kafka::producer::Producer;
use openssl::error::ErrorStack;
use openssl::ssl::{SslConnector, SslMethod, SslVerifyMode, SslVersion};

#[derive(Debug)]
pub struct KafkaBootstrapTlsConfig(Vec<String>);

pub fn get_bootstrap_config<I>(vars: &mut I) -> Result<KafkaBootstrapTlsConfig, String>
where
    I: Iterator<Item = (String, String)>,
{
    let local_vars: Vec<(String, String)> = vars.collect();
    let disable_kafka_domain_validation = match local_vars
        .iter()
        .find(|var| var.0 == "DISABLE_KAFKA_DOMAIN_VALIDATION")
    {
        None => Ok(false),
        Some(var) => {
            if var.1.is_empty() {
                return Err(
                    "Optional environment variable present but empty: DISABLE_KAFKA_DOMAIN_VALIDATION"
                        .to_owned(),
                );
            } else {
                match var.1.as_ref() {
                    "true" => Ok(true),
                    "false" => Ok(false),
                    _ => Err("Optional environment variable did not pass validation: DISABLE_KAFKA_DOMAIN_VALIDATION".to_owned())
                }
            }
        }
    }?;

    let kafka_bootstrap_tls = match local_vars.iter().find(|var| var.0 == "KAFKA_BOOTSTRAP_TLS") {
        None => Err("Required environment variable missing: KAFKA_BOOTSTRAP_TLS".to_owned()),
        Some(var) => {
            if var.1.is_empty() {
                return Err(
                    "Required environment variable present but empty: KAFKA_BOOTSTRAP_TLS"
                        .to_owned(),
                );
            } else {
                let raw_hosts: Vec<String> = var.1.split(',').map(|s| s.to_owned()).collect();
                let valid = raw_hosts.iter().all(|x| {
                    let parts: Vec<&str> = x.split(':').collect();
                    let domain_valid = if disable_kafka_domain_validation {
                        true
                    } else {
                        parts[0].parse::<DomainName>().is_ok()
                    };
                    let port_valid = u16::from_str_radix(parts[1], 10).is_ok();
                    domain_valid && port_valid
                });
                if valid {
                    Ok(raw_hosts)
                } else {
                    Err(
                        "KAFKA_BOOTSTRAP_TLS environment variable did not pass validation"
                            .to_owned(),
                    )
                }
            }
        }
    }?;

    Ok(KafkaBootstrapTlsConfig(kafka_bootstrap_tls))
}

pub fn build_ssl_connector() -> Result<SslConnector, ErrorStack> {
    let mut ssl_connector_builder = SslConnector::builder(SslMethod::tls())?;
    ssl_connector_builder.set_verify(SslVerifyMode::PEER);
    ssl_connector_builder.set_default_verify_paths()?;
    ssl_connector_builder.set_min_proto_version(Some(SslVersion::TLS1_2))?;
    ssl_connector_builder.set_cipher_list("ECDHE-ECDSA-AES256-GCM-SHA384:ECDHE-RSA-AES256-GCM-SHA384:ECDHE-ECDSA-AES128-GCM-SHA256:ECDHE-RSA-AES128-GCM-SHA256")?;
    Ok(ssl_connector_builder.build())
}

pub fn build_kafka_producer(
    config: KafkaBootstrapTlsConfig,
    ssl_connector: SslConnector,
) -> Result<Producer, KafkaError> {
    let security_config = SecurityConfig::new(ssl_connector).with_hostname_verification(true);

    Producer::from_hosts(config.0)
        .with_compression(Compression::GZIP)
        .with_security(security_config)
        .create()
}

pub fn build_kafka_consumer(
    config: KafkaBootstrapTlsConfig,
    topic: String,
    consumer_group_name: String,
    ssl_connector: SslConnector,
) -> Result<Consumer, KafkaError> {
    let security_config = SecurityConfig::new(ssl_connector).with_hostname_verification(true);

    Consumer::from_hosts(config.0)
        .with_security(security_config)
        .with_topic(topic)
        .with_group(consumer_group_name)
        .with_offset_storage(GroupOffsetStorage::Kafka)
        .create()
}
