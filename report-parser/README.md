# Kiln Report Parser 

This component handler the process of taking raw tool reports, parsing them into individual findings and enriching that data. It is built as a console app and deployable as a Docker container.

## Building
First, ensure you have cargo-make installed by running `cargo install cargo-make`. Then from this directory, run `cargo make build-report-parser-docker`. This will run linting, unit tests, check the crate builds, then build the crate for the "musl" target needed to create a static binary and finally build and tag the Alpine linux-based kiln/report-parser docker image.

## Deploying
See the [suggested Kafka deployment](../docs/suggested_kafka_deployment.md) documentation to understand how this component should be deployed and what it needs to be able to communicate with. The report-parser container is setup with an entrypoint, so no command needs to be passed to the container when calling docker run.

Tool reports are consumed from to a Kafka topic called "ToolReports" and vulnerable dependency events are published to a topic called "DependencyEvents". If you do not have auto topic creation enabled for your cluster, you will need to create these topics.

**Please note: There is currently a known issue with running this component in a HA configuration, which results in duplicate messages being written to the DependencyEvents Kafka topic. This is being tracked in #139.**

## Configuration
This component is configured using environment variables. Ensure that the environment variable `KAFKA_BOOTSTRAP_TLS` is set to a comma separated list of host:port pairs to bootstrap connectivity to your Kafka cluster over TLS.

By default, this component will validate that hosts in the `KAFKA_BOOTSTRAP_TLS` environment variable are valid domain names. If you need to connect to a cluster using bare hostnames, you can disable this validation by setting: `DISABLE_KAFKA_DOMAIN_VALIDATION=true`.

If your Kafka cluster uses TLS certificates issued by a private Certificate Authority, you will need to provide the CA Certificate in PEM format so that certificate validation can be performed when connecting to the Kafka cluster. You should do this by including the CA certificate in PEM format in the `/tls` directory of the container, probably through a volume mount.

If you want to provide an alternative URL for downloading NIST NVD data, this can be configured by starting the report-paser with the `NVD_BASE_URL` environment variable set to the URL of your NVD mirror.
