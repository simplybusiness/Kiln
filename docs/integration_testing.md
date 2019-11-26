# Starting a Kiln stack locally
Currently, there is no automated integration testing of Kiln, but it is possible to quickly bring up a local Kiln stack to manually test changes with a Kafka cluster. Assuming you want to test a new tool image against the rest of Kiln, you will need the following components:

* Data forwarder compiled for musl-libc (to include in tool image)
* Tool image of choice (using bundler-audit as an example)
* Data collector docker image built
* Java JRE (required for building Java Key Store for Kafka)

Parts of this process have been automated using Cargo-make, which is the task runner for the Kiln build process.

## Building the Data-forwarder binary for musl-libc
There is a Cargo-make target for building this component for musl-libc which will also run linting and unit tests. `cd` to the project root directory and run `cargo make build-data-forwarder`.

## Building the tool docker image
This step assumes you have already built the Data-forwarder using the previously mentioned Cargo-make target. The Bundler-audit `Makefile.toml` includes a set of tasks that can be used as a starting point for building new tool images. `cd` to the directory containing your tool image, then run:
```
cargo make pre-build-bundler-audit-docker
cargo make build-bundler-audit-docker
```

The first task will ensure that the Data-forwarder binary is available for the Docker image build to copy it into the image, while the second one will actually build the image.

## Building the Data-collector docker image
To build the data-collector Docker image, run `cargo make build-docker-images` from the project root. This target will be kept updated with all of the docker image targets. It also includes linting and unit testing of the code being built. If you want a faster iteration cycle, you can cd into the directory of the component you're working on and run `cargo make musl-build && cargo make build-image`. These commands make use of build caching, so while the first build will be slow as dependencies need to be built for the `musl` target, subsequent builds should be much faster. Running that command will result in a tagged docker image: kiln/data-collector:latest.

## Generating TLS certificates
Kiln expects to connect to a Kafka cluster over TLS only, so in order to run a stack locally, we need to setup a basic PKI to ensure certificates can be validated and the connection will be successful. This process has been scripted, but still requires a small amount of user interaction.

To start the process, run the `gen_certs.sh` script in the root of the project. When prompted if you trust each certificate, enter 'yes'. This is the Java Keytool building the keystore and truststore required to connect to Kafka. This step should result in a new directory called 'tls' containing a signed CA certificate and 2 Java Keystore files containing the CA certificate and the Kafka broker certificate.

## Starting the stack
Once you've built the required docker images and generated the PKI using the `gen_certs.sh` script, you can bring up a Kiln stack. This requires two terminals, because Kiln expects that the Kafka cluster is ready to accept incoming connections when it starts.

In the first terminal, run `docker-compose up zookeeper kafka`. Once this has finished starting up and you see a message in the console output about the ToolReports topic being created, you can bring up the data-collector by running `docker-compose up data-collector` in the second terminal. You initially won't see any output from the data-collector, but you can check it's working by sending an HTTP request to it and checking that a log line is printed.

## Connecting the console consumer
To check the full flow of messages being consumed from the Kafka ToolReports topic, the easiest way is to start a Kafka console consumer. To do this, you need to `docker exec` into the Kafka broker container and from there you can start the console consumer.

Run `docker exec -it kiln_kafka_1 bash` to get a shell within the running Kafka container.

Then to start the console consumer, run `$KAFKA_HOME/bin/kafka-console-consumer.sh --bootstrap-server kafka:9092 --topic ToolReports --consumer.config /tls/client-ssl.properties --from-beginning`. Now if you send a valid HTTP request to the data-collector, you should see a serialised Avro message printed in this terminal.

## Running a tool image against a local Kiln stack
Using the Bundler-audit tool image as an example, this command will start the tool, mounting the current working directory for analysis and report it to a locally running Kiln stack: `docker run -it -v "${PWD}:/code" --net host -e SCAN_ENV="Local" -e APP_NAME="Railsgoat" -e DATA_COLLECTOR_URL="http://localhost:8081" kiln/bundler-audit:0.6.1`

A good codebase to test this particular example on is [OWASP RailsGoat](https://github.com/OWASP/railsgoat).

## Sending an example request
You should rarely need to use these instructions. They are preserved in case changes are made to the Data-collector or downstream components and you need to test a specific failure case that isn't easy to replicate some other way.

Below are a valid JSON payload for a request to the data-collector and an example cURL command to send this payload:

```
{
    "application_name": "Test application",
    "git_branch": "master",
    "git_commit_hash": "e99f715d0fe787cd43de967b8a79b56960fed3e5",
    "tool_name": "example tool",
    "tool_output": "{}",
    "output_format": "JSON",
    "start_time": "2019-09-13T19:35:38+00:00",
    "end_time": "2019-09-13T19:37:14+00:00",
    "environment": "Local",
	"tool_version": "1.0"
}
```

```
curl -X POST \
  http://127.0.0.1:8081 \
  -H 'Accept: */*' \
  -H 'Accept-Encoding: gzip, deflate' \
  -H 'Cache-Control: no-cache' \
  -H 'Connection: keep-alive' \
  -H 'Content-Length: 372' \
  -H 'Content-Type: application/json' \
  -H 'Host: 127.0.0.1:8081' \
  -H 'cache-control: no-cache' \
  -d '{
    "application_name": "Test application",
    "git_branch": "master",
    "git_commit_hash": "e99f715d0fe787cd43de967b8a79b56960fed3e5",
    "tool_name": "example tool",
    "tool_output": "{}",
    "output_format": "JSON",
    "start_time": "2019-09-13T19:35:38+00:00",
    "end_time": "2019-09-13T19:37:14+00:00",
    "environment": "Local",
    "tool_version": "1.0"
}'
```
