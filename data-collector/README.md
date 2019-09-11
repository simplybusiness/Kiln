# Kiln Data Collector

This is the HTTP service for tools to send data to for sending to the Kafka cluster. It is built using the [Serverless framework](https://serverless.com/) and is deployed to AWS as an API gateway and Lambda function written in Rust.

## Deploying
- Create an environment config file for the stage you want to deploy in `config/` such as `config/dev.yml`. The contents of this file should conform to the format described in the Configuration section below.
- Run `yarn` to install dependencies
- Run `npx serverless deploy --stage dev` replacing `dev` with the stage you want to deploy

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
```
