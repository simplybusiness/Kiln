# Kiln
![](https://github.com/simplybusiness/kiln/workflows/CI/badge.svg)

Kiln is a collection of dockerised application security tools, with some special sauce to collect the output and send it to an Apache Kafka cluster. This data can then be analysed and used to perform Slack notification, raise items on a team's backlog, or determine trends in security findings, among other things.

## Tools supported
- Ruby
    - [ ] Bundler Audit
    - [ ] Rubocop
    - [ ] Brakeman
- Python
    - [ ] Safety
    - [ ] Bandit
- Javascript/Typescript
    - [ ] Yarn Audit
    - [ ] NPM Audit
    - [ ] ESLint Security Linting
    - [ ] ESLint Typescript Security Linting
- Java/Scala
    - [ ] Scala Build Tool Dependency Check
    - [ ] Gradle Dependency Check
- Golang
    - [ ] Gosec
- Other
    - [ ] Trufflehog
    - [ ] Graudit

## Integrations
- [ ] Slack
- [ ] Trello

## Architecture
Kiln is architected as a module, event sourcing system with only two required components: the data collector and an Apache Kafka cluster. Tool output is send to the data-collector from the docker container running the tool, which inserts the tool output and some additional metadata into a Kafka topic.

All integrations are Kafka consumers that process the events in the tool output topic and respond accordingly. For example, a Slack notification integration can consume events as they're added to the topic, compare the application name to a list of applications it knows about and send a message to the appropriate Slack channel with new security findings.

Tools (data producers) are wrapped in a docker container, which includes a small binary that takes the output from the tool and sends it to the data-collector endpoint to be recorded.

![Kiln architecture diagram](docs/images/Kiln Architecture diagram.png)

## Contributing

