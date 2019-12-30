pub const TOOL_REPORT_SCHEMA: &str = r#"
    {
        "type": "record",
        "name": "ToolReport",
        "fields": [
            {"name": "event_version", "type": "string"},
            {"name": "event_id", "type": "string"},
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
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use avro_rs::Schema;

    #[test]
    fn schema_is_valid() {
        Schema::parse_str(TOOL_REPORT_SCHEMA).expect("expected Ok(_) value");
    }
}
