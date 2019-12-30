#[cfg(feature = "avro")]
use crate::avro_schema::DEPENDENCY_EVENT_SCHEMA;
#[cfg(feature = "avro")]
use failure::err_msg;

use crate::tool_report::{ApplicationName, EventID, EventVersion, GitBranch, GitCommitHash};
use crate::validation::ValidationError;

#[cfg(feature = "avro")]
use avro_rs::schema::Schema;

use std::convert::TryFrom;

use chrono::{DateTime, Utc};
use url::Url;

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq)]
pub struct DependencyEvent{
    pub event_version: EventVersion,
    pub event_id: EventID,
    pub parent_event_id: EventID,
    pub application_name: ApplicationName,
    pub git_branch: GitBranch,
    pub git_commit_hash: GitCommitHash,
    pub timestamp: Timestamp,
    pub affected_package: AffectedPackage,
    pub installed_version: InstalledVersion,
    pub advisory_id: AdvisoryId,
    pub advisory_url: AdvisoryUrl,
    pub advisory_description: AdvisoryDescription
}

#[cfg(feature = "avro")]
impl<'a> TryFrom<avro_rs::types::Value> for DependencyEvent {
    type Error = failure::Error;

    fn try_from(value: avro_rs::types::Value) -> Result<Self, Self::Error> {
        let schema = Schema::parse_str(DEPENDENCY_EVENT_SCHEMA).unwrap();
        let resolved_value = value.resolve(&schema).map_err(|err| {
            err_msg(format!(
                "Error resolving Avro schema: {}",
                err.name().unwrap()
            ))
        })?;

        if let avro_rs::types::Value::Record(record) = resolved_value {
            let mut fields = record.iter();
            let event_version =
                EventVersion::try_from(fields.find(|&x| x.0 == "event_version").unwrap().1.clone())
                    .map_err(|err| err_msg(err.error_message))?;
            let event_id =
                EventID::try_from(fields.find(|&x| x.0 == "event_id").unwrap().1.clone())
                    .map_err(|err| err_msg(err.error_message))?;
            let parent_event_id =
                EventID::try_from(fields.find(|&x| x.0 == "parent_event_id").unwrap().1.clone())
                    .map_err(|err| err_msg(err.error_message))?;
            let application_name = ApplicationName::try_from(
                fields
                    .find(|&x| x.0 == "application_name")
                    .unwrap()
                    .1
                    .clone(),
            )
            .map_err(|err| err_msg(err.error_message))?;
            let git_branch =
                GitBranch::try_from(fields.find(|&x| x.0 == "git_branch").unwrap().1.clone())
                    .map_err(|err| err_msg(err.error_message))?;
            let git_commit_hash = GitCommitHash::try_from(
                fields
                    .find(|&x| x.0 == "git_commit_hash")
                    .unwrap()
                    .1
                    .clone(),
            )
            .map_err(|err| err_msg(err.error_message))?;
            let timestamp = Timestamp::try_from(
                fields
                    .find(|&x| x.0 == "timestamp")
                    .unwrap()
                    .1
                    .clone(),
            )
            .map_err(|err| err_msg(err.error_message))?;
            let affected_package = AffectedPackage::try_from(
                fields
                    .find(|&x| x.0 == "affected_package")
                    .unwrap()
                    .1
                    .clone(),
            )
            .map_err(|err| err_msg(err.error_message))?;
            let installed_version = InstalledVersion::try_from(
                fields
                    .find(|&x| x.0 == "installed_version")
                    .unwrap()
                    .1
                    .clone(),
            )
            .map_err(|err| err_msg(err.error_message))?;
            let advisory_id = AdvisoryId::try_from(
                fields
                    .find(|&x| x.0 == "advisory_id")
                    .unwrap()
                    .1
                    .clone(),
            )
            .map_err(|err| err_msg(err.error_message))?;
            let advisory_url = AdvisoryUrl::try_from(
                fields
                    .find(|&x| x.0 == "advisory_url")
                    .unwrap()
                    .1
                    .clone(),
            )
            .map_err(|err| err_msg(err.error_message))?;
            let advisory_description = AdvisoryDescription::try_from(
                fields
                    .find(|&x| x.0 == "advisory_description")
                    .unwrap()
                    .1
                    .clone(),
            )
            .map_err(|err| err_msg(err.error_message))?;

            Ok(DependencyEvent {
                event_version,
                event_id,
                parent_event_id,
                application_name,
                git_branch,
                git_commit_hash,
                timestamp,
                affected_package,
                installed_version,
                advisory_id,
                advisory_url,
                advisory_description
            })
        } else {
            Err(err_msg("Something went wrong decoding Avro record"))
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Timestamp(DateTime<Utc>);

impl std::fmt::Display for Timestamp {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0.to_rfc3339())
    }
}

impl TryFrom<String> for Timestamp {
    type Error = ValidationError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        DateTime::parse_from_rfc3339(&value)
            .map(|dt| Timestamp(DateTime::<Utc>::from(dt)))
            .map_err(|_| ValidationError::timestamp_not_a_valid_timestamp())
    }
}

impl From<DateTime<Utc>> for Timestamp {
    fn from(dt: DateTime<Utc>) -> Self {
        Self(dt)
    }
}

#[cfg(feature = "avro")]
impl TryFrom<avro_rs::types::Value> for Timestamp {
    type Error = ValidationError;

    fn try_from(value: avro_rs::types::Value) -> Result<Self, Self::Error> {
        match value {
            avro_rs::types::Value::String(s) => Timestamp::try_from(s),
            _ => Err(ValidationError::timestamp_not_a_string()),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct AffectedPackage(String);

impl TryFrom<String> for AffectedPackage {
    type Error = ValidationError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.is_empty() {
            Err(ValidationError::affected_package_empty())
        } else {
            Ok(AffectedPackage(value))
        }
    }
}

#[cfg(feature = "avro")]
impl TryFrom<avro_rs::types::Value> for AffectedPackage {
    type Error = ValidationError;

    fn try_from(value: avro_rs::types::Value) -> Result<Self, Self::Error> {
        match value {
            avro_rs::types::Value::String(s) => AffectedPackage::try_from(s),
            _ => Err(ValidationError::affected_package_not_a_string()),
        }
    }
}


impl std::fmt::Display for AffectedPackage {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct InstalledVersion(String);

impl TryFrom<String> for InstalledVersion {
    type Error = ValidationError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.is_empty() {
            Err(ValidationError::installed_version_empty())
        } else {
            Ok(InstalledVersion(value))
        }
    }
}

#[cfg(feature = "avro")]
impl TryFrom<avro_rs::types::Value> for InstalledVersion {
    type Error = ValidationError;

    fn try_from(value: avro_rs::types::Value) -> Result<Self, Self::Error> {
        match value {
            avro_rs::types::Value::String(s) => InstalledVersion::try_from(s),
            _ => Err(ValidationError::installed_version_not_a_string()),
        }
    }
}


impl std::fmt::Display for InstalledVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct AdvisoryId(String);

impl TryFrom<String> for AdvisoryId {
    type Error = ValidationError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.is_empty() {
            Err(ValidationError::advisory_id_empty())
        } else {
            Ok(AdvisoryId(value))
        }
    }
}

#[cfg(feature = "avro")]
impl TryFrom<avro_rs::types::Value> for AdvisoryId {
    type Error = ValidationError;

    fn try_from(value: avro_rs::types::Value) -> Result<Self, Self::Error> {
        match value {
            avro_rs::types::Value::String(s) => AdvisoryId::try_from(s),
            _ => Err(ValidationError::advisory_id_not_a_string()),
        }
    }
}

impl std::fmt::Display for AdvisoryId{
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct AdvisoryUrl(Url);

impl TryFrom<String> for AdvisoryUrl {
    type Error = ValidationError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.is_empty() {
            Err(ValidationError::advisory_url_empty())
        } else if Url::parse(&value).is_err() {
            Err(ValidationError::advisory_url_not_valid())
        } else {
            Ok(AdvisoryUrl(Url::parse(&value).unwrap()))
        }
    }
}

#[cfg(feature = "avro")]
impl TryFrom<avro_rs::types::Value> for AdvisoryUrl {
    type Error = ValidationError;

    fn try_from(value: avro_rs::types::Value) -> Result<Self, Self::Error> {
        match value {
            avro_rs::types::Value::String(s) => AdvisoryUrl::try_from(s),
            _ => Err(ValidationError::advisory_url_not_a_string()),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct AdvisoryDescription(String);

impl TryFrom<String> for AdvisoryDescription {
    type Error = ValidationError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.is_empty() {
            Err(ValidationError::advisory_description_empty())
        } else {
            Ok(AdvisoryDescription(value))
        }
    }
}

#[cfg(feature = "avro")]
impl TryFrom<avro_rs::types::Value> for AdvisoryDescription {
    type Error = ValidationError;

    fn try_from(value: avro_rs::types::Value) -> Result<Self, Self::Error> {
        match value {
            avro_rs::types::Value::String(s) => AdvisoryDescription::try_from(s),
            _ => Err(ValidationError::advisory_description_not_a_string()),
        }
    }
}


impl std::fmt::Display for AdvisoryDescription {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
