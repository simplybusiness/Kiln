# Kiln-lib

## Avro Schema
In Kiln, messages are serialised to the [Apache Avro format](https://avro.apache.org/docs/current/) before being sent to an Apache Kafka topic to be recorded. Below is the schema used to encode messages in Avro. Note that every field is recorded as a string. This is intentional, because all data validation should happen by building an instance of the ToolReport struct, either from JSON values in the case of the data-collector, or from string using the TryFrom<String> implementations for fields in the structy directly for other components. This scheme is validated using a test that parses the schema, so any invalid changes should be caught by CI.

```
{
    "type": "record",
    "name": "ToolReport",
    "fields": [
        {"name": "application_name", "type": "string"},
        {"name": "git_branch", "type": ["null", "string"]},
        {"name": "git_commit_hash", "type": "string"},
        {"name": "tool_name", "type": "string"},
        {"name": "tool_output", "type": "string"},
        {"name": "output_format", "type": {"type": "enum", "name": "OutputFormat", "symbols": ["JSON", "PlainText"]}},
        {"name": "start_time", "type": "string"},
        {"name": "end_time", "type": "string"},
        {"name": "environment", "type": {"type": "enum", "name": "Environment", "symbols": ["Local", "CI"]}},
        {"name": "tool_version", "type": ["null", "string"]}
    ]
}
```
