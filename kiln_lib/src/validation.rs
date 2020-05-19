use serde::Serialize;
use std::error::Error; 
use std::fmt; 

#[derive(Debug, PartialEq, Serialize)]
pub struct ValidationError {
    pub error_code: u8,
    pub error_message: String,
    pub json_field_name: Option<String>,
}

impl Error for ValidationError { } 

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "Error Validating data: (Err {}) {}", self.error_code, self.error_message)
    }
}

impl ValidationError {
    pub fn body_empty() -> ValidationError {
        ValidationError {
            error_code: 100,
            error_message: "Request body empty".into(),
            json_field_name: None,
        }
    }

    pub fn body_media_type_incorrect() -> ValidationError {
        ValidationError {
            error_code: 101,
            error_message: "Request body not correct media type".into(),
            json_field_name: None,
        }
    }

    pub fn application_name_empty() -> ValidationError {
        ValidationError {
            error_code: 111,
            error_message: "Application name present but empty".into(),
            json_field_name: Some("application_name".into()),
        }
    }

    pub fn application_name_missing() -> ValidationError {
        ValidationError {
            error_code: 102,
            error_message: "Application name required".into(),
            json_field_name: Some("application_name".into()),
        }
    }

    pub fn application_name_not_a_string() -> ValidationError {
        ValidationError {
            error_code: 112,
            error_message: "Application name not a valid string".into(),
            json_field_name: Some("application_name".into()),
        }
    }

    pub fn git_branch_name_empty() -> ValidationError {
        ValidationError {
            error_code: 113,
            error_message: "Git branch name present but empty".into(),
            json_field_name: Some("git_branch".into()),
        }
    }

    pub fn git_branch_name_not_a_string() -> ValidationError {
        ValidationError {
            error_code: 114,
            error_message: "Git branch name not a valid string".into(),
            json_field_name: Some("git_branch".into()),
        }
    }

    pub fn git_commit_hash_empty() -> ValidationError {
        ValidationError {
            error_code: 115,
            error_message: "Git commit hash present but empty".into(),
            json_field_name: Some("git_commit_hash".into()),
        }
    }

    pub fn git_commit_hash_missing() -> ValidationError {
        ValidationError {
            error_code: 104,
            error_message: "Git commit hash required".into(),
            json_field_name: Some("git_commit_hash".into()),
        }
    }

    pub fn git_commit_hash_not_a_string() -> ValidationError {
        ValidationError {
            error_code: 117,
            error_message: "Git commit hash not a valid string".into(),
            json_field_name: Some("git_commit_hash".into()),
        }
    }

    pub fn git_commit_hash_not_valid() -> ValidationError {
        ValidationError {
            error_code: 116,
            error_message: "Git commit hash not valid".into(),
            json_field_name: Some("git_commit_hash".into()),
        }
    }

    pub fn tool_name_empty() -> ValidationError {
        ValidationError {
            error_code: 118,
            error_message: "Tool name present but empty".into(),
            json_field_name: Some("tool_name".into()),
        }
    }

    pub fn tool_name_missing() -> ValidationError {
        ValidationError {
            error_code: 105,
            error_message: "Tool name required".into(),
            json_field_name: Some("tool_name".into()),
        }
    }

    pub fn tool_name_not_a_string() -> ValidationError {
        ValidationError {
            error_code: 119,
            error_message: "Tool name not a valid string".into(),
            json_field_name: Some("tool_name".into()),
        }
    }

    pub fn tool_output_empty() -> ValidationError {
        ValidationError {
            error_code: 120,
            error_message: "Tool output present but empty".into(),
            json_field_name: Some("tool_output".into()),
        }
    }

    pub fn tool_output_missing() -> ValidationError {
        ValidationError {
            error_code: 106,
            error_message: "Tool output required".into(),
            json_field_name: Some("tool_output".into()),
        }
    }

    pub fn tool_output_not_a_string() -> ValidationError {
        ValidationError {
            error_code: 121,
            error_message: "Tool output not a valid string".into(),
            json_field_name: Some("tool_output".into()),
        }
    }

    pub fn tool_output_format_empty() -> ValidationError {
        ValidationError {
            error_code: 122,
            error_message: "Tool output format present but empty".into(),
            json_field_name: Some("tool_output_format".into()),
        }
    }

    pub fn tool_output_format_missing() -> ValidationError {
        ValidationError {
            error_code: 107,
            error_message: "Tool output format required".into(),
            json_field_name: Some("tool_output_format".into()),
        }
    }

    pub fn tool_output_format_not_a_string() -> ValidationError {
        ValidationError {
            error_code: 123,
            error_message: "Tool output format not a valid string".into(),
            json_field_name: Some("tool_output_format".into()),
        }
    }

    pub fn tool_output_format_invalid() -> ValidationError {
        ValidationError {
            error_code: 124,
            error_message: "Tool output format not acceptable".into(),
            json_field_name: Some("tool_output_format".into()),
        }
    }

    pub fn start_time_missing() -> ValidationError {
        ValidationError {
            error_code: 108,
            error_message: "Start time required".into(),
            json_field_name: Some("start_time".into()),
        }
    }

    pub fn start_time_not_a_timestamp() -> ValidationError {
        ValidationError {
            error_code: 125,
            error_message: "Start time not a valid timestamp".into(),
            json_field_name: Some("start_time".into()),
        }
    }

    pub fn end_time_missing() -> ValidationError {
        ValidationError {
            error_code: 109,
            error_message: "End time required".into(),
            json_field_name: Some("end_time".into()),
        }
    }

    pub fn end_time_not_a_timestamp() -> ValidationError {
        ValidationError {
            error_code: 126,
            error_message: "End time not a valid timestamp".into(),
            json_field_name: Some("end_time".into()),
        }
    }

    pub fn environment_not_a_valid_option() -> ValidationError {
        ValidationError {
            error_code: 128,
            error_message: "Environment not a valid option".into(),
            json_field_name: Some("environment".into()),
        }
    }

    pub fn environment_missing() -> ValidationError {
        ValidationError {
            error_code: 110,
            error_message: "Environment required".into(),
            json_field_name: Some("environment".into()),
        }
    }

    pub fn environment_not_a_string() -> ValidationError {
        ValidationError {
            error_code: 127,
            error_message: "Environment not a valid string".into(),
            json_field_name: Some("environment".into()),
        }
    }

    pub fn environment_empty() -> ValidationError {
        ValidationError {
            error_code: 133,
            error_message: "Environment present but empty".into(),
            json_field_name: Some("environment".into()),
        }
    }

    pub fn tool_version_not_a_string() -> ValidationError {
        ValidationError {
            error_code: 130,
            error_message: "Tool version not a valid string".into(),
            json_field_name: Some("tool_version".into()),
        }
    }

    pub fn tool_version_present_but_empty() -> ValidationError {
        ValidationError {
            error_code: 129,
            error_message: "Tool version present but empty".into(),
            json_field_name: Some("tool_version".into()),
        }
    }

    pub fn avro_schema_validation_failed() -> ValidationError {
        ValidationError {
            error_code: 130,
            error_message: "Tried to deserialise a ToolReport from Avro but value didn't pass schema validation".into(),
            json_field_name: None, 
        }
    }

    pub fn tool_output_format_not_an_enum() -> ValidationError {
        ValidationError {
            error_code: 131,
            error_message: "Tool output format not an avro enum".into(),
            json_field_name: None, 
        }
    }

    pub fn environment_not_an_enum() -> ValidationError {
        ValidationError {
            error_code: 132,
            error_message: "Environment not an avro enum".into(),
            json_field_name: None, 
        }
    }

    pub fn event_version_missing() -> ValidationError {
        ValidationError {
            error_code: 133,
            error_message: "Event version missing".into(),
            json_field_name: Some("event_version".into()),
        }
    }

    pub fn event_version_not_a_string() -> ValidationError {
        ValidationError {
            error_code: 134,
            error_message: "Event version not a string".into(),
            json_field_name: Some("event_version".into()),
        }
    }

    pub fn event_version_present_but_empty() -> ValidationError {
        ValidationError {
            error_code: 135,
            error_message: "Event version present but empty".into(),
            json_field_name: Some("event_version".into()),
        }
    }

    pub fn event_version_unknown() -> ValidationError {
        ValidationError {
            error_code: 136,
            error_message: "Event version unknown".into(),
            json_field_name: Some("event_version".into()),
        }
    }

    pub fn event_id_missing() -> ValidationError {
        ValidationError {
            error_code: 137,
            error_message: "Event ID missing".into(),
            json_field_name: Some("event_version".into()),
        }
    }

    pub fn event_id_not_a_string() -> ValidationError {
        ValidationError {
            error_code: 138,
            error_message: "Event ID not a string".into(),
            json_field_name: Some("event_version".into()),
        }
    }

    pub fn event_id_present_but_empty() -> ValidationError {
        ValidationError {
            error_code: 138,
            error_message: "Event ID present but empty".into(),
            json_field_name: Some("event_version".into()),
        }
    }

    pub fn event_id_not_a_uuid() -> ValidationError {
        ValidationError {
            error_code: 139,
            error_message: "Event ID does not look like a UUID".into(),
            json_field_name: Some("event_version".into()),
        }
    }

    pub fn advisory_description_empty() -> ValidationError {
        ValidationError {
            error_code: 140,
            error_message: "Advisory description empty".into(),
            json_field_name: Some("advisory_description".into()),
        }
    }

    pub fn advisory_id_empty() -> ValidationError {
        ValidationError {
            error_code: 141,
            error_message: "Advisory Id empty".into(),
            json_field_name: Some("advisory_id".into()),
        }
    }

    pub fn affected_package_empty() -> ValidationError {
        ValidationError {
            error_code: 142,
            error_message: "Affected package empty".into(),
            json_field_name: Some("affected_package".into()),
        }
    }

    pub fn installed_version_empty() -> ValidationError {
        ValidationError {
            error_code: 143,
            error_message: "Installed version empty".into(),
            json_field_name: Some("installed_version".into()),
        }
    }

    pub fn timestamp_not_a_valid_timestamp() -> ValidationError {
        ValidationError {
            error_code: 144,
            error_message: "Timestamp does not look like a valid timestamp".into(),
            json_field_name: Some("timestamp".into()),
        }
    }

    pub fn timestamp_not_a_string() -> ValidationError {
        ValidationError {
            error_code: 145,
            error_message: "Timestamp does not look like a string".into(),
            json_field_name: Some("timestamp".into()),
        }
    }

    pub fn affected_package_not_a_string() -> ValidationError {
        ValidationError {
            error_code: 146,
            error_message: "Affected package not a string".into(),
            json_field_name: Some("affected_package".into()),
        }
    }

    pub fn installed_version_not_a_string() -> ValidationError {
        ValidationError {
            error_code: 147,
            error_message: "Installed version not a string".into(),
            json_field_name: Some("installed_version".into()),
        }
    }

    pub fn advisory_id_not_a_string() -> ValidationError {
        ValidationError {
            error_code: 148,
            error_message: "Advisory Id not a string".into(),
            json_field_name: Some("advisory_id".into()),
        }
    }

    pub fn advisory_url_empty() -> ValidationError {
        ValidationError {
            error_code: 149,
            error_message: "Advisory url empty".into(),
            json_field_name: Some("advisory_url".into()),
        }
    }

    pub fn advisory_url_not_valid() -> ValidationError {
        ValidationError {
            error_code: 150,
            error_message: "Advisory Url not a valid Url".into(),
            json_field_name: Some("advisory_url".into()),
        }
    }

    pub fn advisory_url_not_a_string() -> ValidationError {
        ValidationError {
            error_code: 151,
            error_message: "Advisory Url not a string".into(),
            json_field_name: Some("advisory_url".into()),
        }
    }

    pub fn advisory_description_not_a_string() -> ValidationError {
        ValidationError {
            error_code: 151,
            error_message: "Advisory description not a string".into(),
            json_field_name: Some("advisory_description".into()),
        }
    }

    pub fn cvss_version_not_a_string() -> ValidationError {
        ValidationError {
            error_code: 152,
            error_message: "CVSS Version field not parsable as a string".into(),
            json_field_name: None,
        }
    }

    pub fn cvss_score_not_valid() -> ValidationError {
        ValidationError {
            error_code: 153,
            error_message: "CVSS score not valid".into(),
            json_field_name: None,
        }
    }

    pub fn cvss_version_known_without_score() -> ValidationError {
        ValidationError {
            error_code: 154,
            error_message: "CVSS version is known but score not provided".into(),
            json_field_name: None,
        }
    }

    pub fn cvss_version_unknown_with_score() -> ValidationError {
        ValidationError {
            error_code: 155,
            error_message: "CVSS version is unknown but score was provided".into(),
            json_field_name: None,
        }
    }

    pub fn cvss_not_a_record() -> ValidationError {
        ValidationError {
            error_code: 156,
            error_message: "CVSS avro value not a record".into(),
            json_field_name: None,
        }
    }

    pub fn suppressed_issues_not_an_array() -> ValidationError {
        ValidationError {
            error_code: 157,
            error_message: "Suppressed Issues not an array of objects".into(),
            json_field_name: Some("suppressed_issues".to_owned()),
        }
    }

    pub fn issue_hash_not_valid() -> ValidationError {
        ValidationError {
            error_code: 158,
            error_message: "One or more issue hashes do not look like a valid SHA256 hashes".into(),
            json_field_name: Some("suppressed_issues[].issue_hash".to_owned()),
        }
    }

    pub fn expiry_date_not_a_valid_date() -> ValidationError {
        ValidationError {
            error_code: 159,
            error_message: "One or more issue suppression expiry dates do not look like valid dates".into(),
            json_field_name: Some("suppressed_issues[].expiry_date".to_owned()),
        }
    }

    pub fn suppression_reason_empty() -> ValidationError {
        ValidationError {
            error_code: 160,
            error_message: "One or more issue suppression reasons are empty".into(),
            json_field_name: Some("suppressed_issues[].suppression_reason".to_owned()),
        }
    }

    pub fn issue_hash_required() -> ValidationError {
        ValidationError {
            error_code: 161,
            error_message: "One or more issue hashes are missing".into(),
            json_field_name: Some("suppressed_issues[].issue_hash".to_owned()),
        }
    }

    pub fn issue_hash_not_a_string() -> ValidationError {
        ValidationError {
            error_code: 162,
            error_message: "One or more issue hashes are not strings".into(),
            json_field_name: Some("suppressed_issues[].issue_hash".to_owned()),
        }
    }

    pub fn suppression_reason_required() -> ValidationError {
        ValidationError {
            error_code: 163,
            error_message: "One or more issue suppression reasons are missing".into(),
            json_field_name: Some("suppressed_issues[].suppression_reason".to_owned()),
        }
    }

    pub fn suppression_reason_not_a_string() -> ValidationError {
        ValidationError {
            error_code: 164,
            error_message: "One or more issue suppression reasons are not strings".into(),
            json_field_name: Some("suppressed_issues[].suppression_reason".to_owned()),
        }
    }

    pub fn expiry_date_not_a_string() -> ValidationError {
        ValidationError {
            error_code: 165,
            error_message: "One or more issue suppression expiry dates are not strings".into(),
            json_field_name: Some("suppressed_issues[].expiry_date".to_owned()),
        }
    }

    pub fn suppressed_issue_not_a_record() -> ValidationError {
        ValidationError {
            error_code: 166,
            error_message: "Suppressed Issue Avro object is not a record".into(),
            json_field_name: None,
        }
    }

    pub fn suppressed_flag_not_a_boolean() -> ValidationError {
        ValidationError {
            error_code: 167,
            error_message: "Suppression flag Avro value is not a boolean".into(),
            json_field_name: None,
        }
    }

    pub fn suppressed_issue_toml_value_not_a_table() -> ValidationError {
        ValidationError {
            error_code: 168,
            error_message: "Suppressed Issue TOML type must be a Table".into(),
            json_field_name: None
        }
    }
}

#[cfg(feature = "web")]
use actix_web::HttpResponse;

#[cfg(feature = "web")]
impl Into<HttpResponse> for ValidationError {
    fn into(self) -> HttpResponse {
        HttpResponse::BadRequest()
            .json(self)
    }
}
