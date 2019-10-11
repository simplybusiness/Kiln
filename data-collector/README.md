# Kiln Data Collector

This is the HTTP service for tools to send data to for sending to the Kafka cluster. It is built using Actix-web and will be deployable as a docker container.

## Deploying
TBD

Tool reports are published to a Kafka topic called "ToolReports". If you do not have auto topic creation enabled for your cluster, you will need to crate this topic.

## Configuration
TBD 

## Request & Response Documentation

You shouldn't generally need to make manual requests to the data-collector, instead prefer to use the ToolReport struct from kiln_lib and serialise that to JSON before sending to the data-collector. If you do need to make a manual request to the data-collector, see [docs/request-response.md](docs/request-response.md).
