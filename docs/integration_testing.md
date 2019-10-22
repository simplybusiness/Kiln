# Starting a Kiln stack locally
Currently, there is no automated integration testing of Kiln, but it is possible to quickly bring up a local Kiln stack to manually test changes with a Kafka cluster.

## Building the Data-collector docker image
Cargo-make is used as a task runner for automating the Kiln build process. To build the data-collector Docker image, run `cargo make build-docker-images` from the project root. This target will be kept updated with all of the docker image targets. It also includes linting and unit testing of the code being built. If you want a faster iteration cycle, you can cd into the directory of the component you're working on and run `cargo make musl-build && cargo make build-image`. These commands make use of build caching, so while the first build will be slow as dependencies need to be built for the `musl` target, subsequent builds should be much faster. Running that command will result in a tagged docker image: kiln/data-collector:latest.

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

## Sending an example request
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
