#[cfg(feature = "avro")]
use crate::avro_schema::DEPENDENCY_EVENT_SCHEMA;
#[cfg(feature = "avro")]
use failure::err_msg;

use crate::tool_report::{ApplicationName, EventID, EventVersion, GitBranch, GitCommitHash};
use crate::traits::Hashable;
use crate::validation::ValidationError;

#[cfg(feature = "avro")]
use avro_rs::schema::Schema;

#[cfg(feature = "avro")]
use std::collections::HashMap;
use std::convert::TryFrom;

use chrono::{DateTime, Utc};
use ring::digest;
use serde::{Serialize, Serializer};
use url::Url;

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct DependencyEvent {
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
    pub advisory_description: AdvisoryDescription,
    pub cvss: Cvss,
    pub suppressed: bool,
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
            let parent_event_id = EventID::try_from(
                fields
                    .find(|&x| x.0 == "parent_event_id")
                    .unwrap()
                    .1
                    .clone(),
            )
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
            let timestamp =
                Timestamp::try_from(fields.find(|&x| x.0 == "timestamp").unwrap().1.clone())
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
            let advisory_id =
                AdvisoryId::try_from(fields.find(|&x| x.0 == "advisory_id").unwrap().1.clone())
                    .map_err(|err| err_msg(err.error_message))?;
            let advisory_url =
                AdvisoryUrl::try_from(fields.find(|&x| x.0 == "advisory_url").unwrap().1.clone())
                    .map_err(|err| err_msg(err.error_message))?;
            let advisory_description = AdvisoryDescription::try_from(
                fields
                    .find(|&x| x.0 == "advisory_description")
                    .unwrap()
                    .1
                    .clone(),
            )
            .map_err(|err| err_msg(err.error_message))?;
            let cvss = Cvss::try_from(fields.find(|&x| x.0 == "cvss").unwrap().1.clone())
                .map_err(|err| err_msg(err.error_message))?;
            let suppressed_avro = fields.find(|&x| x.0 == "suppressed").unwrap().1.clone();
            let suppressed = match suppressed_avro {
                avro_rs::types::Value::Boolean(b) => Ok(b),
                _ => Err(err_msg(ValidationError::suppressed_flag_not_a_boolean())),
            }?;

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
                advisory_description,
                cvss,
                suppressed,
            })
        } else {
            Err(err_msg("Something went wrong decoding Avro record"))
        }
    }
}

impl Hashable for DependencyEvent {
    fn hash(&self) -> Vec<u8> {
        let mut hash_ctx = digest::Context::new(&digest::SHA256);
        hash_ctx.update(&self.application_name.hash());
        hash_ctx.update(&self.affected_package.hash());
        hash_ctx.update(&self.installed_version.hash());
        hash_ctx.update(&self.advisory_id.hash());
        hash_ctx.finish().as_ref().to_vec()
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
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

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct AffectedPackage(String);

impl Hashable for AffectedPackage {
    fn hash(&self) -> Vec<u8> {
        digest::digest(&digest::SHA256, &self.0.as_bytes())
            .as_ref()
            .to_vec()
    }
}

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

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct InstalledVersion(String);

impl Hashable for InstalledVersion {
    fn hash(&self) -> Vec<u8> {
        digest::digest(&digest::SHA256, &self.0.as_bytes())
            .as_ref()
            .to_vec()
    }
}

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

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct AdvisoryId(String);

impl Hashable for AdvisoryId {
    fn hash(&self) -> Vec<u8> {
        digest::digest(&digest::SHA256, &self.0.as_bytes())
            .as_ref()
            .to_vec()
    }
}

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

impl std::fmt::Display for AdvisoryId {
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

impl std::fmt::Display for AdvisoryUrl {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0.as_str())
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

impl Serialize for AdvisoryUrl {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.0.as_str())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
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

#[derive(Clone, Debug, PartialEq, Serialize)]
pub enum CvssVersion {
    Unknown,
    V2,
    V3,
}

impl std::fmt::Display for CvssVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            CvssVersion::Unknown => write!(f, "Unknown"),
            CvssVersion::V2 => write!(f, "V2"),
            CvssVersion::V3 => write!(f, "V3"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(default)]
pub struct Cvss {
    version: CvssVersion,
    score: Option<f32>,
}

impl std::fmt::Display for Cvss {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self.version {
            CvssVersion::Unknown => write!(f, "Unknown"),
            _ => write!(
                f,
                "{}({})",
                self.version.to_string(),
                self.score.unwrap().to_string()
            ),
        }
    }
}

#[cfg(feature = "avro")]
impl TryFrom<avro_rs::types::Value> for Cvss {
    type Error = ValidationError;

    fn try_from(value: avro_rs::types::Value) -> Result<Self, Self::Error> {
        match value {
            avro_rs::types::Value::Record(fields) => {
                let fields = fields.into_iter().collect::<HashMap<_, _>>();

                let version = match fields.get("version").unwrap() {
                    avro_rs::types::Value::Enum(_, version) => match version.as_ref() {
                        "V2" => Ok(CvssVersion::V2),
                        "V3" => Ok(CvssVersion::V3),
                        _ => Ok(CvssVersion::Unknown),
                    },
                    _ => Err(ValidationError::cvss_version_not_a_string()),
                }?;

                let score = match fields.get("score").unwrap() {
                    avro_rs::types::Value::Null => Ok(None),
                    avro_rs::types::Value::Float(val) => Ok(Some(*val)),
                    _ => Err(ValidationError::cvss_score_not_valid()),
                }?;

                Cvss::builder()
                    .with_version(version)
                    .with_score(score)
                    .build()
            }
            _ => Err(ValidationError::cvss_not_a_record()),
        }
    }
}

pub struct CvssBuilder {
    version: CvssVersion,
    score: Option<f32>,
}

 impl Default for CvssBuilder {
     fn default() -> Self {
        Self::new()
    }
}

impl Cvss {
    pub fn builder() -> CvssBuilder {
        CvssBuilder {
            version: CvssVersion::Unknown,
            score: None,
        }
    }
}

impl CvssBuilder {
    pub fn new() -> CvssBuilder {
        CvssBuilder {
            version: CvssVersion::Unknown,
            score: None,
        }
    }

    pub fn with_version(mut self, version: CvssVersion) -> Self {
        self.version = version;
        self
    }

    pub fn with_score(mut self, score: Option<f32>) -> Self {
        self.score = score;
        self
    }

    pub fn build(self) -> Result<Cvss, ValidationError> {
        if self.version == CvssVersion::Unknown && self.score.is_some() {
            Err(ValidationError::cvss_version_unknown_with_score())
        } else if self.version != CvssVersion::Unknown && self.score.is_none() {
            Err(ValidationError::cvss_version_known_without_score())
        } else {
            Ok(Cvss {
                version: self.version,
                score: self.score,
            })
        }
    }
}

#[cfg(test)]
#[cfg(feature = "all")]
pub mod tests {
    use super::*;
    use avro_rs::{Reader, Schema, Writer};

    #[test]
    fn timestamp_try_from_string_returns_error_when_timestamp_not_valid() {
        let expected = ValidationError::timestamp_not_a_valid_timestamp();
        let actual =
            Timestamp::try_from("not a timestamp".to_string()).expect_err("Expected Err(_) value");

        assert_eq!(expected, actual)
    }

    #[test]
    fn affected_package_try_from_string_returns_error_when_value_empty() {
        let expected = ValidationError::affected_package_empty();
        let actual = AffectedPackage::try_from("".to_string()).expect_err("Expected Err(_) value");

        assert_eq!(expected, actual)
    }

    #[test]
    fn installed_version_try_from_string_returns_error_when_value_empty() {
        let expected = ValidationError::installed_version_empty();
        let actual = InstalledVersion::try_from("".to_string()).expect_err("Expected Err(_) value");

        assert_eq!(expected, actual)
    }

    #[test]
    fn advisory_id_try_from_string_returns_error_when_value_empty() {
        let expected = ValidationError::advisory_id_empty();
        let actual = AdvisoryId::try_from("".to_string()).expect_err("Expected Err(_) value");

        assert_eq!(expected, actual)
    }

    #[test]
    fn advisory_url_try_from_string_returns_error_when_value_empty() {
        let expected = ValidationError::advisory_url_empty();
        let actual = AdvisoryUrl::try_from("".to_string()).expect_err("Expected Err(_) value");

        assert_eq!(expected, actual)
    }

    #[test]
    fn advisory_url_try_from_string_returns_error_when_value_not_valid() {
        let expected = ValidationError::advisory_url_not_valid();
        let actual =
            AdvisoryUrl::try_from("not a url".to_string()).expect_err("Expected Err(_) value");

        assert_eq!(expected, actual)
    }

    #[test]
    fn advisory_description_try_from_string_returns_error_when_value_empty() {
        let expected = ValidationError::advisory_description_empty();
        let actual =
            AdvisoryDescription::try_from("".to_string()).expect_err("Expected Err(_) value");

        assert_eq!(expected, actual)
    }

    #[test]
    fn hash_of_dependency_event_is_correct() {
        let event = DependencyEvent {
            event_version: EventVersion::try_from("1".to_string()).unwrap(),
            event_id: EventID::try_from("383bc5f5-d099-40a4-a1a9-8c8a97559479".to_string()).unwrap(),
            parent_event_id: EventID::try_from("383bc5f5-d099-40a4-a1a9-8c8a97559479".to_string()).unwrap(),
            application_name: ApplicationName::try_from("Test application".to_string()).unwrap(),
            git_branch: GitBranch::try_from(Some("master".to_string())).unwrap(),
            git_commit_hash: GitCommitHash::try_from("e99f715d0fe787cd43de967b8a79b56960fed3e5".to_string()).unwrap(),
            timestamp: Timestamp::try_from("2019-09-13T19:37:14+00:00".to_string()).unwrap(),
            affected_package: AffectedPackage::try_from("BadPkg".to_string()).unwrap(),
            installed_version: InstalledVersion::try_from("1.0".to_string()).unwrap(),
            advisory_id: AdvisoryId::try_from("CVE-2017-5638".to_string()).unwrap(),
            advisory_url: AdvisoryUrl::try_from("https://nvd.nist.gov/vuln/detail/CVE-2017-5638".to_string()).unwrap(),
            advisory_description: AdvisoryDescription::try_from("The Jakarta Multipart parser in Apache Struts 2 2.3.x before 2.3.32 and 2.5.x before 2.5.10.1 has incorrect exception handling and error-message generation during file-upload attempts, which allows remote attackers to execute arbitrary commands via a crafted Content-Type, Content-Disposition, or Content-Length HTTP header, as exploited in the wild in March 2017 with a Content-Type header containing a #cmd= string.".to_string()).unwrap(),
            cvss: Cvss::builder()
                .with_version(CvssVersion::V3)
                .with_score(Some(10.0f32))
                .build()
                .unwrap(),
            suppressed: false
        };

        let mut hash_ctx = digest::Context::new(&digest::SHA256);
        hash_ctx.update(digest::digest(&digest::SHA256, b"Test application").as_ref());
        hash_ctx.update(digest::digest(&digest::SHA256, b"BadPkg").as_ref());
        hash_ctx.update(digest::digest(&digest::SHA256, b"1.0").as_ref());
        hash_ctx.update(digest::digest(&digest::SHA256, b"CVE-2017-5638").as_ref());
        let expected_hash = hash_ctx.finish();
        let actual_hash = event.hash();
        assert_eq!(expected_hash.as_ref(), actual_hash.as_slice());
    }

    #[test]
    fn dependency_event_can_round_trip_to_avro_and_back() {
        let event = DependencyEvent {
            event_version: EventVersion::try_from("1".to_string()).unwrap(),
            event_id: EventID::try_from("383bc5f5-d099-40a4-a1a9-8c8a97559479".to_string()).unwrap(),
            parent_event_id: EventID::try_from("383bc5f5-d099-40a4-a1a9-8c8a97559479".to_string()).unwrap(),
            application_name: ApplicationName::try_from("Test application".to_string()).unwrap(),
            git_branch: GitBranch::try_from(Some("master".to_string())).unwrap(),
            git_commit_hash: GitCommitHash::try_from("e99f715d0fe787cd43de967b8a79b56960fed3e5".to_string()).unwrap(),
            timestamp: Timestamp::try_from("2019-09-13T19:37:14+00:00".to_string()).unwrap(),
            affected_package: AffectedPackage::try_from("BadPkg".to_string()).unwrap(),
            installed_version: InstalledVersion::try_from("1.0".to_string()).unwrap(),
            advisory_id: AdvisoryId::try_from("CVE-2017-5638".to_string()).unwrap(),
            advisory_url: AdvisoryUrl::try_from("https://nvd.nist.gov/vuln/detail/CVE-2017-5638".to_string()).unwrap(),
            advisory_description: AdvisoryDescription::try_from("The Jakarta Multipart parser in Apache Struts 2 2.3.x before 2.3.32 and 2.5.x before 2.5.10.1 has incorrect exception handling and error-message generation during file-upload attempts, which allows remote attackers to execute arbitrary commands via a crafted Content-Type, Content-Disposition, or Content-Length HTTP header, as exploited in the wild in March 2017 with a Content-Type header containing a #cmd= string.".to_string()).unwrap(),
            cvss: Cvss::builder()
                .with_version(CvssVersion::V3)
                .with_score(Some(10.0f32))
                .build()
                .unwrap(),
            suppressed: false
        };

        let schema = Schema::parse_str(DEPENDENCY_EVENT_SCHEMA).unwrap();
        let mut writer = Writer::new(&schema, Vec::new());
        writer.append_ser(event.clone()).unwrap();
        writer.flush().unwrap();

        let input = writer.into_inner();
        let reader = Reader::with_schema(&schema, &input[..]).unwrap();
        let mut input_records = reader.into_iter().collect::<Vec<_>>();
        let parsed_record = input_records.remove(0).unwrap();

        let actual = DependencyEvent::try_from(parsed_record).expect("expected Ok(_) value");

        assert_eq!(event, actual);
    }
}
