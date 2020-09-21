# UNRELEASED
* Nothing... yet

# 0.2.1 - 2020/09/21
## Report-parser
* Fixed an issue from 0.2.0 where the crate version upgrade was not propagated to Cargo.lock

# 0.2.0 - 2020/09/16

## CLI
* Added progress bars to show tool image pull progress
* Added support for providing a custom tool image for CLI to use 
* Added support for issue suppression
* Replaced `shiplift` crate with `bollard` for interacting with Docker API
* Added support for cleaning up old tool images
* Added support for running tools in offline mode
* Changed how tool containers are named to support multiple concurrent tool executions for CI environments
* Changed what image tag the CLI uses by default for tools. When built in Release mode, it will use an image tag with the same version as the CLI. When built in debug mode, it will use git-latest.

## Data-Collector
* Replace `Kafka` crate with `rdkafka`
* Upgraded to Actix_web 2.0
* Changed how custom CA Certificates are handled by including certificates at `/tls` in system CA bundle
* Added a /health endpoint that returns an HTTP 200 to support load balancer health checks
* Added Elastic Common Schema compatible JSON logging output

## Data-forwarder
* Added probe for CA bundle
* Added support for reading suppressed issues from kiln.toml in project root
* Added retry logic using fibonacci backoff to be more resilient to transient network issues

## Report-parser
* Replace `Kafka` crate with `rdkafka`
* Changed how custom CA Certificates are handled by including certificates at `/tls` in system CA bundle
* Added support for suppressed issues. If an issue should be suppressed, it will still be produced to Kafka, but with a flag indicating whether it should be suppressed
* Added support for customising the URL used to fetch NIST NVD data to support mirroring
* Added Elastic Common Schema compatible JSON logging output

## Slack-connector
* Replace `Kafka` crate with `rdkafka`
* Changed how custom CA Certificates are handled by including certificates at `/tls` in system CA bundle
* Issues that should be suppressed won't be posted to Slack
* Switched to Async HTTP client
* Added support for queueing messages to respect Slack rate limits and retry on failure
* Added Elastic Common Schema compatible JSON logging output

## Tools
### Bundler-audit
* Add CA Certificates package to Docker image
* If offline flag is provided by CLI, the vulnerability database won't be updated before running
* Changed docker tags used to remove tool version, which will be handled by changing the semver compatible version used in the tag

## Kiln_lib
* Replace `Kafka` crate with `rdkafka`
* Changed OpenSSL to use vendored version and static linking
* Upgraded to Actix_web 2.0
* Added support for issue suppression

# 0.1.0 - 2020/01/08

* Added initial version of Data-collector component
* Added initial version of Report-parser component
* Added initial version of Slack connector
* Added initial version of CLI
* Added Bundler-audit tool image, bundler-audit version 0.6.1
