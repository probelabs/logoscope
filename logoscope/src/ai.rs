use crate::{anomaly, schema, temporal, masking, parser, correlation};
use serde::{Serialize, Deserialize};
use rayon::prelude::*;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Summary {
    pub total_lines: usize,
    pub unique_patterns: usize,
    pub compression_ratio: f64,
    pub time_span: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiOutput {
    pub summary: Summary,
    pub patterns: Vec<PatternOut>,
    pub schema_changes: Vec<SchemaChangeOut>,
    pub anomalies: AnomaliesOut,
    pub query_interface: QueryInterfaceOut,
    pub errors: ErrorsOut,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternOut {
    pub template: String,
    pub frequency: f64,
    pub total_count: usize,
    pub severity: Option<String>,
    pub temporal: TemporalOut,
    pub examples: Vec<String>,
    pub correlations: Vec<CorrelatedOut>,
    pub confidence: f64,
    pub sources: SourceBreakdown,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TemporalOut {
    pub bursts: usize,
    pub largest_burst: Option<String>,
    pub trend: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorrelatedOut {
    pub template: String,
    pub count: usize,
    pub strength: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaChangeOut {
    pub timestamp: Option<String>,
    pub change_type: String,
    pub field: String,
    pub impact: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AnomaliesOut {
    pub pattern_anomalies: Vec<PatternAnomalyOut>,
    pub field_anomalies: Vec<String>,
    pub temporal_anomalies: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternAnomalyOut {
    pub kind: String,
    pub template: String,
    pub frequency: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryInterfaceOut {
    pub available_commands: Vec<String>,
    pub suggested_investigations: Vec<SuggestionOut>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestionOut {
    pub priority: String,
    pub description: String,
    pub query: SuggestQuery,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestQuery {
    pub command: String,
    pub params: SuggestParams,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SuggestParams {
    pub start: Option<String>,
    pub end: Option<String>,
    pub pattern: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ErrorsOut {
    pub total: usize,
    pub samples: Vec<ErrorSample>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorSample {
    pub line_number: usize,
    pub kind: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SourceBreakdown {
    pub by_service: Vec<CountItem>,
    pub by_host: Vec<CountItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CountItem {
    pub name: String,
    pub count: usize,
}

use std::collections::HashSet;

pub fn summarize_lines(lines: &[&str]) -> AiOutput {
    summarize_impl(lines, &[], None)
}

pub fn summarize_lines_with_hints<'a>(lines: &[&'a str], time_keys: &[&'a str]) -> AiOutput {
    summarize_impl(lines, time_keys, None)
}

pub fn summarize_lines_with_baseline<'a>(lines: &[&'a str], baseline_templates: &HashSet<String>) -> AiOutput {
    summarize_impl(lines, &[], Some(baseline_templates))
}

fn summarize_impl<'a>(lines: &[&'a str], time_keys: &[&'a str], baseline_opt: Option<&HashSet<String>>) -> AiOutput {
    let total = lines.len();
    let mut min_ts: Option<chrono::DateTime<chrono::Utc>> = None;
    let mut max_ts: Option<chrono::DateTime<chrono::Utc>> = None;
    // Collect per-line data (parallel)
    #[derive(Clone)]
    struct LineDeriv {
        message: String,
        timestamp: Option<chrono::DateTime<chrono::Utc>>,
        template: String,
        level: Option<String>,
        service: Option<String>,
        host: Option<String>,
        malformed_json: bool,
        fingerprint: Option<schema::Fingerprint>,
    }

    let derived: Vec<LineDeriv> = lines
        .par_iter()
        .enumerate()
        .map(|(i, l)| {
            let looks_json = l.trim_start().starts_with('{') || l.trim_start().starts_with('[');
            let rec = if time_keys.is_empty() { parser::parse_line(l, i + 1) } else { parser::parse_line_with_hints(l, i + 1, time_keys) };
            let malformed_json = looks_json && rec.flat_fields.is_none();
            // Build template base: for JSON, drop high-cardinality source keys
            let base = if let Some(ff) = rec.flat_fields.as_ref() {
                let mut items: Vec<(String,String)> = ff.iter().map(|(k,v)| (k.clone(), v.clone())).collect();
                items.sort_by(|a,b| a.0.cmp(&b.0));
                let drop_key = |k: &str| {
                    k == "host" || k == "hostname" || k == "service" ||
                    k.starts_with("kubernetes.") || k == "pod" || k == "namespace" || k == "container" || k == "container_id"
                };
                let s = items.into_iter()
                    .filter(|(k,_)| !drop_key(k))
                    .map(|(k,v)| format!("{}={}", k, v))
                    .collect::<Vec<String>>().join(" ");
                if s.is_empty() { rec.message.clone() } else { s }
            } else {
                rec.message.clone()
            };
            let masked = masking::mask_text(&base);
            let template = to_generic_template(&masked);
            let level = rec.flat_fields.as_ref().and_then(|f| f.get("level").cloned());
            let (service_opt, host_opt) = extract_source(&rec, &rec.message);
            let fingerprint = if rec.flat_fields.is_some() {
                if let Some(rv) = rec.raw_json.as_ref() {
                    Some(schema::fingerprint_value(rv))
                } else {
                    serde_json::from_str::<serde_json::Value>(&rec.message)
                        .ok()
                        .map(|v| schema::fingerprint_value(&v))
                }
            } else { None };

            LineDeriv { message: rec.message, timestamp: rec.timestamp, template, level, service: service_opt, host: host_opt, malformed_json, fingerprint }
        })
        .collect();

    // Combine derived data
    let mut messages: Vec<String> = Vec::with_capacity(total);
    let mut timestamps: Vec<Option<chrono::DateTime<chrono::Utc>>> = Vec::with_capacity(total);
    let mut levels: Vec<Option<String>> = Vec::with_capacity(total);
    let mut templates: Vec<String> = Vec::with_capacity(total);
    let mut json_fps: Vec<(usize, schema::Fingerprint, Option<chrono::DateTime<chrono::Utc>>)> = Vec::new();
    let mut error_samples: Vec<ErrorSample> = Vec::new();
    let mut service_by_tpl: HashMap<String, HashMap<String, usize>> = HashMap::new();
    let mut host_by_tpl: HashMap<String, HashMap<String, usize>> = HashMap::new();
    for (i, d) in derived.into_iter().enumerate() {
        if let Some(ts) = d.timestamp {
            min_ts = Some(match min_ts { Some(m) => m.min(ts), None => ts });
            max_ts = Some(match max_ts { Some(m) => m.max(ts), None => ts });
        }
        if d.malformed_json && error_samples.len() < 10 {
            error_samples.push(ErrorSample { line_number: i + 1, kind: "malformed_json".into() });
        }
        if let Some(fp) = d.fingerprint { json_fps.push((i, fp, d.timestamp)); }
        if let Some(svc) = d.service.clone() {
            *service_by_tpl.entry(d.template.clone()).or_default().entry(svc).or_insert(0) += 1;
        }
        if let Some(h) = d.host.clone() {
            *host_by_tpl.entry(d.template.clone()).or_default().entry(h).or_insert(0) += 1;
        }
        messages.push(d.message);
        timestamps.push(d.timestamp);
        levels.push(d.level);
        templates.push(d.template);
    }
// Cluster by template
    let mut counts: HashMap<String, usize> = HashMap::new();
    let mut idxs_by_tpl: HashMap<String, Vec<usize>> = HashMap::new();
    let mut times_by_tpl: HashMap<String, Vec<chrono::DateTime<chrono::Utc>>> = HashMap::new();
    for (i, tpl) in templates.iter().enumerate() {
        *counts.entry(tpl.clone()).or_insert(0) += 1;
        idxs_by_tpl.entry(tpl.clone()).or_default().push(i);
        if let Some(ts) = timestamps[i] {
            times_by_tpl.entry(tpl.clone()).or_default().push(ts);
        }
    }
    let unique = counts.len();
    let compression_ratio = if unique > 0 {
        (total as f64) / (unique as f64)
    } else {
        0.0
    };
    let time_span = match (min_ts, max_ts) {
        (Some(a), Some(b)) => Some(format!("{} to {}", a.to_rfc3339_opts(chrono::SecondsFormat::Secs, true), b.to_rfc3339_opts(chrono::SecondsFormat::Secs, true))),
        _ => None,
    };
    // Patterns output
    let mut patterns = Vec::new();
    let mut suggestions: Vec<SuggestionOut> = Vec::new();
    // Precompute correlations
    let cors = correlation::compute_correlations(&times_by_tpl, chrono::Duration::seconds(10));
    for (tpl, cnt) in counts.iter() {
        let idxs = idxs_by_tpl.get(tpl).unwrap();
        // severity = most frequent level
        let mut lvl_counts: HashMap<String, usize> = HashMap::new();
        let mut ts_for_tpl: Vec<chrono::DateTime<chrono::Utc>> = Vec::new();
        let mut exs: Vec<String> = Vec::new();
        for &i in idxs.iter() {
            if let Some(lv) = levels[i].as_ref() { *lvl_counts.entry(lv.clone()).or_insert(0) += 1; }
            if let Some(ts) = timestamps[i] { ts_for_tpl.push(ts); }
            if exs.len() < 3 { exs.push(messages[i].clone()); }
        }
        let severity = lvl_counts.into_iter().max_by_key(|(_, c)| *c).map(|(l, _)| l);
        let bursts = temporal::compute_bursts(&ts_for_tpl, chrono::Duration::minutes(1), 3.0);
        let largest_burst = bursts.iter().max_by_key(|b| b.peak_rate).map(|b| b.start_time.to_rfc3339_opts(chrono::SecondsFormat::Secs, true));
        let trend = trend_label(&ts_for_tpl);
        // Correlations for this tpl
        let mut related: Vec<CorrelatedOut> = cors
            .iter()
            .filter_map(|c| {
                if c.a == *tpl { Some(CorrelatedOut { template: c.b.clone(), count: c.count, strength: c.strength }) }
                else if c.b == *tpl { Some(CorrelatedOut { template: c.a.clone(), count: c.count, strength: c.strength }) }
                else { None }
            })
            .collect();
        related.sort_by(|x, y| y.strength.partial_cmp(&x.strength).unwrap());
        if related.len() > 3 { related.truncate(3); }

        // Confidence score (0..1): presence across time + frequency factor
        let presence = if !ts_for_tpl.is_empty() && min_ts.is_some() && max_ts.is_some() {
            let a = min_ts.unwrap(); let b = max_ts.unwrap();
            let span_secs = (b - a).num_seconds().abs().max(60) as f64;
            let unique_minutes = ts_for_tpl.iter().map(|t| (t.timestamp()/60)).collect::<std::collections::BTreeSet<_>>().len() as f64;
            (unique_minutes * 60.0 / span_secs).min(1.0)
        } else { 0.0 };
        let freq_factor = ((*cnt as f64) / (total as f64)).sqrt().min(1.0);
        let confidence = (presence * 0.6) + (freq_factor * 0.4);

        // sources breakdown top3
        let mut svc_items: Vec<CountItem> = service_by_tpl.get(tpl).map(|m| m.iter().map(|(k,v)| CountItem { name: k.clone(), count: *v }).collect()).unwrap_or_else(Vec::new);
        svc_items.sort_by(|a,b| b.count.cmp(&a.count).then_with(|| a.name.cmp(&b.name)));
        if svc_items.len() > 3 { svc_items.truncate(3); }
        let mut host_items: Vec<CountItem> = host_by_tpl.get(tpl).map(|m| m.iter().map(|(k,v)| CountItem { name: k.clone(), count: *v }).collect()).unwrap_or_else(Vec::new);
        host_items.sort_by(|a,b| b.count.cmp(&a.count).then_with(|| a.name.cmp(&b.name)));
        if host_items.len() > 3 { host_items.truncate(3); }

        patterns.push(PatternOut {
            template: tpl.clone(),
            frequency: (*cnt as f64) / (total as f64),
            total_count: *cnt,
            severity,
            temporal: TemporalOut { bursts: bursts.len(), largest_burst, trend },
            examples: exs,
            correlations: related,
            confidence,
            sources: SourceBreakdown { by_service: svc_items, by_host: host_items },
        });
        // Suggestions from bursts
        if let Some(b) = bursts.iter().max_by_key(|b| b.peak_rate) {
            suggestions.push(SuggestionOut {
                priority: "HIGH".into(),
                description: format!("Pattern burst for '{}'", tpl),
                query: SuggestQuery {
                    command: "GET_LINES_BY_TIME".into(),
                    params: SuggestParams {
                        start: Some(b.start_time.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)),
                        end: Some(b.end_time.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)),
                        pattern: Some(tpl.clone()),
                    },
                },
            });
        }
    }
    // Schema changes (diff first and last JSON fingerprints)
    let mut schema_changes = Vec::new();
    if json_fps.len() >= 2 {
        let (_first_idx, first_fp, _) = &json_fps[0];
        let (_last_idx, last_fp, last_ts) = &json_fps[json_fps.len() - 1];
        let changes = schema::diff_fingerprints(first_fp, last_fp);
        for ch in changes {
            match ch {
                schema::SchemaChange::FieldAdded { field, .. } => {
                    schema_changes.push(SchemaChangeOut { timestamp: last_ts.map(|t| t.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)), change_type: "field_added".into(), field: field.clone(), impact: None });
                    if let Some(ts) = last_ts {
                        let start = (*ts - chrono::Duration::minutes(5)).to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
                        let end = (*ts + chrono::Duration::minutes(5)).to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
                        suggestions.push(SuggestionOut { priority: "MEDIUM".into(), description: format!("Schema field added: {}", field), query: SuggestQuery { command: "GET_LINES_BY_TIME".into(), params: SuggestParams { start: Some(start), end: Some(end), pattern: None } } });
                    }
                }
                schema::SchemaChange::FieldRemoved { field, .. } => {
                    schema_changes.push(SchemaChangeOut { timestamp: last_ts.map(|t| t.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)), change_type: "field_removed".into(), field: field.clone(), impact: None });
                    if let Some(ts) = last_ts {
                        let start = (*ts - chrono::Duration::minutes(5)).to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
                        let end = (*ts + chrono::Duration::minutes(5)).to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
                        suggestions.push(SuggestionOut { priority: "MEDIUM".into(), description: format!("Schema field removed: {}", field), query: SuggestQuery { command: "GET_LINES_BY_TIME".into(), params: SuggestParams { start: Some(start), end: Some(end), pattern: None } } });
                    }
                }
                schema::SchemaChange::TypeChanged { field, .. } => {
                    schema_changes.push(SchemaChangeOut { timestamp: last_ts.map(|t| t.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)), change_type: "type_changed".into(), field: field.clone(), impact: None });
                    if let Some(ts) = last_ts {
                        let start = (*ts - chrono::Duration::minutes(5)).to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
                        let end = (*ts + chrono::Duration::minutes(5)).to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
                        suggestions.push(SuggestionOut { priority: "MEDIUM".into(), description: format!("Schema type changed: {}", field), query: SuggestQuery { command: "GET_LINES_BY_TIME".into(), params: SuggestParams { start: Some(start), end: Some(end), pattern: None } } });
                    }
                }
            }
        }
    }
    // Pattern anomalies (new & rare) with default threshold (10%).
    // NewPattern is only emitted when a non-empty baseline is provided (e.g., streaming mode).
    let empty_baseline = std::collections::HashSet::<String>::new();
    let baseline_ref = baseline_opt.unwrap_or(&empty_baseline);
    let pattern_anoms = anomaly::detect_pattern_anomalies(&counts, total, baseline_ref, 0.1);
    let pattern_anomalies: Vec<PatternAnomalyOut> = pattern_anoms
        .into_iter()
        .map(|a| PatternAnomalyOut { kind: match a.kind { anomaly::AnomalyKind::NewPattern => "NewPattern".into(), anomaly::AnomalyKind::RarePattern => "RarePattern".into() }, template: a.template, frequency: a.frequency })
        .collect();
    // Field anomalies using robust numeric stats and categorical explosions
    let lines_refs: Vec<&str> = lines.iter().map(|s| *s).collect();
    let num_outliers = crate::field_anomaly::analyze_numeric_outliers(&lines_refs, 3.0);
    let cat_explosions = crate::field_anomaly::analyze_categorical_explosions(&lines_refs, 0.8, 10);
    let mut field_anomalies = Vec::new();
    for o in num_outliers {
        field_anomalies.push(format!("numeric_outlier field={} value={:.2} z={:.2} template={}", o.field, o.value, o.robust_z, o.template));
    }
    for e in cat_explosions {
        field_anomalies.push(format!("cardinality_explosion field={} unique={} total={} ratio={:.2} template={}", e.field, e.unique_count, e.total, e.ratio, e.template));
    }

    // Temporal anomalies: bursts and gaps summarized per template
    let mut temporal_anomalies = Vec::new();
    for (tpl, ts_list) in times_by_tpl.iter() {
        let bursts = temporal::compute_bursts(ts_list, chrono::Duration::minutes(1), 3.0);
        for b in bursts {
            temporal_anomalies.push(format!("burst template={} start={} end={} peak={}", tpl, b.start_time.to_rfc3339_opts(chrono::SecondsFormat::Secs, true), b.end_time.to_rfc3339_opts(chrono::SecondsFormat::Secs, true), b.peak_rate));
        }
        let gaps = temporal::compute_gaps(ts_list, 10.0);
        for g in gaps {
            temporal_anomalies.push(format!("gap template={} start={} end={} dur_s={}", tpl, g.start_time.to_rfc3339_opts(chrono::SecondsFormat::Secs, true), g.end_time.to_rfc3339_opts(chrono::SecondsFormat::Secs, true), g.duration_seconds));
        }
    }

    let anomalies = AnomaliesOut { pattern_anomalies: pattern_anomalies.clone(), field_anomalies, temporal_anomalies };
    // Suggestions from anomalies
    for pa in pattern_anomalies.into_iter() {
        let priority = if pa.kind == "NewPattern" { "HIGH" } else { "LOW" };
        suggestions.push(SuggestionOut {
            priority: priority.into(),
            description: format!("{}: {}", pa.kind, pa.template),
            query: SuggestQuery { command: "GET_LINES_BY_PATTERN".into(), params: SuggestParams { start: None, end: None, pattern: Some(pa.template) } },
        });
    }

    // Deduplicate suggestions by query key, keeping the highest priority version
    let mut best: std::collections::HashMap<String, SuggestionOut> = std::collections::HashMap::new();
    fn prio_rank(p: &str) -> i32 { match p { "HIGH" => 3, "MEDIUM" => 2, _ => 1 } }
    for s in suggestions.into_iter() {
        let key = format!(
            "{}|{}|{}|{}",
            s.query.command,
            s.query.params.start.clone().unwrap_or_default(),
            s.query.params.end.clone().unwrap_or_default(),
            s.query.params.pattern.clone().unwrap_or_default()
        );
        if let Some(existing) = best.get(&key) {
            if prio_rank(&s.priority) <= prio_rank(&existing.priority) { continue; }
        }
        best.insert(key, s);
    }
    let mut deduped: Vec<SuggestionOut> = best.into_values().collect();
    deduped.sort_by(|a,b| prio_rank(&b.priority).cmp(&prio_rank(&a.priority)));

    let query_interface = QueryInterfaceOut {
        available_commands: vec!["GET_LINES_BY_PATTERN".into(), "GET_LINES_BY_TIME".into(), "GET_CONTEXT".into()],
        suggested_investigations: deduped,
    };

    AiOutput {
        summary: Summary { total_lines: total, unique_patterns: unique, compression_ratio, time_span },
        patterns,
        schema_changes,
        anomalies,
        query_interface,
        errors: ErrorsOut { total: error_samples.len(), samples: error_samples },
    }
}

fn to_generic_template(masked: &str) -> String {
    masked
        .replace("<NUM>", "<*>")
        .replace("<IP>", "<*>")
        .replace("<EMAIL>", "<*>")
        .replace("<TIMESTAMP>", "<*>")
}

fn trend_label(ts: &[chrono::DateTime<chrono::Utc>]) -> Option<String> {
    if ts.len() < 4 { return None; }
    let mut v = ts.to_vec();
    v.sort_unstable();
    let mid = v.len()/2;
    let first = &v[..mid];
    let second = &v[mid..];
    if second.len() == 0 { return None; }
    let rate1 = first.len() as f64 / ((first.last()?.timestamp() - first.first()?.timestamp()).abs().max(1) as f64);
    let rate2 = second.len() as f64 / ((second.last()?.timestamp() - second.first()?.timestamp()).abs().max(1) as f64);
    if rate2 > rate1 { Some("increasing".into()) } else if rate2 < rate1 { Some("decreasing".into()) } else { Some("steady".into()) }
}

fn extract_source(rec: &parser::ParsedRecord, message: &str) -> (Option<String>, Option<String>) {
    // JSON preferred via flat_fields
    if let Some(f) = rec.flat_fields.as_ref() {
        let service_keys = [
            "service", "app", "application", "kubernetes.labels.app", "kubernetes.container_name",
        ];
        let host_keys = [
            "host", "hostname", "kubernetes.host", "kubernetes.node_name", "kubernetes.pod_name",
        ];
        for k in service_keys.iter() {
            if let Some(v) = f.get(*k) { return (Some(v.clone()), pick_host(f, &host_keys)); }
        }
        // host only
        return (None, pick_host(f, &host_keys));
    }
    // Plaintext: try syslog-like host after timestamp
    if let Some(h) = extract_host_from_plaintext(message) { return (None, Some(h)); }
    (None, None)
}

fn pick_host(f: &std::collections::BTreeMap<String,String>, keys: &[&str]) -> Option<String> { for k in keys { if let Some(v)=f.get(*k){ return Some(v.clone()); } } None }

fn extract_host_from_plaintext(line: &str) -> Option<String> {
    // Example: "Sep 05 10:00:00 host app[123]: msg" -> host
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() >= 4 {
        // naive: 3rd token is time, 4th is host
        // Accept month name as first token
        let months = ["Jan","Feb","Mar","Apr","May","Jun","Jul","Aug","Sep","Oct","Nov","Dec"]; 
        if months.contains(&parts[0]) && parts[2].contains(':') { return Some(parts[3].to_string()); }
    }
    None
}
