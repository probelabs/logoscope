use chrono::{DateTime, Datelike, NaiveDateTime, TimeZone, Utc};
use itertools::Itertools;
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogFormat {
    Json,
    Plaintext,
}

#[derive(Debug, Clone)]
pub struct ParsedRecord {
    pub format: LogFormat,
    pub line_number: usize,
    pub message: String,
    pub timestamp: Option<DateTime<Utc>>, // extracted or None
    pub flat_fields: Option<BTreeMap<String, String>>, // for JSON
    pub synthetic_message: Option<String>,             // for JSON derived message
}

pub fn parse_line(line: &str, line_number: usize) -> ParsedRecord {
    parse_line_with_hints(line, line_number, &[])
}

pub fn parse_line_with_hints(line: &str, line_number: usize, time_keys: &[&str]) -> ParsedRecord {
    match serde_json::from_str::<Value>(line) {
        Ok(Value::Object(map)) => {
            let mut flat = BTreeMap::new();
            flatten_json("", &Value::Object(map), &mut flat);

            let message = line.trim_end().to_string();

            // timestamp extraction: prioritized by hints, then scan all
            let mut ts: Option<DateTime<Utc>> = None;
            for key in time_keys {
                if let Some(val) = flat.get(*key) {
                    if let Some(t) = parse_ts_candidate(val) { ts = Some(t); break; }
                }
            }
            if ts.is_none() {
                for (_k, v) in flat.iter() {
                    if let Some(t) = parse_ts_candidate(v) { ts = Some(t); break; }
                }
            }

            // Build synthetic message: stable key order key=value
            let synthetic = Some(
                flat.iter()
                    .map(|(k, v)| format!("{}={}", k, v))
                    .join(" "),
            );

            ParsedRecord {
                format: LogFormat::Json,
                line_number,
                message,
                timestamp: ts,
                flat_fields: Some(flat),
                synthetic_message: synthetic,
            }
        }
        _ => {
            let message = line.trim_end().to_string();
            let timestamp = detect_timestamp_in_text(&message);
            ParsedRecord {
                format: LogFormat::Plaintext,
                line_number,
                message,
                timestamp,
                flat_fields: None,
                synthetic_message: None,
            }
        }
    }
}

fn flatten_json(prefix: &str, v: &Value, out: &mut BTreeMap<String, String>) {
    match v {
        Value::Object(map) => {
            for (k, v) in map.iter() {
                let key = if prefix.is_empty() {
                    k.clone()
                } else {
                    format!("{}.{}", prefix, k)
                };
                flatten_json(&key, v, out);
            }
        }
        Value::Array(arr) => {
            for (idx, item) in arr.iter().enumerate() {
                let key = if prefix.is_empty() {
                    idx.to_string()
                } else {
                    format!("{}.{}", prefix, idx)
                };
                flatten_json(&key, item, out);
            }
        }
        Value::Null => {
            out.insert(prefix.to_string(), "null".to_string());
        }
        Value::Bool(b) => {
            out.insert(prefix.to_string(), b.to_string());
        }
        Value::Number(n) => {
            out.insert(prefix.to_string(), n.to_string());
        }
        Value::String(s) => {
            out.insert(prefix.to_string(), s.clone());
        }
    }
}

fn parse_ts_candidate(s: &str) -> Option<DateTime<Utc>> {
    parse_ts_string(s).or_else(|| parse_ts_number_string(s))
}

fn parse_ts_string(s: &str) -> Option<DateTime<Utc>> {
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Some(dt.with_timezone(&Utc));
    }
    // Try common formats, with/without timezone
    let fmts = [
        "%Y-%m-%d %H:%M:%S%.f%:z",
        "%Y-%m-%d %H:%M:%S%:z",
        "%Y-%m-%dT%H:%M:%S%.f%:z",
        "%Y-%m-%dT%H:%M:%S%:z",
        "%Y-%m-%d %H:%M:%S%.f",
        "%Y-%m-%d %H:%M:%S",
        "%Y/%m/%d %H:%M:%S",
    ];
    for f in fmts.iter() {
        if let Ok(ndt) = NaiveDateTime::parse_from_str(s, f) {
            return Some(Utc.from_utc_datetime(&ndt));
        }
    }
    None
}

fn parse_ts_number_string(s: &str) -> Option<DateTime<Utc>> {
    let digits_only = s.chars().all(|c| c.is_ascii_digit());
    if !digits_only { return None; }
    match s.len() {
        10 => s.parse::<i64>().ok().and_then(epoch_secs_to_dt),
        13 => s.parse::<i64>().ok().and_then(epoch_millis_to_dt),
        16 => s.parse::<i64>().ok().and_then(epoch_micros_to_dt),
        _ => None,
    }
}

fn epoch_secs_to_dt(sec: i64) -> Option<DateTime<Utc>> {
    Some(DateTime::<Utc>::from(std::time::UNIX_EPOCH + std::time::Duration::from_secs(sec as u64)))
}
fn epoch_millis_to_dt(ms: i64) -> Option<DateTime<Utc>> {
    let secs = (ms / 1000) as u64;
    let nsub = ((ms % 1000).abs() as u32) * 1_000_000;
    DateTime::<Utc>::from_timestamp(secs as i64, nsub)
}
fn epoch_micros_to_dt(us: i64) -> Option<DateTime<Utc>> {
    let secs = (us / 1_000_000) as i64;
    let nsub = ((us % 1_000_000).abs() as u32) * 1_000;
    DateTime::<Utc>::from_timestamp(secs, nsub)
}

pub fn detect_timestamp_in_text(s: &str) -> Option<DateTime<Utc>> {
    // Try ISO8601/RFC3339 substring with optional timezone or Z
    static RE_ISO_ANY: once_cell::sync::Lazy<regex::Regex> = once_cell::sync::Lazy::new(|| {
        regex::Regex::new(r"\b\d{4}-\d{2}-\d{2}[ T]\d{2}:\d{2}:\d{2}(?:\.\d+)?(?:Z|[+-]\d{2}:\d{2})?\b").unwrap()
    });
    if let Some(m) = RE_ISO_ANY.find(s) {
        let mstr = m.as_str();
        if let Ok(dt) = DateTime::parse_from_rfc3339(mstr) {
            return Some(dt.with_timezone(&Utc));
        }
        // Try naive as UTC
        if let Ok(ndt) = NaiveDateTime::parse_from_str(mstr, "%Y-%m-%d %H:%M:%S")
            .or_else(|_| NaiveDateTime::parse_from_str(mstr, "%Y-%m-%d %H:%M:%S%.f"))
            .or_else(|_| NaiveDateTime::parse_from_str(mstr, "%Y-%m-%dT%H:%M:%S"))
            .or_else(|_| NaiveDateTime::parse_from_str(mstr, "%Y-%m-%dT%H:%M:%S%.f"))
        {
            return Some(Utc.from_utc_datetime(&ndt));
        }
    }
    // Try syslog: `Sep 05 14:20:00`
    static RE_SYSLOG: once_cell::sync::Lazy<regex::Regex> = once_cell::sync::Lazy::new(|| {
        regex::Regex::new(r"\b(Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec)\s+\d{1,2}\s+\d{2}:\d{2}:\d{2}\b").unwrap()
    });
    if let Some(m) = RE_SYSLOG.find(s) {
        let year = Utc::now().year();
        let candidate = format!("{} {}", year, m.as_str());
        if let Ok(naive) = chrono::NaiveDateTime::parse_from_str(&candidate, "%Y %b %d %H:%M:%S") {
            return Some(Utc.from_utc_datetime(&naive));
        }
    }
    // Try epoch (10s/13ms/16us)
    static RE_EPOCH10: once_cell::sync::Lazy<regex::Regex> = once_cell::sync::Lazy::new(|| {
        regex::Regex::new(r"\b\d{10}\b").unwrap()
    });
    static RE_EPOCH13: once_cell::sync::Lazy<regex::Regex> = once_cell::sync::Lazy::new(|| {
        regex::Regex::new(r"\b\d{13}\b").unwrap()
    });
    static RE_EPOCH16: once_cell::sync::Lazy<regex::Regex> = once_cell::sync::Lazy::new(|| {
        regex::Regex::new(r"\b\d{16}\b").unwrap()
    });
    if let Some(m) = RE_EPOCH16.find(s) {
        if let Ok(us) = m.as_str().parse::<i64>() { return epoch_micros_to_dt(us); }
    }
    if let Some(m) = RE_EPOCH13.find(s) {
        if let Ok(ms) = m.as_str().parse::<i64>() { return epoch_millis_to_dt(ms); }
    }
    if let Some(m) = RE_EPOCH10.find(s) {
        if let Ok(sec) = m.as_str().parse::<i64>() { return epoch_secs_to_dt(sec); }
    }
    None
}
