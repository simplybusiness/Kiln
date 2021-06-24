# Kiln Data Collector

This is the HTTP service for tools to send data to for sending to the Kafka cluster. It is built using Actix-web and is deployable as a docker container.

## Building
First, ensure you have cargo-make installed by running `cargo install cargo-make`. Then from this directory, run `cargo make build-data-collector-docker`. This will run linting, unit tests, check the crate builds, then build the crate for the "musl" target needed to create a static binary and finally build and tag the Alpine linux-based kiln/data-collector docker image.

## Deploying
See the [suggested Kafka deployment](../docs/suggested_kafka_deployment.md) documentation to understand how this component should be deployed and what it needs to be able to communicate with. The data-collector container is setup with an entrypoint, so no command needs to be passed to the container when calling docker run.

Tool reports are published to a Kafka topic called "ToolReports". If you do not have auto topic creation enabled for your cluster, you will need to crate this topic. Please note that for the ToolReports topic we recommend changing the default kafka message size from 1MB to 10MB to support potentially large amount of tool output by setting the parameters `max.message.bytes` and `replica.fetch.max.bytes`.

## Configuration
This component is configured using environment variables. Ensure that the environment variable `KAFKA_BOOTSTRAP_TLS` is set to a comma separated list of host:port pairs to bootstrap connectivity to your Kafka cluster over TLS.

By default, this component will validate that hosts in the `KAFKA_BOOTSTRAP_TLS` environment variable are valid domain names. If you need to connect to a cluster using bare hostnames, you can disable this validation by setting: `DISABLE_KAFKA_DOMAIN_VALIDATION=true`.

If your Kafka cluster uses TLS certificates issued by a private Certificate Authority, you will need to provide the CA Certificate in PEM format so that certificate validation can be performed when connecting to the Kafka cluster. You should do this by including the CA certificate in PEM format in the `/tls` directory of the container, probably through a volume mount. 

## Request & Response Documentation

You shouldn't generally need to make manual requests to the data-collector, instead prefer to use the ToolReport struct from kiln_lib and serialise that to JSON before sending to the data-collector. If you do need to make a manual request to the data-collector, see [docs/request-response.md](docs/request-response.md).
