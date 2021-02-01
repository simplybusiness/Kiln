# UNRELEASED
* Nothing... yet

# 0.4.2 - 2021/02/01
## Data-collector
* Fixed a bug in the formatting of log output that was causing logs to be incompatible with Elastic Common Schema. The `source.address` field contained an ip address:port pair, when it should have just been an IP address.

# 0.4.1 - 2021/01/07
## Report Parser
* Fixed a bug in how NIST NVD data is pulled that meant data after 2020 would not be pulled because of a hardcoded year

# 0.4.0 - 2021/01/06
## Data Collector
* Fixed the format of ECS formatted log data to correctly use nested objects
## Report Parser
* Fixed the format of ECS formatted log data to correctly use nested objects
## Slack Connector
* Fixed the format of ECS formatted log data to correctly use nested objects
## CLI
* KILN_SCAN_ENV environment variable is read by CLI to tell Data Forwarder what environment scan is running in
* If running a release build and a Docker image for version the tool being run is present locally, use local image instead of repulling
* Add support for pulling tool images from private Docker registries
* Add support for providing credentials to authenticate Docker API requests
* Upgraded to support Docker Registry Image Manifest format V2 Schema 2, which is used by AWS ECR
* Improved error handling and error messages when pulling Docker images fails

# 0.3.2 - 2020/10/27
## CLI
* Fixed an issue in the path mapping feature introduced in 0.3.1 that would fail to correctly map the path supplied with `--work-dir` to a path inside a container running the CLI in certain circumstances, caused by a bug in the version of the Bollard crate that was being used.

# 0.3.1 - 2020/10/09
## CLI
* Fixed an issue that would cause the CLI to fail to read kiln.toml if run in a docker container and the `--work-dir=path/to/directory` option was used.

# 0.3.0 - 2020/10/07
## CLI
* Added support for overriding the directory to scan with tools with the `--work-dir=path/to/directory` option. This defaults to the current directory if unspecified. Relative and absolute paths are supported.

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
