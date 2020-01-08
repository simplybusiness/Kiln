#[macro_use] extern crate lazy_static;
#[macro_use] extern crate log;

use avro_rs::{Reader, Schema, Writer};
use chrono::prelude::*;
use chrono::Duration;
use data_encoding::HEXUPPER;
use failure::err_msg;
use flate2::read::GzDecoder;
use iter_read::IterRead;
use kiln_lib::avro_schema::DEPENDENCY_EVENT_SCHEMA;
use kiln_lib::kafka::*;
use kiln_lib::dependency_event::{DependencyEvent, Timestamp, AdvisoryDescription, AdvisoryId, AdvisoryUrl, InstalledVersion, AffectedPackage, Cvss, CvssVersion};
use kiln_lib::tool_report::{ToolReport, EventVersion, EventID};
use regex::Regex;
use ring::digest;
use serde_json::Value;
use std::boxed::Box;
use std::collections::HashMap;
use std::convert::TryFrom;
use std::env;
use std::error::Error;
use std::io::Read;
use std::str::FromStr;
use url::Url;
use uuid::Uuid;

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    openssl_probe::init_ssl_cert_env_vars();
    let config = get_bootstrap_config(&mut env::vars())
        .map_err(|err| failure::err_msg(format!("Configuration Error: {}", err)))?;

    let ssl_connector = build_ssl_connector().map_err(|err| {
        failure::err_msg(format!(
            "OpenSSL Error {}: {}",
            err.errors()[0].code(),
            err.errors()[0].reason().unwrap()
        ))
    })?;

    let mut consumer = build_kafka_consumer(
        config.clone(),
        "ToolReports".to_string(),
        "report-parser".to_string(),
        ssl_connector.clone(),
    )
        .map_err(|err| err_msg(format!("Kafka Consumer Error: {}", err.description())))?;

    let mut producer = build_kafka_producer(
        config.clone(),
        ssl_connector.clone(),
    )
        .map_err(|err| err_msg(format!("Kafka Producer Error: {}", err.description())))?;

    let base_url = Url::parse(&env::var("NVD_BASE_URL").unwrap_or("https://nvd.nist.gov/feeds/json/cve/1.1/".to_string()))?;
    let mut last_updated_time = None;

    let mut vulns = HashMap::new();
    for year in 2002..=2020 {
        let parsed_vulns = download_and_parse_vulns(year.to_string(), last_updated_time, &base_url);
        if let Err(err) = parsed_vulns {
            error!("{}", err);
            return Err(err)
        } else {
            parsed_vulns
            .into_iter()
            .fold(&mut vulns, |acc, values| {
                if values.is_some() {
                    for (k, v) in values.unwrap().drain() {
                        acc.insert(k, v);
                    }
                }
                acc
            });
            info!("Successfully got vulns for {}", year);
        }
    }

    let modified_vulns = download_and_parse_vulns("modified".to_string(), last_updated_time, &base_url);
    if let Err(err) = modified_vulns {
        error!("{}", err);
        return Err(err)
    } else {
        modified_vulns
        .into_iter()
        .fold(&mut vulns, |acc, values| {
            if values.is_some() {
                for (k, v) in values.unwrap().drain() {
                    acc.insert(k, v);
                }
            }
            acc
        });
        info!("Successfully got latest vulns");
    }

    last_updated_time = Some(Utc::now());

    loop {
        if last_updated_time.unwrap().lt(&(Utc::now() - Duration::hours(2))) {
            let modified_vulns = download_and_parse_vulns("modified".to_string(), last_updated_time, &base_url);
            if let Err(err) = modified_vulns {
                error!("{}", err);
            } else {
                if let Some(mut modified_vulns) = modified_vulns.unwrap() {
                    for (k, v) in modified_vulns.drain() {
                        vulns.insert(k, v);
                    }
                    info!("Successfully got new vuln information"); 
                }
                last_updated_time = Some(Utc::now());
            }
        }

        for ms in consumer.poll().unwrap().iter() {
            for m in ms.messages() {
                let reader = Reader::new(m.value)?;
                for value in reader {
                    let report = ToolReport::try_from(value?)?;
                    let records = parse_tool_report(&report, &vulns)?;
                    for record in records.into_iter() {
                        producer
                            .send(&kafka::producer::Record::from_value(
                                "DependencyEvents",
                                record,
                            ))
                            .map_err(|err| err_msg(format!("Error publishing to Kafka: {}", err.to_string())))?;
                    }
                }
            }
            consumer.consume_messageset(ms)?;
        }
        consumer.commit_consumed()?;
    }
}

fn download_and_parse_vulns(index: String, last_updated_time: Option<DateTime<Utc>>, base_url: &Url) -> Result<Option<HashMap<String, Cvss>>, Box<dyn Error>> {
    lazy_static! {
        static ref META_LAST_MOD_RE: Regex = Regex::new("lastModifiedDate:(.*)\r\n").unwrap();
        static ref META_COMPRESSED_GZ_SIZE_RE: Regex = Regex::new("gzSize:(.*)\r\n").unwrap();
        static ref META_UNCOMPRESSED_SIZE_RE: Regex = Regex::new("size:(.*)\r\n").unwrap();
        static ref META_SHA256_RE: Regex = Regex::new("sha256:(.*)\r\n").unwrap();
    }

    let meta_filename = format!("nvdcve-1.1-{}.meta", index);
    let meta_url = base_url.join(&meta_filename)?;

    let meta_resp_text = reqwest::blocking::get(meta_url)
        .map_err(|err| Box::new(err_msg(format!("Error downloading {}: {}", meta_filename, err)).compat()))
        .and_then(|resp| resp.text()
            .map_err(|err| Box::new(err_msg(format!("Error reading body of {}: {}", meta_filename, err)).compat()))
        )?;

    let last_mod_timestamp = META_LAST_MOD_RE.captures(&meta_resp_text)
        .and_then(|captures| captures.get(1))
        .ok_or(Box::new(err_msg(format!("Error reading lastModifiedDate from {}", meta_filename)).compat()))
        .map(|capture| capture.as_str())?;

    let uncompressed_size = META_UNCOMPRESSED_SIZE_RE.captures(&meta_resp_text)
        .and_then(|captures| captures.get(1))
        .ok_or(Box::new(err_msg(format!("Error reading size from {}", meta_filename)).compat()))
        .map(|capture| capture.as_str())?;

    let compressed_size = META_COMPRESSED_GZ_SIZE_RE.captures(&meta_resp_text)
        .and_then(|captures| captures.get(1))
        .ok_or(Box::new(err_msg(format!("Error reading compressed size from {}", meta_filename)).compat()))
        .map(|capture| capture.as_str())?;

    let hash = META_SHA256_RE.captures(&meta_resp_text)
        .and_then(|captures| captures.get(1))
        .ok_or(Box::new(err_msg(format!("Error reading sha256 hash from {}", meta_filename)).compat()))
        .map(|capture| capture.as_str())?;

    if last_updated_time.is_none() || last_updated_time.unwrap().lt(&DateTime::parse_from_rfc3339(last_mod_timestamp)?.with_timezone(&Utc)) {
        let data_filename = format!("nvdcve-1.1-{}.json.gz", index);
        let data_url = base_url.join(&data_filename)?;

        let mut resp = reqwest::blocking::get(data_url)
            .map_err(|err| Box::new(err_msg(format!("Error downloading {}: {}", data_filename, err)).compat()))?;

        let mut resp_compressed_bytes = Vec::<u8>::with_capacity(usize::from_str(compressed_size)?);
        resp.read_to_end(&mut resp_compressed_bytes)
            .map_err(|err| Box::new(err_msg(format!("Error reading {} ({})", data_filename, err)).compat()))?;

        let mut uncompressed_bytes = Vec::<u8>::with_capacity(usize::from_str(uncompressed_size)?);
        let mut gz = GzDecoder::new(IterRead::new(resp_compressed_bytes.iter()));
        gz.read_to_end(&mut uncompressed_bytes)
            .map_err(|err| Box::new(err_msg(format!("Error decompressing {} ({})", data_filename, err)).compat()))?;

        let computed_hash = HEXUPPER.encode(digest::digest(&digest::SHA256, &uncompressed_bytes).as_ref());

        if hash != computed_hash {
            return Err(Box::new(err_msg(format!("Hash mismatch for {}, expected {}, got {}", data_filename, hash, computed_hash)).compat()));
        }

        let parsed_json: Value = serde_json::from_slice(&uncompressed_bytes)?;

        let cve_items = parsed_json["CVE_Items"]
            .as_array()
            .unwrap()
            .iter()
            .map(|vuln_info| {
                let v3_score = vuln_info.get("impact")
                    .and_then(|impact| impact.get("baseMetricV3"))
                    .and_then(|base_metric_v3| base_metric_v3.get("cvssV3"))
                    .and_then(|cvss| cvss["baseScore"].as_f64());

                let v2_score = vuln_info.get("impact")
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
                    Cvss::builder()
                    .with_version(CvssVersion::Unknown)
                    .build()
                };

                return (vuln_info["cve"]["CVE_data_meta"]["ID"].as_str().unwrap().to_string(), cvss.unwrap());
            })
            .collect::<HashMap<_, _>>();

        return Ok(Some(cve_items));
    }

    Ok(None)
}

fn parse_tool_report(report: &ToolReport, vulns: &HashMap<String, Cvss>) -> Result<Vec<Vec<u8>>, Box<dyn Error>> {
    let events = if report.tool_name == "bundler-audit" {
        if report.output_format == "PlainText" {
            parse_bundler_audit_plaintext(&report, &vulns)
        } else {
            Err(Box::new(err_msg(format!("Unknown output format for Bundler-audit in ToolReport: {:?}", report)).compat()).into())
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

fn parse_bundler_audit_plaintext(report: &ToolReport, vulns: &HashMap<String, Cvss>) -> Result<Vec<DependencyEvent>, Box<dyn Error>> {
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
        let advisory_id = AdvisoryId::try_from(fields.get("Advisory").or(Some(&"".to_string())).unwrap().to_owned())?;

        let default_cvss = Cvss::builder()
                .with_version(CvssVersion::Unknown)
                .build()
                .unwrap();

        let cvss = vulns.get(&advisory_id.to_string())
            .unwrap_or(&default_cvss);

        let event = DependencyEvent {
            event_version: EventVersion::try_from("1".to_string())?,
            event_id: EventID::try_from(Uuid::new_v4().to_hyphenated().to_string())?,
            parent_event_id: report.event_id.clone(),
            application_name: report.application_name.clone(),
            git_branch: report.git_branch.clone(),
            git_commit_hash: report.git_commit_hash.clone(),
            timestamp: Timestamp::try_from(report.end_time.to_string())?,
            affected_package: AffectedPackage::try_from(fields.get("Name").or(Some(&"".to_string())).unwrap().to_owned())?,
            installed_version: InstalledVersion::try_from(fields.get("Version").or(Some(&"".to_string())).unwrap().to_owned())?,
            advisory_url: AdvisoryUrl::try_from(fields.get("URL").or(Some(&"".to_string())).unwrap().to_owned())?,
            advisory_id,
            advisory_description: AdvisoryDescription::try_from(fields.get("Title").or(Some(&"".to_string())).unwrap().to_owned())?,
            cvss: cvss.clone(),
       };
       events.push(event);
    }
    Ok(events)
}
