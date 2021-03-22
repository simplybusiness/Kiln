use addr::{parser::DomainName, psl::List};
use openssl_probe::ProbeResult;
use rdkafka::config::ClientConfig;
use rdkafka::consumer::stream_consumer::StreamConsumer;
use rdkafka::error::KafkaError;
use rdkafka::producer::future_producer::FutureProducer;
use std::fmt::Display;

#[derive(Debug, Clone)]
pub struct KafkaBootstrapTlsConfig(Vec<String>);

#[derive(Debug)]
pub enum ValidationFailureReason {
    Missing,
    PresentButEmpty,
    CouldNotBeParsed,
}

impl Display for ValidationFailureReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationFailureReason::Missing => f.write_str("value is missing"),
            ValidationFailureReason::PresentButEmpty => f.write_str("value is present but empty"),
            ValidationFailureReason::CouldNotBeParsed => f.write_str("value could not be parsed"),
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum KafkaConfigError {
    #[error("Required environment variable {var} failed validation because {reason}")]
    RequiredValueValidationFailure {
        var: String,
        reason: ValidationFailureReason,
    },
    #[error("Optional environment variable {var} failed validation because {reason}")]
    OptionalValueValidationFailure {
        var: String,
        reason: ValidationFailureReason,
    },
    #[error("Kafka client could not be created")]
    KafkaError(#[from] KafkaError),
    #[error("Could not find TLS trust store")]
    TlsTrustStore,
}

pub fn get_bootstrap_config<I>(vars: &mut I) -> Result<KafkaBootstrapTlsConfig, KafkaConfigError>
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
                return Err(KafkaConfigError::OptionalValueValidationFailure {
                    var: "DISABLE_KAFKA_DOMAIN_VALIDATION".into(),
                    reason: ValidationFailureReason::PresentButEmpty,
                });
            } else {
                match var.1.as_ref() {
                    "true" => Ok(true),
                    "false" => Ok(false),
                    _ => Err(KafkaConfigError::OptionalValueValidationFailure {
                        var: "DISABLE_KAFKA_DOMAIN_VALIDATION".into(),
                        reason: ValidationFailureReason::CouldNotBeParsed,
                    }),
                }
            }
        }
    }?;

    let kafka_bootstrap_tls = match local_vars.iter().find(|var| var.0 == "KAFKA_BOOTSTRAP_TLS") {
        None => Err(KafkaConfigError::RequiredValueValidationFailure {
            var: "KAFKA_BOOTSTRAP_TLS".into(),
            reason: ValidationFailureReason::Missing,
        }),
        Some(var) => {
            if var.1.is_empty() {
                return Err(KafkaConfigError::RequiredValueValidationFailure {
                    var: "KAFKA_BOOTSTRAP_TLS".into(),
                    reason: ValidationFailureReason::PresentButEmpty,
                });
            } else {
                let raw_hosts: Vec<String> = var.1.split(',').map(|s| s.to_owned()).collect();
                let valid = raw_hosts.iter().all(|x| {
                    let parts: Vec<&str> = x.split(':').collect();
                    let domain_valid = if disable_kafka_domain_validation {
                        true
                    } else {
                        List.parse_domain_name(parts[0])
                            .map(|name| name.has_known_suffix())
                            .unwrap_or(false)
                    };
                    let port_valid = u16::from_str_radix(parts[1], 10).is_ok();
                    domain_valid && port_valid
                });
                if valid {
                    Ok(raw_hosts)
                } else {
                    Err(KafkaConfigError::RequiredValueValidationFailure {
                        var: "KAFKA_BOOTSTRAP_TLS".into(),
                        reason: ValidationFailureReason::CouldNotBeParsed,
                    })
                }
            }
        }
    }?;

    Ok(KafkaBootstrapTlsConfig(kafka_bootstrap_tls))
}

pub fn build_kafka_producer(
    config: KafkaBootstrapTlsConfig,
) -> Result<FutureProducer, KafkaConfigError> {
    let cert_probe_result = openssl_probe::probe();
    let cert_location = match cert_probe_result {
        ProbeResult { cert_file, .. } if cert_file.is_some() => Ok(cert_file),
        ProbeResult { cert_dir, .. } if cert_dir.is_some() => Ok(cert_dir),
        _ => Err(KafkaConfigError::TlsTrustStore),
    }?;

    ClientConfig::new()
        .set("metadata.broker.list", &config.0.join(","))
        .set("compression.type", "gzip")
        .set("security.protocol", "SSL")
        .set("ssl.cipher.suites", "ECDHE-ECDSA-AES256-GCM-SHA384,ECDHE-RSA-AES256-GCM-SHA384,ECDHE-ECDSA-AES128-GCM-SHA256,ECDHE-RSA-AES128-GCM-SHA256")
        .set("ssl.ca.location", cert_location.unwrap().to_string_lossy())
        .create()
        .map_err(|err| err.into())
}

pub fn build_kafka_consumer(
    config: KafkaBootstrapTlsConfig,
    consumer_group_name: String,
) -> Result<StreamConsumer, KafkaConfigError> {
    let cert_probe_result = openssl_probe::probe();
    let cert_location = match cert_probe_result {
        ProbeResult { cert_file, .. } if cert_file.is_some() => Ok(cert_file),
        ProbeResult { cert_dir, .. } if cert_dir.is_some() => Ok(cert_dir),
        _ => Err(KafkaConfigError::TlsTrustStore),
    }?;

    ClientConfig::new()
        .set("metadata.broker.list", &config.0.join(","))
        .set("group.id", &consumer_group_name)
        .set("compression.type", "gzip")
        .set("security.protocol", "SSL")
        .set("ssl.cipher.suites", "ECDHE-ECDSA-AES256-GCM-SHA384,ECDHE-RSA-AES256-GCM-SHA384,ECDHE-ECDSA-AES128-GCM-SHA256,ECDHE-RSA-AES128-GCM-SHA256")
        .set("ssl.ca.location", cert_location.unwrap().to_string_lossy())
        .create()
        .map_err(|err| err.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(unused_must_use)]
    #[tokio::test]
    async fn creating_kafka_producer_does_not_return_a_client_config_error() {
        let config =
            KafkaBootstrapTlsConfig(vec!["host1:1234".to_string(), "host2:1234".to_string()]);
        build_kafka_producer(config).unwrap();
    }

    #[allow(unused_must_use)]
    #[tokio::test]
    async fn creating_kafka_consumer_does_not_return_a_client_config_error() {
        let config =
            KafkaBootstrapTlsConfig(vec!["host1:1234".to_string(), "host2:1234".to_string()]);
        build_kafka_consumer(config, "TestConsumerGroup".to_string()).unwrap();
    }

    #[test]
    fn get_bootstrap_config_returns_config_when_environment_vars_present_and_valid() {
        let hostname =
            "my.kafka.host.example.com:1234,my.second.kafka.host.example.com:1234".to_owned();
        let mut fake_vars = vec![("KAFKA_BOOTSTRAP_TLS".to_owned(), hostname)].into_iter();

        let expected = vec![
            "my.kafka.host.example.com:1234".to_owned(),
            "my.second.kafka.host.example.com:1234".to_owned(),
        ];

        let actual = get_bootstrap_config(&mut fake_vars).expect("expected Ok(_) value");

        assert_eq!(actual.0, expected);
    }

    #[test]
    fn get_bootstrap_config_returns_error_when_environment_vars_missing() {
        let mut fake_vars = std::iter::empty::<(String, String)>();

        let actual = get_bootstrap_config(&mut fake_vars).expect_err("expected Err(_) value");

        assert_eq!(
            actual.to_string(),
            "Required environment variable KAFKA_BOOTSTRAP_TLS failed validation because value is missing"
        )
    }

    #[test]
    fn get_bootstrap_config_returns_error_when_environment_vars_present_but_empty() {
        let hostname = "".to_owned();
        let mut fake_vars = vec![("KAFKA_BOOTSTRAP_TLS".to_owned(), hostname.clone())].into_iter();

        let actual = get_bootstrap_config(&mut fake_vars).expect_err("expected Err(_) value");

        assert_eq!(
            actual.to_string(),
            "Required environment variable KAFKA_BOOTSTRAP_TLS failed validation because value is present but empty"
        )
    }

    #[test]
    fn get_bootstrap_config_returns_error_when_hostname_invalid_and_domain_validation_enabled() {
        let hostname = "kafka:1234".to_owned();
        let mut fake_vars = vec![("KAFKA_BOOTSTRAP_TLS".to_owned(), hostname.clone())].into_iter();

        let actual = get_bootstrap_config(&mut fake_vars).expect_err("expected Err(_) value");

        assert_eq!(
            actual.to_string(),
            "Required environment variable KAFKA_BOOTSTRAP_TLS failed validation because value could not be parsed"
        )
    }

    #[test]
    fn get_bootstrap_config_returns_configration_when_hostname_not_a_valid_domain_and_domain_validation_disabled(
    ) {
        let hostname = "kafka:1234".to_owned();
        let mut fake_vars = vec![
            ("KAFKA_BOOTSTRAP_TLS".to_owned(), hostname.clone()),
            (
                "DISABLE_KAFKA_DOMAIN_VALIDATION".to_owned(),
                "true".to_owned(),
            ),
        ]
        .into_iter();
        let expected = vec![hostname.clone()];

        let actual = get_bootstrap_config(&mut fake_vars).expect("expected Ok(_) value");

        assert_eq!(actual.0, expected)
    }

    #[test]
    fn get_bootstrap_config_returns_error_when_port_number_invalid() {
        let hostname = "my.kafka.host.example.com:1234567".to_owned();
        let mut fake_vars = vec![("KAFKA_BOOTSTRAP_TLS".to_owned(), hostname.clone())].into_iter();

        let actual = get_bootstrap_config(&mut fake_vars).expect_err("expected Err(_) value");

        assert_eq!(
            actual.to_string(),
            "Required environment variable KAFKA_BOOTSTRAP_TLS failed validation because value could not be parsed"
        )
    }

    #[test]
    fn get_bootstrap_config_returns_error_when_disable_kafka_domain_validation_present_but_empty() {
        let mut fake_vars =
            vec![("DISABLE_KAFKA_DOMAIN_VALIDATION".to_owned(), "".to_owned())].into_iter();
        let actual = get_bootstrap_config(&mut fake_vars).expect_err("expected Err(_) value");

        assert_eq!(
            actual.to_string(),
            "Optional environment variable DISABLE_KAFKA_DOMAIN_VALIDATION failed validation because value is present but empty"
        )
    }

    #[test]
    fn get_bootstrap_config_returns_error_when_disable_kafka_domain_validation_present_but_invalid()
    {
        let mut fake_vars = vec![(
            "DISABLE_KAFKA_DOMAIN_VALIDATION".to_owned(),
            "blah".to_owned(),
        )]
        .into_iter();
        let actual = get_bootstrap_config(&mut fake_vars).expect_err("expected Err(_) value");

        assert_eq!(
            actual.to_string(),
            "Optional environment variable DISABLE_KAFKA_DOMAIN_VALIDATION failed validation because value could not be parsed"
        )
    }
}
