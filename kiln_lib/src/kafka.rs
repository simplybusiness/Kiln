use addr::DomainName;
use openssl_probe::ProbeResult;
use rdkafka::config::ClientConfig;
use rdkafka::consumer::stream_consumer::StreamConsumer;
use rdkafka::error::KafkaError;
use rdkafka::producer::future_producer::FutureProducer;

#[derive(Debug, Clone)]
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

pub fn build_kafka_producer(config: KafkaBootstrapTlsConfig) -> Result<FutureProducer, KafkaError> {
    let cert_probe_result = openssl_probe::probe();
    let cert_location = match cert_probe_result {
        ProbeResult { cert_file, .. } if cert_file.is_some() => cert_file,
        ProbeResult { cert_dir, .. } if cert_dir.is_some() => cert_dir,
        _ => panic!("Could not find TLS Certificate Store"),
    };

    ClientConfig::new()
        .set("metadata.broker.list", &config.0.join(","))
        .set("compression.type", "gzip")
        .set("security.protocol", "SSL")
        .set("ssl.cipher.suites", "ECDHE-ECDSA-AES256-GCM-SHA384,ECDHE-RSA-AES256-GCM-SHA384,ECDHE-ECDSA-AES128-GCM-SHA256,ECDHE-RSA-AES128-GCM-SHA256")
        .set("ssl.ca.location", &cert_location.unwrap().to_string_lossy())
        .create()
}

pub fn build_kafka_consumer(
    config: KafkaBootstrapTlsConfig,
    consumer_group_name: String,
) -> Result<StreamConsumer, KafkaError> {
    let cert_probe_result = openssl_probe::probe();
    let cert_location = match cert_probe_result {
        ProbeResult { cert_file, .. } if cert_file.is_some() => cert_file,
        ProbeResult { cert_dir, .. } if cert_dir.is_some() => cert_dir,
        _ => panic!("Could not find TLS Certificate Store"),
    };

    ClientConfig::new()
        .set("metadata.broker.list", &config.0.join(","))
        .set("group.id", &consumer_group_name)
        .set("compression.type", "gzip")
        .set("security.protocol", "SSL")
        .set("ssl.cipher.suites", "ECDHE-ECDSA-AES256-GCM-SHA384,ECDHE-RSA-AES256-GCM-SHA384,ECDHE-ECDSA-AES128-GCM-SHA256,ECDHE-RSA-AES128-GCM-SHA256")
        .set("ssl.ca.location", &cert_location.unwrap().to_string_lossy())
        .create()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(unused_must_use)]
    #[test]
    fn creating_kafka_producer_does_not_return_a_client_config_error() {
        let config =
            KafkaBootstrapTlsConfig(vec!["host1:1234".to_string(), "host2:1234".to_string()]);
        build_kafka_producer(config).unwrap();
    }

    #[allow(unused_must_use)]
    #[test]
    fn creating_kafka_consumer_does_not_return_a_client_config_error() {
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
            "Required environment variable missing: KAFKA_BOOTSTRAP_TLS"
        )
    }

    #[test]
    fn get_bootstrap_config_returns_error_when_environment_vars_present_but_empty() {
        let hostname = "".to_owned();
        let mut fake_vars = vec![("KAFKA_BOOTSTRAP_TLS".to_owned(), hostname.clone())].into_iter();

        let actual = get_bootstrap_config(&mut fake_vars).expect_err("expected Err(_) value");

        assert_eq!(
            actual.to_string(),
            "Required environment variable present but empty: KAFKA_BOOTSTRAP_TLS"
        )
    }

    #[test]
    fn get_bootstrap_config_returns_error_when_hostname_invalid_and_domain_validation_enabled() {
        let hostname = "kafka:1234".to_owned();
        let mut fake_vars = vec![("KAFKA_BOOTSTRAP_TLS".to_owned(), hostname.clone())].into_iter();

        let actual = get_bootstrap_config(&mut fake_vars).expect_err("expected Err(_) value");

        assert_eq!(
            actual.to_string(),
            "KAFKA_BOOTSTRAP_TLS environment variable did not pass validation"
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
            "KAFKA_BOOTSTRAP_TLS environment variable did not pass validation"
        )
    }

    #[test]
    fn get_bootstrap_config_returns_error_when_disable_kafka_domain_validation_present_but_empty() {
        let mut fake_vars =
            vec![("DISABLE_KAFKA_DOMAIN_VALIDATION".to_owned(), "".to_owned())].into_iter();
        let actual = get_bootstrap_config(&mut fake_vars).expect_err("expected Err(_) value");

        assert_eq!(
            actual.to_string(),
            "Optional environment variable present but empty: DISABLE_KAFKA_DOMAIN_VALIDATION"
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
            "Optional environment variable did not pass validation: DISABLE_KAFKA_DOMAIN_VALIDATION"
        )
    }
}
