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

## Request & Response Documentation

This service has a single endpoint, expose behind an AWS API gateway.

Requests to this endpoint must be in JSON format in the following format:
```
{
        application_name: String,
        git_branch: String,
        git_commit_hash: String,
        tool_name: String,
        tool_output: String,
        output_format: String(Json|PlainText),
        start_time: Timestamp string in RFC3339 format,
        end_time: Timestamp string in RFC3339 format,
        environment: String(Local|CI),
        tool_version: String (optional)
}
```

(Where the above conflicts with the validation logic, the validation logic and corresponding tests are correct, and creating an issue to correct this would be appreciated!)

All fields not explicitly marked optional are required. If a field is present, it must not be empty. A successful request will return an HTTP 200 OK with an empty body.

A request that fails validation will return an HTTP 400 Bad Request with a JSON body containing an "error_code" field and an "error_message" field with a brief description of the reason the request body didn't pass validation.
