#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate slog;

use avro_rs::{Reader, Schema, Writer};
use chrono::prelude::*;
use chrono::Duration;
use chrono::{SecondsFormat, Utc};
use data_encoding::HEXUPPER;
use failure::err_msg;
use flate2::read::GzDecoder;
use futures_util::stream::StreamExt;
use iter_read::IterRead;
use kiln_lib::avro_schema::DEPENDENCY_EVENT_SCHEMA;
use kiln_lib::dependency_event::{
    AdvisoryDescription, AdvisoryId, AdvisoryUrl, AffectedPackage, Cvss, CvssVersion,
    DependencyEvent, InstalledVersion, Timestamp,
};
use kiln_lib::kafka::*;
use kiln_lib::tool_report::{EventID, EventVersion, IssueHash, SuppressedIssue, ToolReport};
use kiln_lib::traits::Hashable;
use rdkafka::consumer::{CommitMode, Consumer};
use rdkafka::message::Message;
use rdkafka::producer::future_producer::FutureRecord;
use regex::Regex;
use reqwest::blocking::Client;
use ring::digest;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::boxed::Box;
use std::collections::HashMap;
use std::convert::TryFrom;
use std::env;
use std::error::Error;
use std::io::Read;
use std::str::FromStr;
use url::Url;

use slog::o;
use slog::Drain;
use slog::{FnValue, PushFnValue};
use slog_derive::SerdeValue;
use slog_json::Json;
use uuid::Uuid;

const SERVICE_NAME: &str = "report-parser";

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let drain = Json::new(std::io::stdout()).build().fuse();

    let drain = slog_async::Async::new(drain).build().fuse();

    let root_logger = slog::Logger::root(
        drain,
        o!(
            "@timestamp" => PushFnValue(move |_, ser| {
                ser.emit(Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true))
            }),
            "log.level" => FnValue(move |rinfo| {
                rinfo.level().as_str()
            }),
            "message" => PushFnValue(move |record, ser| {
                ser.emit(record.msg())
            }),
            "ecs.version" => "1.5",
            "service.version" => env!("CARGO_PKG_VERSION"),
            "service.name" => SERVICE_NAME,
            "service.type" => "kiln",
            "event.kind" => "event",
            "event.category" => "web",
            "event.id" => FnValue(move |_| {
                Uuid::new_v4().to_hyphenated().to_string()
            }),
        ),
    );

    let config = get_bootstrap_config(&mut env::vars()).map_err(|err| {
        error!(root_logger, "Error building Kafka configuration";
            o!(
                "error.message" => err.to_string(),
                "event.type" => EventType(vec!("error".to_string())),
            )
        );
        err
    })?;

    let consumer =
        build_kafka_consumer(config.clone(), "report-parser".to_string()).map_err(|err| {
            error!(root_logger, "Error building Kafka consumer";
                o!(
                    "error.message" => err.to_string(),
                    "event.type" => EventType(vec!("error".to_string())),
                )
            );
            err
        })?;

    consumer.subscribe(&["ToolReports"]).map_err(|err| {
        error!(root_logger, "Error subscribing to ToolReports Kafka topic";
            o!(
                "error.message" => err.to_string(),
                "event.type" => EventType(vec!("error".to_string())),
            )
        );
        err
    })?;

    let producer = build_kafka_producer(config.clone()).map_err(|err| {
        error!(root_logger, "Error building Kafka producer";
            o!(
                "error.message" => err.to_string(),
                "event.type" => EventType(vec!("error".to_string())),
            )
        );
        err
    })?;

    let base_url = Url::parse(
        &env::var("NVD_BASE_URL")
            .unwrap_or_else(|_| "https://nvd.nist.gov/feeds/json/cve/1.1/".to_string()),
    )?;
    let mut last_updated_time = None;
    let client_builder = Client::builder();
    let client = client_builder
        .timeout(Some(std::time::Duration::new(10, 0)))
        .build()?;

    let mut vulns = HashMap::new();
    for year in 2002..=2020 {
        download_and_parse_vulns(year.to_string(), last_updated_time, &base_url, &client)
            .map_err(|err| {
                error!(root_logger, "Error downloading vulns for {}", year;
                    o!(
                        "error.message" => err.to_string(),
                        "event.type" => EventType(vec!("error".to_string())),
                    )
                );
                err
            })
            .and_then(|parsed_vulns| {
                parsed_vulns
                    .into_iter()
                    .fold(&mut vulns, |acc, mut values| {
                        for (k, v) in values.drain() {
                            acc.insert(k, v);
                        }
                        acc
                    });
                info!(root_logger, "Successfully got vulns for {}", year;
                    o!(
                        "event.type" => EventType(vec!("info".to_string())),
                    )
                );
                Ok(())
            })?;
    }

    download_and_parse_vulns(
        "modified".to_string(),
        last_updated_time,
        &base_url,
        &client,
    )
    .map_err(|err| {
        error!(root_logger, "Error downloading modified vulns info";
            o!(
                "error.message" => err.to_string(),
                "event.type" => EventType(vec!("error".to_string())),
            )
        );
        err
    })
    .and_then(|modified_vulns| {
        modified_vulns
            .into_iter()
            .fold(&mut vulns, |acc, mut values| {
                for (k, v) in values.drain() {
                    acc.insert(k, v);
                }
                acc
            });
        info!(root_logger, "Successfully got latest vulns";
            o!(
                "event.type" => EventType(vec!("info".to_string())),
            )
        );
        Ok(())
    })?;

    last_updated_time = Some(Utc::now());

    let mut messages = consumer.start_with(std::time::Duration::from_secs(1), false);

    loop {
        if last_updated_time
            .unwrap()
            .lt(&(Utc::now() - Duration::hours(2)))
        {
            download_and_parse_vulns(
                "modified".to_string(),
                last_updated_time,
                &base_url,
                &client,
            )
            .map_err(|err| {
                error!(root_logger, "Error updating vuln info";
                    o!(
                        "error.message" => err.to_string(),
                        "event.type" => EventType(vec!("error".to_string())),
                    )
                );
                err
            })
            .and_then(|modified_vulns| {
                if let Some(mut modified_vulns) = modified_vulns {
                    for (k, v) in modified_vulns.drain() {
                        vulns.insert(k, v);
                    }
                }
                last_updated_time = Some(Utc::now());
                info!(root_logger, "Successfully got latest vulns";
                    o!(
                        "event.type" => EventType(vec!("info".to_string())),
                    )
                );
                Ok(())
            })?;
        }

        if let Some(Ok(message)) = messages.next().await {
            if let Some(body) = message.payload() {
                let reader = Reader::new(body)?;
                for value in reader {
                    let report = ToolReport::try_from(value?)?;
                    let app_name = report.application_name.to_string();
                    let records = parse_tool_report(&report, &vulns)?;
                    for record in records.into_iter() {
                        let kafka_payload = FutureRecord::to("DependencyEvents")
                            .payload(&record)
                            .key(&app_name);
                        producer.send(kafka_payload, 5000).await?.map_err(|err| {
                            err_msg(format!("Error publishing to Kafka: {}", err.0.to_string()))
                        })?;
                    }
                }
            }
            consumer.commit_message(&message, CommitMode::Async)?;
        }
    }
}

fn download_and_parse_vulns(
    index: String,
    last_updated_time: Option<DateTime<Utc>>,
    base_url: &Url,
    client: &Client,
) -> Result<Option<HashMap<String, Cvss>>, Box<dyn Error>> {
    lazy_static! {
        static ref META_LAST_MOD_RE: Regex = Regex::new("lastModifiedDate:(.*)\r\n").unwrap();
        static ref META_COMPRESSED_GZ_SIZE_RE: Regex = Regex::new("gzSize:(.*)\r\n").unwrap();
        static ref META_UNCOMPRESSED_SIZE_RE: Regex = Regex::new("size:(.*)\r\n").unwrap();
        static ref META_SHA256_RE: Regex = Regex::new("sha256:(.*)\r\n").unwrap();
    }

    let meta_filename = format!("nvdcve-1.1-{}.meta", index);
    let meta_url = base_url.join(&meta_filename)?;

    let meta_resp_text = client
        .get(meta_url)
        .send()
        .map_err(|err| {
            Box::new(err_msg(format!("Error downloading {}: {}", meta_filename, err)).compat())
        })
        .and_then(|resp| {
            resp.text().map_err(|err| {
                Box::new(
                    err_msg(format!("Error reading body of {}: {}", meta_filename, err)).compat(),
                )
            })
        })?;

    let last_mod_timestamp = META_LAST_MOD_RE
        .captures(&meta_resp_text)
        .and_then(|captures| captures.get(1))
        .ok_or_else(|| {
            Box::new(
                err_msg(format!(
                    "Error reading lastModifiedDate from {}",
                    meta_filename
                ))
                .compat(),
            )
        })
        .map(|capture| capture.as_str())?;

    let uncompressed_size = META_UNCOMPRESSED_SIZE_RE
        .captures(&meta_resp_text)
        .and_then(|captures| captures.get(1))
        .ok_or_else(|| {
            Box::new(err_msg(format!("Error reading size from {}", meta_filename)).compat())
        })
        .map(|capture| capture.as_str())?;

    let compressed_size = META_COMPRESSED_GZ_SIZE_RE
        .captures(&meta_resp_text)
        .and_then(|captures| captures.get(1))
        .ok_or_else(|| {
            Box::new(
                err_msg(format!(
                    "Error reading compressed size from {}",
                    meta_filename
                ))
                .compat(),
            )
        })
        .map(|capture| capture.as_str())?;

    let hash = META_SHA256_RE
        .captures(&meta_resp_text)
        .and_then(|captures| captures.get(1))
        .ok_or_else(|| {
            Box::new(err_msg(format!("Error reading sha256 hash from {}", meta_filename)).compat())
        })
        .map(|capture| capture.as_str())?;

    if last_updated_time.is_none()
        || last_updated_time
            .unwrap()
            .lt(&DateTime::parse_from_rfc3339(last_mod_timestamp)?.with_timezone(&Utc))
    {
        let data_filename = format!("nvdcve-1.1-{}.json.gz", index);
        let data_url = base_url.join(&data_filename)?;

        let mut resp = client.get(data_url).send().map_err(|err| {
            Box::new(err_msg(format!("Error downloading {}: {}", data_filename, err)).compat())
        })?;

        let mut resp_compressed_bytes = Vec::<u8>::with_capacity(usize::from_str(compressed_size)?);
        resp.read_to_end(&mut resp_compressed_bytes)
            .map_err(|err| {
                Box::new(err_msg(format!("Error reading {} ({})", data_filename, err)).compat())
            })?;

        let mut uncompressed_bytes = Vec::<u8>::with_capacity(usize::from_str(uncompressed_size)?);
        let mut gz = GzDecoder::new(IterRead::new(resp_compressed_bytes.iter()));
        gz.read_to_end(&mut uncompressed_bytes).map_err(|err| {
            Box::new(err_msg(format!("Error decompressing {} ({})", data_filename, err)).compat())
        })?;

        let computed_hash =
            HEXUPPER.encode(digest::digest(&digest::SHA256, &uncompressed_bytes).as_ref());

        if hash != computed_hash {
            return Err(Box::new(
                err_msg(format!(
                    "Hash mismatch for {}, expected {}, got {}",
                    data_filename, hash, computed_hash
                ))
                .compat(),
            ));
        }

        let parsed_json: Value = serde_json::from_slice(&uncompressed_bytes)?;

        let cve_items = parsed_json["CVE_Items"]
            .as_array()
            .unwrap()
            .iter()
            .map(|vuln_info| {
                let v3_score = vuln_info
                    .get("impact")
                    .and_then(|impact| impact.get("baseMetricV3"))
                    .and_then(|base_metric_v3| base_metric_v3.get("cvssV3"))
                    .and_then(|cvss| cvss["baseScore"].as_f64());

                let v2_score = vuln_info
                    .get("impact")
                    .and_then(|impact| impact.get("baseMetricV2"))
                    .and_then(|base_metric_v2| base_metric_v2.get("cvssV2"))
                    .and_then(|cvss| cvss["baseScore"].as_f64());

                let cvss = if let Some(v3_score) = v3_score {
                    Cvss::builder()
                        .with_version(CvssVersion::V3)
                        .with_score(Some(v3_score as f32))
                        .build()
                } else if let Some(v2_score) = v2_score {
                    Cvss::builder()
                        .with_version(CvssVersion::V2)
                        .with_score(Some(v2_score as f32))
                        .build()
                } else {
                    Cvss::builder().with_version(CvssVersion::Unknown).build()
                };

                (
                    vuln_info["cve"]["CVE_data_meta"]["ID"]
                        .as_str()
                        .unwrap()
                        .to_string(),
                    cvss.unwrap(),
                )
            })
            .collect::<HashMap<_, _>>();

        return Ok(Some(cve_items));
    }

    Ok(None)
}

fn parse_tool_report(
    report: &ToolReport,
    vulns: &HashMap<String, Cvss>,
) -> Result<Vec<Vec<u8>>, Box<dyn Error>> {
    let events = if report.tool_name == "bundler-audit" {
        if report.output_format == "PlainText" {
            parse_bundler_audit_plaintext(&report, &vulns)
        } else {
            Err(Box::new(
                err_msg(format!(
                    "Unknown output format for Bundler-audit in ToolReport: {:?}",
                    report
                ))
                .compat(),
            )
            .into())
        }
    } else {
        Err(Box::new(err_msg(format!("Unknown tool in ToolReport: {:?}", report)).compat()).into())
    }?;

    Ok(events
        .iter()
        .map(|event| {
            let schema = Schema::parse_str(DEPENDENCY_EVENT_SCHEMA).unwrap();
            let mut writer = Writer::new(&schema, Vec::new());
            writer.append_ser(event).unwrap();
            writer.flush().unwrap();
            writer.into_inner()
        })
        .collect::<Vec<Vec<u8>>>())
}

fn parse_bundler_audit_plaintext(
    report: &ToolReport,
    vulns: &HashMap<String, Cvss>,
) -> Result<Vec<DependencyEvent>, Box<dyn Error>> {
    lazy_static! {
        static ref BLOCK_RE: Regex = Regex::new("(Name: .*\nVersion: .*\nAdvisory: .*\nCriticality: .*\nURL: .*\nTitle: .*\nSolution:.*\n)").unwrap();
    }
    let mut events = Vec::new();
    for block in BLOCK_RE.captures_iter(report.tool_output.as_ref()) {
        let block = block.get(0).unwrap().as_str();
        let fields = block
            .trim_end()
            .split('\n')
            .map(|line| line.split(": ").collect::<Vec<_>>())
            .map(|fields| (fields[0].to_string(), fields[1].to_string()))
            .collect::<HashMap<_, _>>();
        let advisory_id = AdvisoryId::try_from(
            fields
                .get("Advisory")
                .cloned()
                .or_else(|| Some("".to_string()))
                .unwrap()
                .to_owned(),
        )?;

        let default_cvss = Cvss::builder()
            .with_version(CvssVersion::Unknown)
            .build()
            .unwrap();

        let cvss = vulns.get(&advisory_id.to_string()).unwrap_or(&default_cvss);

        let mut event = DependencyEvent {
            event_version: EventVersion::try_from("1".to_string())?,
            event_id: EventID::try_from(Uuid::new_v4().to_hyphenated().to_string())?,
            parent_event_id: report.event_id.clone(),
            application_name: report.application_name.clone(),
            git_branch: report.git_branch.clone(),
            git_commit_hash: report.git_commit_hash.clone(),
            timestamp: Timestamp::try_from(report.end_time.to_string())?,
            affected_package: AffectedPackage::try_from(
                fields
                    .get("Name")
                    .cloned()
                    .or_else(|| Some("".to_string()))
                    .unwrap()
                    .to_owned(),
            )?,
            installed_version: InstalledVersion::try_from(
                fields
                    .get("Version")
                    .cloned()
                    .or_else(|| Some("".to_string()))
                    .unwrap()
                    .to_owned(),
            )?,
            advisory_url: AdvisoryUrl::try_from(
                fields
                    .get("URL")
                    .cloned()
                    .or_else(|| Some("".to_string()))
                    .unwrap()
                    .to_owned(),
            )?,
            advisory_id,
            advisory_description: AdvisoryDescription::try_from(
                fields
                    .get("Title")
                    .cloned()
                    .or_else(|| Some("".to_owned()))
                    .unwrap()
                    .to_owned(),
            )?,
            cvss: cvss.clone(),
            suppressed: false,
        };

        let issue_hash = IssueHash::try_from(hex::encode(event.hash()))?;

        event.suppressed =
            should_issue_be_suppressed(&issue_hash, &report.suppressed_issues, &Utc::now());

        events.push(event);
    }
    Ok(events)
}

#[derive(Clone, SerdeValue, Serialize, Deserialize)]
struct EventType(Vec<String>);

fn should_issue_be_suppressed(
    issue_hash: &IssueHash,
    suppressed_issues: &[SuppressedIssue],
    current_time: &DateTime<Utc>,
) -> bool {
    if suppressed_issues.is_empty() {
        false
    } else {
        let matching_issues = suppressed_issues
            .iter()
            .filter(|x| &x.issue_hash == issue_hash)
            .collect::<Vec<_>>();
        if matching_issues.is_empty() {
            false
        } else {
            matching_issues
                .iter()
                .any(|x| x.expiry_date.is_none() || x.expiry_date > *current_time)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kiln_lib::tool_report::{ExpiryDate, SuppressedBy, SuppressionReason};

    #[test]
    fn issue_suppression_works_when_suppressed_issues_is_empty() {
        let test_hash = IssueHash::try_from(
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_owned(),
        )
        .unwrap();
        let suppressed_issues: Vec<SuppressedIssue> = vec![];
        let test_date = Utc.ymd(2020, 05, 18).and_hms(12, 00, 00);
        assert_eq!(
            false,
            should_issue_be_suppressed(&test_hash, &suppressed_issues, &test_date)
        );
    }

    #[test]
    fn issue_suppression_works_when_suppressed_issues_does_not_contain_matching_hash() {
        let test_hash = IssueHash::try_from(
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_owned(),
        )
        .unwrap();
        let suppressed_issues: Vec<SuppressedIssue> = vec![SuppressedIssue {
            issue_hash: IssueHash::try_from(
                "a441b688fb60942c701fbcee0f30c66c0f7b22da7f0b4c51488488d2a2b64197".to_owned(),
            )
            .unwrap(),
            expiry_date: ExpiryDate::from(None),
            suppression_reason: SuppressionReason::try_from("Test issue".to_owned()).unwrap(),
            suppressed_by: SuppressedBy::try_from("Dan Murphy".to_owned()).unwrap(),
        }];
        let test_date = Utc.ymd(2020, 05, 18).and_hms(12, 00, 00);
        assert_eq!(
            false,
            should_issue_be_suppressed(&test_hash, &suppressed_issues, &test_date)
        );
    }

    #[test]
    fn issue_suppression_works_when_suppressed_issues_contains_hash_with_current_suppression() {
        let test_hash = IssueHash::try_from(
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_owned(),
        )
        .unwrap();
        let suppressed_issues: Vec<SuppressedIssue> = vec![
            SuppressedIssue {
                issue_hash: IssueHash::try_from(
                    "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_owned(),
                )
                .unwrap(),
                expiry_date: ExpiryDate::from(Some(Utc.ymd(2020, 05, 20).and_hms(12, 0, 0))),
                suppression_reason: SuppressionReason::try_from("Matching issue".to_owned())
                    .unwrap(),
                suppressed_by: SuppressedBy::try_from("Dan Murphy".to_owned()).unwrap(),
            },
            SuppressedIssue {
                issue_hash: IssueHash::try_from(
                    "46a9d5bde718bf366178313019f04a753bad00685d38e3ec81c8628f35dfcb1b".to_owned(),
                )
                .unwrap(),
                expiry_date: ExpiryDate::from(None),
                suppression_reason: SuppressionReason::try_from("Test issue".to_owned()).unwrap(),
                suppressed_by: SuppressedBy::try_from("Dan Murphy".to_owned()).unwrap(),
            },
        ];
        let test_date = Utc.ymd(2020, 05, 18).and_hms(12, 00, 00);
        assert_eq!(
            true,
            should_issue_be_suppressed(&test_hash, &suppressed_issues, &test_date)
        );
    }

    #[test]
    fn issue_suppression_works_when_suppressed_issues_contains_hash_with_expired_suppression_by_date(
    ) {
        let test_hash = IssueHash::try_from(
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_owned(),
        )
        .unwrap();
        let suppressed_issues: Vec<SuppressedIssue> = vec![
            SuppressedIssue {
                issue_hash: IssueHash::try_from(
                    "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_owned(),
                )
                .unwrap(),
                expiry_date: ExpiryDate::from(Some(Utc.ymd(2020, 05, 17).and_hms(0, 0, 0))),
                suppression_reason: SuppressionReason::try_from("Matching issue".to_owned())
                    .unwrap(),
                suppressed_by: SuppressedBy::try_from("Dan Murphy".to_owned()).unwrap(),
            },
            SuppressedIssue {
                issue_hash: IssueHash::try_from(
                    "9cf8847d2992e7219e659cdde1969e0d567ebab39a7aba13b36f9916fa26f6ca".to_owned(),
                )
                .unwrap(),
                expiry_date: ExpiryDate::from(None),
                suppression_reason: SuppressionReason::try_from("Test issue".to_owned()).unwrap(),
                suppressed_by: SuppressedBy::try_from("Dan Murphy".to_owned()).unwrap(),
            },
        ];
        let test_date = Utc.ymd(2020, 05, 18).and_hms(12, 00, 00);
        assert_eq!(
            false,
            should_issue_be_suppressed(&test_hash, &suppressed_issues, &test_date)
        );
    }

    #[test]
    fn issue_suppression_works_when_suppressed_issues_contains_hash_with_expired_suppression_by_date_and_time(
    ) {
        let test_hash = IssueHash::try_from(
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_owned(),
        )
        .unwrap();
        let suppressed_issues: Vec<SuppressedIssue> = vec![
            SuppressedIssue {
                issue_hash: IssueHash::try_from(
                    "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_owned(),
                )
                .unwrap(),
                expiry_date: ExpiryDate::from(Some(Utc.ymd(2020, 05, 18).and_hms(10, 0, 0))),
                suppression_reason: SuppressionReason::try_from("Matching issue".to_owned())
                    .unwrap(),
                suppressed_by: SuppressedBy::try_from("Dan Murphy".to_owned()).unwrap(),
            },
            SuppressedIssue {
                issue_hash: IssueHash::try_from(
                    "b100dabbadeedabbad1eadabbadeedabbad1edabbadeedabbad1eadabbadeeda".to_owned(),
                )
                .unwrap(),
                expiry_date: ExpiryDate::from(None),
                suppression_reason: SuppressionReason::try_from("Test issue".to_owned()).unwrap(),
                suppressed_by: SuppressedBy::try_from("Dan Murphy".to_owned()).unwrap(),
            },
        ];
        let test_date = Utc.ymd(2020, 05, 18).and_hms(12, 00, 00);
        assert_eq!(
            false,
            should_issue_be_suppressed(&test_hash, &suppressed_issues, &test_date)
        );
    }

    #[test]
    fn issue_suppression_works_when_suppressed_issues_contains_hash_with_suppression_with_no_expiry(
    ) {
        let test_hash = IssueHash::try_from(
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_owned(),
        )
        .unwrap();
        let suppressed_issues: Vec<SuppressedIssue> = vec![
            SuppressedIssue {
                issue_hash: IssueHash::try_from(
                    "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_owned(),
                )
                .unwrap(),
                expiry_date: ExpiryDate::from(None),
                suppression_reason: SuppressionReason::try_from("Matching issue".to_owned())
                    .unwrap(),
                suppressed_by: SuppressedBy::try_from("Dan Murphy".to_owned()).unwrap(),
            },
            SuppressedIssue {
                issue_hash: IssueHash::try_from(
                    "a41f58ced5996b018dfbd697c1b16675f0cf864a3475d237cdd3f4d8c7160fdb".to_owned(),
                )
                .unwrap(),
                expiry_date: ExpiryDate::from(None),
                suppression_reason: SuppressionReason::try_from("Test issue".to_owned()).unwrap(),
                suppressed_by: SuppressedBy::try_from("Dan Murphy".to_owned()).unwrap(),
            },
        ];
        let test_date = Utc.ymd(2020, 05, 18).and_hms(12, 00, 00);
        assert_eq!(
            true,
            should_issue_be_suppressed(&test_hash, &suppressed_issues, &test_date)
        );
    }

    #[test]
    fn issue_suppression_works_when_suppressed_issues_contains_multiple_hashes_with_two_current_suppressions(
    ) {
        let test_hash = IssueHash::try_from(
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_owned(),
        )
        .unwrap();
        let suppressed_issues: Vec<SuppressedIssue> = vec![
            SuppressedIssue {
                issue_hash: IssueHash::try_from(
                    "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_owned(),
                )
                .unwrap(),
                expiry_date: ExpiryDate::from(Some(Utc.ymd(2020, 05, 19).and_hms(12, 0, 0))),
                suppression_reason: SuppressionReason::try_from("Matching issue".to_owned())
                    .unwrap(),
                suppressed_by: SuppressedBy::try_from("Dan Murphy".to_owned()).unwrap(),
            },
            SuppressedIssue {
                issue_hash: IssueHash::try_from(
                    "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_owned(),
                )
                .unwrap(),
                expiry_date: ExpiryDate::from(Some(Utc.ymd(2020, 07, 19).and_hms(12, 0, 0))),
                suppression_reason: SuppressionReason::try_from("Matching issue".to_owned())
                    .unwrap(),
                suppressed_by: SuppressedBy::try_from("Dan Murphy".to_owned()).unwrap(),
            },
        ];
        let test_date = Utc.ymd(2020, 05, 18).and_hms(12, 00, 00);
        assert_eq!(
            true,
            should_issue_be_suppressed(&test_hash, &suppressed_issues, &test_date)
        );
    }

    #[test]
    fn issue_suppression_works_when_suppressed_issues_contains_multiple_hashes_with_one_expired_suppression_and_one_current_suppression(
    ) {
        let test_hash = IssueHash::try_from(
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_owned(),
        )
        .unwrap();
        let suppressed_issues: Vec<SuppressedIssue> = vec![
            SuppressedIssue {
                issue_hash: IssueHash::try_from(
                    "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_owned(),
                )
                .unwrap(),
                expiry_date: ExpiryDate::from(Some(Utc.ymd(2020, 05, 17).and_hms(12, 0, 0))),
                suppression_reason: SuppressionReason::try_from("Matching issue".to_owned())
                    .unwrap(),
                suppressed_by: SuppressedBy::try_from("Dan Murphy".to_owned()).unwrap(),
            },
            SuppressedIssue {
                issue_hash: IssueHash::try_from(
                    "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_owned(),
                )
                .unwrap(),
                expiry_date: ExpiryDate::from(Some(Utc.ymd(2020, 07, 19).and_hms(12, 0, 0))),
                suppression_reason: SuppressionReason::try_from("Matching issue".to_owned())
                    .unwrap(),
                suppressed_by: SuppressedBy::try_from("Dan Murphy".to_owned()).unwrap(),
            },
        ];
        let test_date = Utc.ymd(2020, 05, 18).and_hms(12, 00, 00);
        assert_eq!(
            true,
            should_issue_be_suppressed(&test_hash, &suppressed_issues, &test_date)
        );
    }
}
