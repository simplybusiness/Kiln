# Kiln Slack Connector

This component handles the process of taking parsed DependencyEvents and posting them to a Slack channel of the user's choosing. It is built as a console app and deployable as a Docker container.

## Building
First, ensure you have cargo-make installed by running `cargo install cargo-make`. Then from this directory, run `cargo make build-slack-connector-docker`. This will run linting, unit tests, check the crate builds, then build the crate for the "musl" target needed to create a static binary and finally build and tag the Alpine linux-based kiln/slack-connector docker image.

## Deploying
See the [suggested Kafka deployment](../docs/suggested_kafka_deployment.md) documentation to understand how this component should be deployed and what it needs to be able to communicate with. The slack-connector container is setup with an entrypoint, so no command needs to be passed to the container when calling docker run.

DependencyEvents are consumed from a Kafka topic called "DependencyEvents". If you do not have auto topic creation enabled for your cluster, you will need to create this topic.

To grant the Slack Connector access to your Slack workspace, you will need to register it as a Slack app for your workspace and generate an access token for it. Instructions for this can be found [here](https://api.slack.com/authentication/basics). Follow those instructions up to and including "Installing the app to a workspace". When you get to the step where you're adding OAuth Scopes, add the following scopes: `channels:read` and `chat:write.public`. By the end of that step, you should have an OAuth2 Access token for the Slack Connector. It is important that this token is handled carefully, because it grants access to your Slack workspace.

## Configuration
This component is configured using environment variables. Ensure that the environment variable `KAFKA_BOOTSTRAP_TLS` is set to a comma separated list of host:port pairs to bootstrap connectivity to your Kafka cluster over TLS.

By default, this component will validate that hosts in the `KAFKA_BOOTSTRAP_TLS` environment variable are valid domain names. If you need to connect to a cluster using bare hostnames, you can disable this validation by setting: `DISABLE_KAFKA_DOMAIN_VALIDATION=true`.

If your Kafka cluster uses TLS certificates issued by a private Certificate Authority, you will need to provide the CA Certificate in PEM format so that certificate validation can be performed when connecting to the Kafka cluster. You should do this by including the CA certificate in PEM format in the `/tls` directory of the container, probably through a volume mount.

You will also need the Channel ID for the Slack Channel you want to route notifications to. This can be found by opening Slack in a web browser and loading the channel you want Kiln to send notifications to. The last components of the URL path will contain the channel ID and will begin with a 'C'. This is supplied to the connector using the `SLACK_CHANNEL_ID` environment variable.

Lastly, you will need to supply the OAuth2 access token you created earlier as the `OAUTH2_TOKEN` environment variable. This value is a secret and should be handled accordingly to avoid accidental disclosure in shell history, logs etc. Unfortunately the topic of secrets management is out of the scope of this documentation.

At present Kiln supports authentication between brokers and producers/consumers using the SASL_PLAIN mechanism. Authentication is optional and configured by setting the `ENABLE_KAFKA_AUTH` environment variable. If this variable is set, you also need to supply the username and password for authentication using `KAFKA_SASL_AUTH_USERNAME` and `KAFKA_SASL_AUTH_PASSWORD` environment variables respectively.  

