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

pub const DEPENDENCY_EVENT_SCHEMA: &str = r#"
    {
        "type": "record",
        "name": "DependencyEvent",
        "fields": [
            {"name": "event_version", "type": "string"},
            {"name": "event_id", "type": "string"},
            {"name": "parent_event_id", "type": "string"},
            {"name": "application_name", "type": "string"},
            {"name": "git_branch", "type": ["null", "string"]},
            {"name": "git_commit_hash", "type": "string"},
            {"name": "timestamp", "type": "string"},
            {"name": "affected_package", "type": "string"},
            {"name": "installed_version", "type": "string"},
            {"name": "advisory_id", "type": "string"},
            {"name": "advisory_url", "type": "string"},
            {"name": "advisory_description", "type": "string"},
            {"name": "cvss", "type": {
                "name": "Cvss", "type": "record", "fields": [
                    {"name": "version", "type": {"type": "enum", "name": "CvssVersion", "symbols": ["Unknown", "V2", "V3"]}},
                    {"name": "score", "type": ["null", "float"]}]
                }
            }
        ]
    }
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use avro_rs::Schema;

    #[test]
    fn tool_report_schema_is_valid() {
        Schema::parse_str(TOOL_REPORT_SCHEMA).expect("expected Ok(_) value");
    }

    #[test]
    fn dependency_event_schema_is_valid() {
        Schema::parse_str(DEPENDENCY_EVENT_SCHEMA).expect("expected Ok(_) value");
    }
}
