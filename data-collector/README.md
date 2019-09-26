# Kiln Data Collector

This is the HTTP service for tools to send data to for sending to the Kafka cluster. It is built using the [Serverless framework](https://serverless.com/) and is deployed to AWS as an API gateway and Lambda function written in Rust.

## Deploying
- Create an environment config file for the stage you want to deploy in `config/` such as `config/dev.yml`. The contents of this file should conform to the format described in the Configuration section below.
- Run `yarn` to install dependencies
- Run `npx serverless deploy --stage dev` replacing `dev` with the stage you want to deploy

Tool reports are published to a Kafka topic called "ToolReports". If you do not have auto topic creation enabled for your cluster, you will need to crate this topic.

## Configuration
YAML is used for configuring the data-collector deployment. An example configuration file is included below.
```YAML
tags:
    team: Infosec

securityGroupIds:
    - sg-123456789abcdef

subnetIds:
    - subnet-123456789abcdef
    - subnet-987654321fedcba

region: eu-west-1

kafka_bootstrap_tls: "b-1.kafka.a12bcd.c2.kafka.eu-west-1.amazonaws.com:9094,b-3.kafka.a12bcd.c2.kafka.eu-west-1.amazonaws.com:9094,b-2.kafka.a12bcd.c2.kafka.eu-west-1.amazonaws.com:9094"
```

## Request & Response Documentation

You shouldn't generally need to make manual requests to the data-collector, instead prefer to use the ToolReport struct from kiln_lib and serialise that to JSON before sending to the data-collector. If you do need to make a manual request to the data-collector, see [docs/request-response.md](docs/request-response.md).
