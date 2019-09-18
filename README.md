# Kiln
![](https://github.com/simplybusiness/kiln/workflows/CI/badge.svg)
[![Contributor Covenant](https://img.shields.io/badge/Contributor%20Covenant-v1.4%20adopted-ff69b4.svg)](CODE_OF_CONDUCT.md)


Kiln is a collection of dockerised application security tools, with some special sauce to collect the output and send it to an Apache Kafka cluster. This data can then be analysed and used to perform Slack notification, raise items on a team's backlog, or determine trends in security findings, among other things.

## Architecture
Kiln is architected as a modular, event sourcing system with only two required components: the Kiln Data Collector and an Apache Kafka cluster. When you run a Kiln Security Sncanner, the tool output is send to the data-collector, which acts as a data validation point and HTTP interface to the Apache Kafka cluster. The data-collector then inserts the tool output and some additional metadata into a Kafka topic. For an introduction to Event Sourcing, checkout https://dev.to/barryosull/event-sourcing-what-it-is-and-why-its-awesome.

All Kiln Connectors are Kafka consumers that process the events in the tool output topic and respond accordingly. For example, the Slack connector can consume events as they're added to the topic, compare the application name to a list of applications it knows about and send a message to the appropriate Slack channel with new security findings.

Kiln Security Scanners are docker containers with security tools baked into the image and also include a small binary that takes the output from the tool and sends it to the Kiln Data Collector to be recorded.

![Kiln architecture diagram](https://github.com/simplybusiness/Kiln/blob/7cafc19b16ca1c13f4e187e6309b2efc16eed7bc/docs/images/Kiln%20Architecture%20diagram.png)

## Contributing
Please note that this project is released with a Contributor Code of Conduct. By participating in this project, you agree to abide by its terms. The Code of Conduct can be found [here](CODE_OF_CONDUCT.md).

To contribute to Kiln, you'll need the following tools installed:
- Serverless framework
- Yarn (for Serverless framework dependencies
- Rust (stable channel, assuming 1.37 as minimum)
- Clippy (For linting)
- Docker

Kiln is still in it's early stages and isn't ready for production use. However, contributions are welcome! If you want to make a change to the project:
- Open an issue to discuss the change (if the change is significant)
- Fork this repo
- Create a new branch in your fork
- Make your change
- Add new tests & ensure existing tests pass
- Ensure linting passes
- Open a PR and explain what changes you have made
- Wait for CI to pass and PR to be reviewed
- Merge!
