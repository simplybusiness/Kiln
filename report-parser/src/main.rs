#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate slog;

use avro_rs::{Reader, Schema, Writer};
use chrono::prelude::*;
use chrono::Duration;
use chrono::{SecondsFormat, Utc};
use compressed_string::ComprString;
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
use kiln_lib::log::NestedJsonFmt;
use kiln_lib::tool_report::{
    ApplicationName, EndTime, Environment, EventID, EventVersion, GitBranch, GitCommitHash,
    IssueHash, OutputFormat, StartTime, SuppressedIssue, ToolName, ToolOutput, ToolReport,
    ToolVersion
};
use kiln_lib::traits::Hashable;
use rdkafka::consumer::{CommitMode, Consumer};
use rdkafka::message::Message;
use rdkafka::producer::future_producer::FutureRecord;
use regex::Regex;
use reqwest::blocking::Client;
use reqwest::header::ETAG;
use ring::digest;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
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
use uuid::Uuid;

use httpmock::MockServer;

const SERVICE_NAME: &str = "report-parser";

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let drain = NestedJsonFmt::new(std::io::stdout()).fuse();

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

    let error_logger = root_logger.new(o!("event.type" => EventType(vec!("error".to_string()))));

    let config = get_bootstrap_config(&mut env::vars()).map_err(|err| {
        error!(error_logger, "Error building Kafka configuration";
            o!(
                "error.message" => err.to_string(),
            )
        );
        err
    })?;

    let consumer =
        build_kafka_consumer(config.clone(), "report-parser".to_string()).map_err(|err| {
            error!(error_logger, "Error building Kafka consumer";
                o!(
                    "error.message" => err.to_string(),
                )
            );
            err
        })?;

    consumer.subscribe(&["ToolReports"]).map_err(|err| {
        error!(error_logger, "Error subscribing to ToolReports Kafka topic";
            o!(
                "error.message" => err.to_string(),
            )
        );
        err
    })?;

    let producer = build_kafka_producer(config.clone()).map_err(|err| {
        error!(error_logger, "Error building Kafka producer";
            o!(
                "error.message" => err.to_string(),
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
    let mut safety_cve_map = HashMap::new();
    for year in 2002..=Utc::today().year() {
        download_and_parse_vulns(year.to_string(), last_updated_time, &base_url, &client)
            .map_err(|err| {
                error!(error_logger, "Error downloading vulns for {}", year;
                    o!(
                        "error.message" => err.to_string(),
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
        error!(error_logger, "Error downloading modified vulns info";
            o!(
                "error.message" => err.to_string(),
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

    let mut etag = None;
    let safety_json_db_url =
        "https://raw.githubusercontent.com/pyupio/safety-db/master/data/insecure_full.json"
            .to_string();
    let mut safety_cve_map =
        download_and_parse_python_safety_vulns(safety_json_db_url.clone(), &mut etag, &client)
            .map_err(|err| {
                error!(error_logger, "Error downloading Python Safety Vulns";
                    o!(
                        "error.message" => err.to_string(),
                    )
                );
                err
            })?;

    last_updated_time = Some(Utc::now());

    let mut messages = consumer.start_with(std::time::Duration::from_secs(1), false);

    loop {
        if last_updated_time
            .unwrap()
            .lt(&(Utc::now() - Duration::days(1)))
        {
            let new_safety_cve_map = download_and_parse_python_safety_vulns(
                safety_json_db_url.clone(),
                &mut etag,
                &client,
            )
            .map_err(|err| {
                error!(error_logger, "Error downloading Python Safety Vulns";
                    o!(
                        "error.message" => err.to_string(),
                    )
                );
                err
            })?;
            if new_safety_cve_map.is_some() {
                safety_cve_map = new_safety_cve_map;
            }
        }

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
                error!(error_logger, "Error updating vuln info";
                    o!(
                        "error.message" => err.to_string(),
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
                let reader = Reader::new(body).map_err(|err| {
                    error!(error_logger, "Error creating Avro reader from message bytes";
                        o!(
                            "error.message" => err.to_string(),
                        )
                    );
                    err
                })?;
                for value in reader {
                    let report = ToolReport::try_from(value?).map_err(|err| {
                        error!(error_logger, "Error parsing Avro to ToolReport";
                            o!(
                                "error.message" => err.to_string(),
                            )
                        );
                        err
                    })?;
                    let app_name = report.application_name.to_string();
                    let records =
                        parse_tool_report(&report, &vulns, safety_cve_map.as_ref().unwrap())
                            .map_err(|err| {
                                error!(error_logger, "Error parsing tool output in ToolReport";
                                    o!(
                                        "error.message" => err.to_string(),
                                    )
                                );
                                err
                            })?;
                    for record in records.into_iter() {
                        let kafka_payload = FutureRecord::to("DependencyEvents")
                            .payload(&record)
                            .key(&app_name);
                        producer.send(kafka_payload, 5000).await?.map_err(|err| {
                            error!(error_logger, "Error publishing DependencyEvent to Kafka";
                                o!(
                                    "error.message" => err.0.to_string(),
                                )
                            );
                            err.0
                        })?;
                    }
                }
            }
            consumer
                .commit_message(&message, CommitMode::Async)
                .map_err(|err| {
                    error!(error_logger, "Error committing consumed offset to Kafka";
                        o!(
                            "error.message" => err.to_string(),
                        )
                    );
                    err
                })?;
        }
    }
}

#[derive(Clone)]
struct VulnData {
    cvss: Cvss,
    advisory_str: ComprString,
    advisory_url: String,
}

fn download_and_parse_vulns(
    index: String,
    last_updated_time: Option<DateTime<Utc>>,
    base_url: &Url,
    client: &Client,
) -> Result<Option<HashMap<String, VulnData>>, Box<dyn Error>> {
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

                let adv_text = vuln_info
                    .get("cve")
                    .and_then(|cve| cve.get("description"))
                    .and_then(|desc| desc.get("description_data"))
                    .and_then(|desc_data| desc_data.get("value"))
                    .unwrap()
                    .to_string();

                let compr_adv_text = ComprString::new(&adv_text);

                let adv_url = vuln_info
                    .get("cve")
                    .and_then(|cve| cve.get("reference"))
                    .and_then(|refer| refer.get("reference_data"))
                    .and_then(|refer_data| refer_data.get("url"))
                    .unwrap()
                    .to_string();

                (
                    vuln_info["cve"]["CVE_data_meta"]["ID"]
                        .as_str()
                        .unwrap()
                        .to_string(),
                    VulnData {
                        advisory_str: compr_adv_text,
                        advisory_url: adv_url,
                        cvss: cvss.unwrap(),
                    },
                )
            })
            .collect::<HashMap<_, _>>();

        return Ok(Some(cve_items));
    }

    Ok(None)
}

fn parse_tool_report(
    report: &ToolReport,
    vulns: &HashMap<String, VulnData>,
    safety_vuln_map: &HashMap<String, Option<String>>,
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
    } else if report.tool_name == "safety" {
        if report.output_format == "JSON" {
            parse_safety_json(&report, vulns, safety_vuln_map)
        } else if report.output_format == "PlainText" {
            Err(Box::new(
                    err_msg(format!(
                            "PlainText output not supported for safety; re-run safety with --json option in ToolReport: {:?}",
                            report
                    ))
                    .compat(),
            )
                .into())
        } else {
            Err(Box::new(
                err_msg(format!(
                    "Unknown output format for safety in ToolReport: {:?}",
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
    vulns: &HashMap<String, VulnData>,
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

        let cvss = vulns
            .get(&advisory_id.to_string())
            .map_or(&default_cvss, |v| &v.cvss);

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

#[derive(Deserialize, Debug)]
struct PythonSafety {
    affected_package: String,
    affected_versions: String,
    installed_version: String,
    advisory_description: String,
    advisory_id: String,
}

#[derive(Deserialize, Debug)]
struct MetaInfo {
    advisory: String,
    timestamp: Value,
}

#[derive(Deserialize, Debug)]
struct SafetyPackageVulnInfo {
    advisory: String,
    cve: Value,
    id: String,
    specs: Vec<String>,
    v: String,
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
enum SafetyJsonData {
    Vuln(Vec<SafetyPackageVulnInfo>),
    Meta(MetaInfo),
}

fn download_and_parse_python_safety_vulns(
    server_name: String,
    etag: &mut Option<String>,
    client: &Client,
) -> Result<Option<HashMap<String, Option<String>>>, Box<dyn Error>> {
    let safety_json_db_url = &server_name.to_string();
    let head_resp = client.head(safety_json_db_url).send()?;
    if head_resp.status().is_success() {
        if let Some(etag_new) = head_resp.headers().get(ETAG) {
            // If the etag passed in is none or different to the one we just got, then download below....
            match etag {
                Some(etag_old) => {
                    if *etag_old == etag_new.to_str().unwrap() {
                        return Ok(None);
                    } else {
                        *etag = Some(etag_new.to_str().unwrap().to_owned());
                    }
                }
                None => *etag = Some(etag_new.to_str().unwrap().to_owned()),
            }
        }
    } else {
        return Err(Box::new(
            err_msg(format!(
                "Unable to grab head from python safety database: ({})",
                head_resp.status()
            ))
            .compat(),
        )
        .into());
    }

    let meta_resp_text = reqwest::blocking::get(safety_json_db_url)?.text()?;
    let python_safety_vuln_info_json: HashMap<String, SafetyJsonData> =
        serde_json::from_str(meta_resp_text.as_ref())?;
    let cve_items = python_safety_vuln_info_json
        .values()
        .filter(|ref _s| match _s {
            SafetyJsonData::Vuln(_s) => true,
            _ => false,
        })
        .map(|s| match s {
            SafetyJsonData::Vuln(s) => s.iter(),
            _ => unreachable!(),
        })
        .collect::<Vec<_>>()
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .iter()
        .map(|p| match &p.cve {
            Value::String(s) => (p.id.to_owned(), Some(s.to_owned())),
            _ => (p.id.to_owned(), None),
        })
        .collect::<HashMap<_, _>>();
    Ok(Some(cve_items))
}

fn parse_safety_json(
    report: &ToolReport,
    vulns: &HashMap<String, VulnData>,
    safety_vulns: &HashMap<String, Option<String>>,
) -> Result<Vec<DependencyEvent>, Box<dyn Error>> {
    let mut events = Vec::new();
    let python_dep_vulns: Vec<PythonSafety> = serde_json::from_str(report.tool_output.as_ref())?;
    for vuln in python_dep_vulns.iter() {
        let advisory_id = AdvisoryId::try_from(vuln.advisory_id.to_owned())?;

        let default_cvss = Cvss::builder()
            .with_version(CvssVersion::Unknown)
            .build()
            .unwrap();

        let mut advisory_str = vuln.advisory_description.to_string();
        let mut advisory_url =
            "https://github.com/pyupio/safety-db/blob/master/data/insecure_full.json".to_string();
        let mut cvss = &default_cvss;

        match safety_vulns
            .get(&format!("pyup.io-{}", advisory_id.to_string()))
            .unwrap()
        {
            Some(s) => {
                let cve_vec = s.split(',').collect::<Vec<&str>>();
                for cve_str in cve_vec {
                    cvss = vulns.get(cve_str).map_or(&default_cvss, |v| &v.cvss);
                    advisory_str = vulns
                        .get(cve_str)
                        .map_or(vuln.advisory_description.to_string(), |v| {
                            v.advisory_str.to_string()
                        });
                    advisory_url = vulns.get(cve_str).map_or(
                        "https://github.com/pyupio/safety-db/blob/master/data/insecure_full.json"
                            .to_string(),
                        |v| v.advisory_url.to_string(),
                    );
                }
            }
            _ => (),
        };
        let mut event = DependencyEvent {
            event_version: EventVersion::try_from("1".to_string())?,
            event_id: EventID::try_from(Uuid::new_v4().to_hyphenated().to_string())?,
            parent_event_id: report.event_id.clone(),
            application_name: report.application_name.clone(),
            git_branch: report.git_branch.clone(),
            git_commit_hash: report.git_commit_hash.clone(),
            timestamp: Timestamp::try_from(report.end_time.to_string())?,
            affected_package: AffectedPackage::try_from(vuln.affected_package.to_owned())?,
            installed_version: InstalledVersion::try_from(vuln.installed_version.to_owned())?,
            advisory_url: AdvisoryUrl::try_from(advisory_url)?,
            advisory_id: advisory_id.clone(),
            advisory_description: AdvisoryDescription::try_from(advisory_str)?,
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

    #[test]
    fn download_safety_vuln_db_error_status() {
        let mut etag = None;
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.any_request();
            then.status(502);
        });
        let client = Client::new();
        assert!(
            download_and_parse_python_safety_vulns(server.url("/data"), &mut etag, &client)
                .is_err(),
            "HTTP error status not handled correctly"
        );
        mock.assert_hits(1);
    }

    #[test]
    fn download_safety_vuln_db_etag_none() {
        let mut etag = None;
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.any_request();
            then.status(200)
                .header("Connection", "keep-alive")
                .header("Content-Length", "348")
                .header("Content-Type", "text/plain; charset=utf-8")
                .header("Cache-Control", "max-age=300")
                .header(
                    "Content-Security-Policy",
                    "default-src \"none\"; style-src \"unsafe-inline\"; sandbox",
                )
                .header(
                    "ETag",
                    "W/\"3e557b6621332dd9eb4ee95322df0a2971c87322fdf56abaa1d6038a05ba4f26\"",
                )
                .header("Strict-Transport-Security", "max-age=31536000")
                .header("X-Content-Type-Options", "nosniff")
                .header("X-Frame-Options", "deny")
                .header("X-XSS-Protection", "1; mode=block")
                .header("Via", "1.1 varnish (Varnish/6.0), 1.1 varnish")
                .header("Content-Encoding", "gzip")
                .header("X-GitHub-Request-Id", "2DD2:2A31:366058:393F4A:5FC8BA35")
                .header("Accept-Ranges", "bytes")
                .header("Date", "Thu, 03 Dec 2020 12:05:06 GMT")
                .header("X-Served-By", "cache-lcy19250-LCY")
                .header("X-Cache", "HFM, HIT")
                .header("X-Cache-Hits", "0, 1")
                .header("X-Timer", "S1606997106.156669,VS0,VE1")
                .header("Vary", "Authorization,Accept-Encoding, Accept-Encoding")
                .header("Access-Control-Allow-Origin", "*")
                .header(
                    "X-Fastly-Request-ID",
                    "4d1a4685623b535c8417cc2dfab67794968e04a6",
                )
                .header("Expires", "Thu, 03 Dec 2020 12:10:06 GMT")
                .header("Source-Age", "129")
                .json_body(json!({
                    "$meta": {
                        "advisory": "PyUp.io metadata",
                        "timestamp": 1606802401
                    },
                    "acqusition": [
                    {
                        "advisory": "acqusition is a package affected by pytosquatting",
                        "cve": null,
                        "id": "pyup.io-34978",
                        "specs": [
                            ">0",
                            "<0"
                        ],
                        "v": ">0,<0"
                    }],
                    "aegea": [
                    {
                        "advisory": "Aegea 2.2.7 avoids CVE-2018-1000805.",
                        "cve": "CVE-2018-1000805",
                        "id": "pyup.io-37611",
                        "specs": [
                            "<2.2.7"
                        ],
                        "v": "<2.2.7"
                    }
                    ]
                }));
        });
        let client = Client::new();
        let res = download_and_parse_python_safety_vulns(server.url("/data"), &mut etag, &client);
        assert!(etag.is_some());
        assert!(
            res.is_ok(),
            "Error result was produced when dealing with a None Etag"
        );
        let res_ok = res.unwrap();
        assert!(
            res_ok.is_some(),
            "Python Safety vulns CVE hash not set when Etag is None"
        );
        let vhash = res_ok.unwrap();
        assert!(
            vhash.len() > 0,
            "Incorrect number of elements in the Python safety map"
        );
        assert!(
            vhash.contains_key("pyup.io-37611"),
            "Python Safety map returned does not contain the correct key"
        );
        let value = vhash.get("pyup.io-37611").unwrap();
        assert_eq!(value.as_ref().unwrap(), "CVE-2018-1000805");
        mock.assert_hits(2);
    }

    #[test]
    fn download_safety_vuln_db_etags_diff() {
        let mut etag =
            Some("3e557b6621332dd9eb4ee95322df0a2971c87322fdf56abaa1d6038a05ba4f22".to_string());
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.any_request();
            then.status(200)
                .header("Connection", "keep-alive")
                .header("Content-Length", "348")
                .header("Content-Type", "text/plain; charset=utf-8")
                .header("Cache-Control", "max-age=300")
                .header(
                    "Content-Security-Policy",
                    "default-src \"none\"; style-src \"unsafe-inline\"; sandbox",
                )
                .header(
                    "ETag",
                    "W/\"3e557b6621332dd9eb4ee95322df0a2971c87322fdf56abaa1d4038a05ba4f26\"",
                )
                .header("Strict-Transport-Security", "max-age=31536000")
                .header("X-Content-Type-Options", "nosniff")
                .header("X-Frame-Options", "deny")
                .header("X-XSS-Protection", "1; mode=block")
                .header("Via", "1.1 varnish (Varnish/6.0), 1.1 varnish")
                .header("Content-Encoding", "gzip")
                .header("X-GitHub-Request-Id", "2DD2:2A31:366058:393F4A:5FC8BA35")
                .header("Accept-Ranges", "bytes")
                .header("Date", "Thu, 03 Dec 2020 12:05:06 GMT")
                .header("X-Served-By", "cache-lcy19250-LCY")
                .header("X-Cache", "HFM, HIT")
                .header("X-Cache-Hits", "0, 1")
                .header("X-Timer", "S1606997106.156669,VS0,VE1")
                .header("Vary", "Authorization,Accept-Encoding, Accept-Encoding")
                .header("Access-Control-Allow-Origin", "*")
                .header(
                    "X-Fastly-Request-ID",
                    "4d1a4685623b535c8417cc2dfab67794968e04a6",
                )
                .header("Expires", "Thu, 03 Dec 2020 12:10:06 GMT")
                .header("Source-Age", "129")
                .json_body(json!({
                    "$meta": {
                        "advisory": "PyUp.io metadata",
                        "timestamp": 1606802401
                    },
                    "acqusition": [
                    {
                        "advisory": "acqusition is a package affected by pytosquatting",
                        "cve": null,
                        "id": "pyup.io-34978",
                        "specs": [
                            ">0",
                            "<0"
                        ],
                        "v": ">0,<0"
                    }],
                    "aegea": [
                    {
                        "advisory": "Aegea 2.2.7 avoids CVE-2018-1000805.",
                        "cve": "CVE-2018-1000805",
                        "id": "pyup.io-37611",
                        "specs": [
                            "<2.2.7"
                        ],
                        "v": "<2.2.7"
                    }
                    ]
                }));
        });
        let client = Client::new();
        let old_etag = etag.clone();
        let res = download_and_parse_python_safety_vulns(server.url("/data"), &mut etag, &client);
        assert!(etag.is_some());
        assert!(old_etag != etag, "Etags cannot be matching");
        assert!(
            res.is_ok(),
            "Error result was produced when dealing with matching Etags"
        );
        let res_ok = res.unwrap();
        assert!(
            res_ok.is_some(),
            "Python Safety vulns CVE hash not set when Etags are matching"
        );
        let vhash = res_ok.unwrap();
        assert!(
            vhash.len() > 0,
            "Incorrect number of elements in the Python safety map"
        );
        assert!(
            vhash.contains_key("pyup.io-37611"),
            "Python Safety map returned does not contain the correct key"
        );
        let value = vhash.get("pyup.io-37611").unwrap();
        assert_eq!(value.as_ref().unwrap(), "CVE-2018-1000805");
        mock.assert_hits(2);
    }

    #[test]
    fn download_safety_vuln_db_etags_matching() {
        let mut etag =
            Some("3e557b6621332dd9eb4ee95322df0a2971c87322fdf56abaa1d6038a05ba4f26".to_string());
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.any_request();
            then.status(200)
                .header("Connection", "keep-alive")
                .header("Content-Length", "348")
                .header("Content-Type", "text/plain; charset=utf-8")
                .header("Cache-Control", "max-age=300")
                .header(
                    "Content-Security-Policy",
                    "default-src \"none\"; style-src \"unsafe-inline\"; sandbox",
                )
                .header(
                    "ETag",
                    "W/\"3e557b6621332dd9eb4ee95322df0a2971c87322fdf56abaa1d6038a05ba4f26\"",
                )
                .header("Strict-Transport-Security", "max-age=31536000")
                .header("X-Content-Type-Options", "nosniff")
                .header("X-Frame-Options", "deny")
                .header("X-XSS-Protection", "1; mode=block")
                .header("Via", "1.1 varnish (Varnish/6.0), 1.1 varnish")
                .header("Content-Encoding", "gzip")
                .header("X-GitHub-Request-Id", "2DD2:2A31:366058:393F4A:5FC8BA35")
                .header("Accept-Ranges", "bytes")
                .header("Date", "Thu, 03 Dec 2020 12:05:06 GMT")
                .header("X-Served-By", "cache-lcy19250-LCY")
                .header("X-Cache", "HFM, HIT")
                .header("X-Cache-Hits", "0, 1")
                .header("X-Timer", "S1606997106.156669,VS0,VE1")
                .header("Vary", "Authorization,Accept-Encoding, Accept-Encoding")
                .header("Access-Control-Allow-Origin", "*")
                .header(
                    "X-Fastly-Request-ID",
                    "4d1a4685623b535c8417cc2dfab67794968e04a6",
                )
                .header("Expires", "Thu, 03 Dec 2020 12:10:06 GMT")
                .header("Source-Age", "129")
                .json_body(json!({
                    "$meta": {
                        "advisory": "PyUp.io metadata",
                        "timestamp": 1606802401
                    },
                    "acqusition": [
                    {
                        "advisory": "acqusition is a package affected by pytosquatting",
                        "cve": null,
                        "id": "pyup.io-34978",
                        "specs": [
                            ">0",
                            "<0"
                        ],
                        "v": ">0,<0"
                    }],
                    "aegea": [
                    {
                        "advisory": "Aegea 2.2.7 avoids CVE-2018-1000805.",
                        "cve": "CVE-2018-1000805",
                        "id": "pyup.io-37611",
                        "specs": [
                            "<2.2.7"
                        ],
                        "v": "<2.2.7"
                    }
                    ]
                }));
        });
        let client = Client::new();
        let old_etag = etag.clone();
        let res = download_and_parse_python_safety_vulns(server.url("/data"), &mut etag, &client);
        assert!(old_etag == etag);
        assert!(
            res.is_ok(),
            "Error result was produced when dealing with a matching Etags"
        );
        let res_ok = res.unwrap();
        assert!(
            res_ok.is_none(),
            "Matching etags should produce a None result"
        );
        mock.assert_hits(1);
    }

    #[test]
    fn download_safety_vuln_db_etag_some() {
        let mut etag = None;
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.any_request();
            then.status(200)
                .header("Connection", "keep-alive")
                .header("Content-Length", "348")
                .header("Content-Type", "text/plain; charset=utf-8")
                .header("Cache-Control", "max-age=300")
                .header(
                    "Content-Security-Policy",
                    "default-src \"none\"; style-src \"unsafe-inline\"; sandbox",
                )
                .header(
                    "ETag",
                    "W/\"3e557b6621332dd9eb4ee95322df0a2971c87322fdf56abaa1d6038a05ba4f26\"",
                )
                .header("Strict-Transport-Security", "max-age=31536000")
                .header("X-Content-Type-Options", "nosniff")
                .header("X-Frame-Options", "deny")
                .header("X-XSS-Protection", "1; mode=block")
                .header("Via", "1.1 varnish (Varnish/6.0), 1.1 varnish")
                .header("Content-Encoding", "gzip")
                .header("X-GitHub-Request-Id", "2DD2:2A31:366058:393F4A:5FC8BA35")
                .header("Accept-Ranges", "bytes")
                .header("Date", "Thu, 03 Dec 2020 12:05:06 GMT")
                .header("X-Served-By", "cache-lcy19250-LCY")
                .header("X-Cache", "HFM, HIT")
                .header("X-Cache-Hits", "0, 1")
                .header("X-Timer", "S1606997106.156669,VS0,VE1")
                .header("Vary", "Authorization,Accept-Encoding, Accept-Encoding")
                .header("Access-Control-Allow-Origin", "*")
                .header(
                    "X-Fastly-Request-ID",
                    "4d1a4685623b535c8417cc2dfab67794968e04a6",
                )
                .header("Expires", "Thu, 03 Dec 2020 12:10:06 GMT")
                .header("Source-Age", "129")
                .json_body(json!({
                    "$meta": {
                        "advisory": "PyUp.io metadata",
                        "timestamp": 1606802401
                    },
                    "acqusition": [
                    {
                        "advisory": "acqusition is a package affected by pytosquatting",
                        "cve": null,
                        "id": "pyup.io-34978",
                        "specs": [
                            ">0",
                            "<0"
                        ],
                        "v": ">0,<0"
                    }],
                    "aegea": [
                    {
                        "advisory": "Aegea 2.2.7 avoids CVE-2018-1000805.",
                        "cve": "CVE-2018-1000805",
                        "id": "pyup.io-37611",
                        "specs": [
                            "<2.2.7"
                        ],
                        "v": "<2.2.7"
                    }
                    ]
                }));
        });
        let client = Client::new();
        match download_and_parse_python_safety_vulns(server.url("/data"), &mut etag, &client) {
            Err(e) => println!("Error: Unable to download safety vulns: {} \n", e),
            Ok(Some(vulnshash)) => {
                println!("Etag:{:?}", etag.clone());
                for (key, val) in &vulnshash {
                    match val {
                        Some(cve) => println!("{} --> {}", key, cve),
                        None => (),
                    }
                }
            }
            Ok(None) => println!("Error: No vulnshash returned"),
        }
        let old_etag = etag.clone();
        /* Download again to ensure that matching etags returns None */
        let newvulns =
            download_and_parse_python_safety_vulns(server.url("/data"), &mut etag, &client)
                .expect("Download safety vuln returned Error");
        assert!(etag == old_etag, "Error: etags do not match");
        assert!(
            newvulns.is_none(),
            "Error: Safety vulns downloaded despite matching etags"
        );
        mock.assert_hits(3);
    }

    #[test]
    fn serde_parse_safety() {
        let serde_json_data = r#"
        {

             "$meta": {
                "advisory": "PyUp.io metadata",
                "timestamp": 1601532001
             }, 
             "acqusition": [
                {
                    "advisory": "acqusition is a package affected by pytosquatting: http://www.nbu.gov.sk/skcsirt-sa-20170909-pypi/",
                    "cve": null,
                    "id": "pyup.io-34978",
                    "specs": [
                        ">0",
                        "<0"
                    ],
                    "v": ">0,<0"
                }
            ], 
            "aegea": [
                {
                    "advisory": "Aegea 2.2.7 avoids CVE-2018-1000805.",
                    "cve": "CVE-2018-1000805",
                    "id": "pyup.io-37611",
                    "specs": [
                        "<2.2.7"
                    ],
                    "v": "<2.2.7"
                }
            ]
        }
        "#;

        let python_safety_vuln_info_json: HashMap<String, SafetyJsonData> =
            serde_json::from_str(serde_json_data.as_ref()).unwrap();
        for (k, _v) in python_safety_vuln_info_json {
            println!("Keys = {}", k)
        }
    }

    #[test]
    fn parse_python_safety_vulns() {
        let python_safety_vulns = r#"[
    [
        "rsa",
        "<4.3",
        "3.4.2",
        "Rsa 4.3 includes two security fixes:\r\n- Choose blinding factor relatively prime to N.\r\n- Reject cyphertexts (when decrypting) and signatures (when verifying) that have  been modified by prepending zero bytes. This resolves CVE-2020-13757.",
        "38414"
    ],
    [
        "pyyaml",
        "<5.3.1",
        "5.1.2",
        "A vulnerability was discovered in the PyYAML library in versions before 5.3.1, where it is susceptible to arbitrary code execution when it processes untrusted YAML files through the full_load method or with the FullLoader loader. Applications that use the library to process untrusted input may be vulnerable to this flaw. An attacker could use this flaw to execute arbitrary code on the system by abusing the python/object/new constructor. See: CVE-2020-1747.",
        "38100"
    ]]"#;

        let compr_adv_text = ComprString::new("Some advsiory text");

        let safety_cve_map: HashMap<String, Option<String>> = [(
            "pyup.io-38414".to_string(),
            Some("CVE-2020-13757".to_string()),
        )]
        .iter()
        .cloned()
        .collect();

        let vulnshash: HashMap<String, VulnData> = [(
            "CVE-2020-13757".to_string(),
            VulnData {
                advisory_str: compr_adv_text,
                advisory_url: "http://someurl.co.uk".to_string(),
                cvss: Cvss::builder()
                    .with_version(CvssVersion::V2)
                    .with_score(Some(7.5))
                    .build()
                    .unwrap(),
            },
        )]
        .iter()
        .cloned()
        .collect();
        let test_report = ToolReport {
            event_version: EventVersion::try_from("1".to_owned()).unwrap(),
            event_id: EventID::try_from("95130bee-95ae-4dac-aecf-5650ff646ea1".to_owned()).unwrap(),
            application_name: ApplicationName::try_from("Test application".to_owned()).unwrap(),
            git_branch: GitBranch::try_from(Some("git".to_owned())).unwrap(),
            git_commit_hash: GitCommitHash::try_from(
                "e99f715d0fe787cd43de967b8a79b56960fed3e5".to_owned(),
            )
            .unwrap(),
            tool_name: ToolName::try_from("safety".to_owned()).unwrap(),
            tool_output: ToolOutput::try_from(python_safety_vulns.to_owned()).unwrap(),
            output_format: OutputFormat::JSON,
            start_time: StartTime::from(DateTime::<Utc>::from(
                DateTime::parse_from_rfc3339("2019-09-13T19:35:38+00:00").unwrap(),
            )),
            end_time: EndTime::from(DateTime::<Utc>::from(
                DateTime::parse_from_rfc3339("2019-09-13T19:37:14+00:00").unwrap(),
            )),
            environment: Environment::Local,
            tool_version: ToolVersion::try_from(Some("1.0".to_owned())).unwrap(),
            suppressed_issues: vec![],
        };
        let events = parse_safety_json(&test_report, &vulnshash, &safety_cve_map);
        for event in &events.unwrap() {
            println!(
                "affected package: {}, installed version: {}, cvss: {}, url: {},  advisory: {}\n",
                event.affected_package,
                event.installed_version,
                event.cvss,
                event.advisory_url,
                event.advisory_description
            )
        }
    }
}
