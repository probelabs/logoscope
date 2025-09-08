use crate::{anomaly, schema, temporal, parser, drain_adapter, param_extractor, analyzers};
use analyzers::Analyzer;
use chrono::TimeZone;
use serde::{Serialize, Deserialize};
use rayon::prelude::*;
use std::collections::HashMap;
use once_cell::sync::Lazy;

// Static regex for template humanization to avoid recompilation
static TEMPLATE_FIELD_PATTERN: Lazy<regex::Regex> = Lazy::new(|| {
    regex::Regex::new(r"([a-zA-Z_][a-zA-Z0-9_.-]*) = <[^>]*>").unwrap()
});

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Summary {
    pub total_lines: usize,
    pub unique_patterns: usize,
    pub compression_ratio: f64,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
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
    pub start_time: Option<String>,  // First occurrence of this pattern
    pub end_time: Option<String>,    // Last occurrence of this pattern
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spike_analysis: Option<SpikeAnalysis>,  // Optional spike detection
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temporal: Option<TemporalOut>,
    pub examples: Vec<String>,
    #[serde(skip)]
    pub correlations: Vec<CorrelatedOut>,
    pub pattern_stability: f64,  // Combined metric: time consistency (60%) + frequency (40%), range 0.0-1.0
    #[serde(skip)]
    pub sources: SourceBreakdown,
    #[serde(skip)]
    pub drain_template: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub param_stats: Option<std::collections::HashMap<String, ParamFieldStats>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameter_anomalies: Option<Vec<ParameterAnomaly>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deep_temporal: Option<DeepTemporalOut>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deep_correlations: Option<Vec<DeepCorrelation>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParamValueCount { pub value: String, pub count: usize }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpikeAnalysis {
    pub rate_per_minute: f64,
    pub spikes: Vec<Spike>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Spike {
    pub time: String,
    pub event_count: usize,
    pub severity: f64,  // How many times above average
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParamFieldStats {
    pub total: usize,
    pub cardinality: usize,
    pub values: Vec<ParamValueCount>,
    pub top_ratio: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterAnomaly {
    pub anomaly_type: String,
    pub param: String,
    pub value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ratio: Option<f64>,
    pub details: String,  // Human-readable explanation of the anomaly
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldAnomaly {
    pub anomaly_type: String,
    pub field: String,
    pub template: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub z_score: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unique_count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ratio: Option<f64>,
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
    pub field_anomalies: Vec<FieldAnomaly>,
    pub temporal_anomalies: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternAnomalyOut {
    pub kind: String,
    pub template: String,
    pub frequency: f64,
    pub count: usize,
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

// Deep analysis structures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeepTemporalOut {
    pub hourly_distribution: Vec<HourlyCount>,
    pub time_clustering: Vec<TimeCluster>,
    pub pattern_evolution: Vec<PatternEvolution>,
    pub burst_analysis: Vec<BurstDetail>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HourlyCount {
    pub hour: u32,  // 0-23
    pub count: usize,
    pub percentage: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeCluster {
    pub time_window: String,  // e.g., "09:00-12:00"
    pub pattern_count: usize,
    pub dominant_pattern: String,
    pub activity_score: f64,  // 0.0-1.0 representing activity intensity
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternEvolution {
    pub time_window: String,
    pub template_changes: Vec<String>,
    pub parameter_shifts: Vec<ParameterShift>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterShift {
    pub parameter: String,
    pub old_dominant_value: String,
    pub new_dominant_value: String,
    pub change_ratio: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BurstDetail {
    pub start_time: String,
    pub end_time: String,
    pub peak_rate: usize,
    pub total_events: usize,
    pub contributing_factors: Vec<String>,
    pub severity: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeepCorrelation {
    pub template_a: String,
    pub template_b: String,
    pub correlation_strength: f64,  // -1.0 to 1.0
    pub time_lag_seconds: i32,      // positive means B follows A
    pub co_occurrence_rate: f64,    // 0.0 to 1.0
    pub analysis_type: String,      // "temporal", "causal", "inverse"
}

// Triage mode structures for compact output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriageOutput {
    pub summary: TriageSummary,
    pub pattern_anomalies: Vec<TriagePattern>,
    pub field_anomalies: Vec<TriageFieldAnomaly>,
    pub insights: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriageSummary {
    pub total_lines: usize,
    pub error_lines: usize,
    pub burst_patterns: usize,
    pub anomaly_count: usize,
    pub time_range: Option<String>,
    pub status: String, // "CRITICAL", "WARNING", "NORMAL"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriagePattern {
    pub template: String,
    pub count: usize,
    pub severity: String,
    pub anomaly_type: Option<String>, // "burst", "new", "rare", etc.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anomaly_details: Option<Vec<String>>, // Array of anomaly detail strings
    #[serde(skip_serializing_if = "Option::is_none")]
    pub example: Option<String>, // Example log entry for this pattern
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriageFieldAnomaly {
    pub anomaly_type: String,
    pub field: String,
    pub description: String,
    pub impact: String, // "CRITICAL", "HIGH", "MEDIUM", "LOW"
    pub details: Vec<String>, // Detailed breakdown of the anomaly
}

use std::collections::HashSet;
use ahash::AHashMap;

#[derive(Clone, Copy, Default)]
pub struct SummarizeOpts {
    pub use_drain: bool,
    pub analyze_spikes: bool,
    pub verbose: bool,
    pub triage: bool,
    pub deep: bool,
}

/// Calculate pattern importance for verbose mode ordering
/// Returns a higher score for more important patterns
fn calculate_pattern_importance(pattern: &PatternOut) -> f64 {
    let mut importance = 0.0;
    
    // 1. Severity level (highest weight: 1000-4000 range)
    let severity_score = match pattern.severity.as_deref() {
        Some("error") | Some("ERROR") | Some("err") | Some("ERR") => 4000.0,
        Some("warn") | Some("WARN") | Some("warning") | Some("WARNING") => 3000.0,  
        Some("info") | Some("INFO") => 2000.0,
        Some("debug") | Some("DEBUG") => 1000.0,
        Some("trace") | Some("TRACE") => 500.0,
        _ => 1500.0, // Unknown/null severity defaults to between info and warn
    };
    importance += severity_score;
    
    // 2. Pattern stability (0-100 range, higher is more important within same severity)
    importance += pattern.pattern_stability * 100.0;
    
    // 3. Presence of anomalies or bursts (0-200 range)
    let mut anomaly_boost = 0.0;
    if pattern.parameter_anomalies.is_some() {
        anomaly_boost += 100.0;
    }
    if let Some(ref temporal) = pattern.temporal {
        if temporal.bursts > 0 {
            anomaly_boost += 50.0 + (temporal.bursts as f64 * 10.0); // More bursts = higher importance
        }
    }
    if pattern.spike_analysis.is_some() {
        anomaly_boost += 50.0;
    }
    importance += anomaly_boost;
    
    // 4. Frequency factor (0-50 range, more frequent = slightly more important within same severity)
    importance += pattern.frequency * 50.0;
    
    importance
}


pub fn summarize_lines(lines: &[&str]) -> AiOutput {
    summarize_impl(lines, &[], None, &SummarizeOpts::default())
}

pub fn summarize_lines_with_hints<'a>(lines: &[&'a str], time_keys: &[&'a str]) -> AiOutput {
    summarize_impl(lines, time_keys, None, &SummarizeOpts::default())
}

pub fn summarize_lines_with_baseline<'a>(lines: &[&'a str], baseline_templates: &HashSet<String>) -> AiOutput {
    summarize_impl(lines, &[], Some(baseline_templates), &SummarizeOpts::default())
}

pub fn summarize_lines_with_opts<'a>(
    lines: &[&'a str],
    time_keys: &[&'a str],
    baseline_templates: Option<&HashSet<String>>,
    opts: &SummarizeOpts,
) -> AiOutput {
    summarize_impl(lines, time_keys, baseline_templates, opts)
}

/// Extract placeholder names from a template string efficiently
/// Returns a HashSet of placeholder names (without < > brackets) for O(1) lookup
fn extract_placeholders(template: &str) -> HashSet<String> {
    let mut placeholders = HashSet::new();
    let chars: Vec<char> = template.chars().collect();
    let mut i = 0;
    
    while i < chars.len() {
        if chars[i] == '<' {
            let start = i + 1;
            let mut end = start;
            
            // Find the closing >
            while end < chars.len() && chars[end] != '>' {
                end += 1;
            }
            
            if end < chars.len() {
                let placeholder: String = chars[start..end].iter().collect();
                if !placeholder.is_empty() {
                    placeholders.insert(placeholder);
                }
                i = end + 1;
            } else {
                i += 1;
            }
        } else {
            i += 1;
        }
    }
    
    placeholders
}

/// Helper function to optimize a template by replacing single-cardinality placeholders with actual values
pub fn optimize_template_with_stats(template: &str, param_stats: &HashMap<String, ParamFieldStats>) -> String {
    let mut optimized = template.to_string();
    
    // Replace single-cardinality parameters with their actual values
    // Use regex replacement to ensure exact placeholder matching and avoid partial replacements
    for (param_type, stats) in param_stats.iter() {
        if stats.cardinality == 1 && stats.values.len() > 0 {
            let placeholder_pattern = format!(r"<{}>", regex::escape(param_type));
            let actual_value = &stats.values[0].value;
            
            // Use regex to ensure we only replace exact placeholder matches
            if let Ok(re) = regex::Regex::new(&placeholder_pattern) {
                optimized = re.replace_all(&optimized, actual_value).to_string();
            }
        }
    }
    
    // Remove empty placeholders and their associated field names
    // Pattern like "field = <>" should be removed entirely
    optimized = optimized.replace(" = <>", "");
    optimized = optimized.replace("  ", " "); // Clean up double spaces
    
    optimized
}

/// Converts full analysis output to compact triage format
pub fn create_triage_output(full_output: &AiOutput) -> TriageOutput {
    // Filter for critical patterns only (ERROR level + high anomaly/burst patterns)
    let mut pattern_anomalies = Vec::new();
    let mut burst_count = 0;
    let mut error_count = 0;
    
    for pattern in &full_output.patterns {
        let is_error = matches!(pattern.severity.as_deref(), 
            Some("error") | Some("ERROR") | Some("err") | Some("ERR"));
        let has_bursts = pattern.temporal.as_ref().map(|t| t.bursts > 0).unwrap_or(false);
        let has_spikes = pattern.spike_analysis.is_some();
        let has_param_anomalies = pattern.parameter_anomalies.is_some();
        
        if is_error {
            error_count += pattern.total_count;
        }
        
        if has_bursts {
            burst_count += 1;
        }
        
        // Include pattern if: ERROR level OR has significant anomalies/bursts
        if is_error || has_bursts || has_spikes || has_param_anomalies {
            let (anomaly_type, anomaly_details) = if has_bursts { 
                let burst_count = pattern.temporal.as_ref().map(|t| t.bursts).unwrap_or(0);
                
                // Get concise burst information - one line per burst
                let burst_details = if let Some(spike_analysis) = &pattern.spike_analysis {
                    // Use spike analysis if available (has start time, peak rate, severity)
                    spike_analysis.spikes.iter().map(|spike| {
                        format!(
                            "Burst: {} events/min peak at {} ({:.1}x above normal)", 
                            spike.event_count,
                            spike.time,
                            spike.severity
                        )
                    }).collect::<Vec<_>>()
                } else {
                    // Concise fallback using available temporal data
                    if let Some(temporal) = pattern.temporal.as_ref() {
                        if let Some(largest_burst_time) = &temporal.largest_burst {
                            let trend_info = temporal.trend.as_ref()
                                .map(|t| format!(" (trend: {})", t))
                                .unwrap_or_default();
                            
                            if burst_count > 1 {
                                vec![format!("Burst: {} occurrences, largest at {}{}", burst_count, largest_burst_time, trend_info)]
                            } else {
                                vec![format!("Burst: detected at {}{}", largest_burst_time, trend_info)]
                            }
                        } else {
                            vec![format!("Burst: {} occurrence(s) detected in pattern", burst_count)]
                        }
                    } else {
                        vec![format!("Burst: {} occurrence(s) in {} events", burst_count, pattern.total_count)]
                    }
                };
                
                (Some("burst".to_string()), Some(burst_details))
            } else if has_spikes { 
                (Some("spike".to_string()), Some(vec!["Unusual traffic spike detected".to_string()]))
            } else if let Some(ref param_anomalies) = pattern.parameter_anomalies {
                // Convert parameter anomalies to array of strings
                let details: Vec<String> = param_anomalies.iter()
                    .map(|a| a.details.clone())
                    .collect();
                (Some("parameter_anomaly".to_string()), if details.is_empty() { None } else { Some(details) })
            } else { 
                (None, None)
            };
            
            pattern_anomalies.push(TriagePattern {
                template: pattern.template.clone(),
                count: pattern.total_count,
                severity: pattern.severity.clone().unwrap_or_else(|| "UNKNOWN".to_string()),
                anomaly_type,
                anomaly_details,
                example: pattern.examples.first().cloned(), // Include first example
            });
        }
    }
    
    // Sort pattern anomalies by importance: ERROR first, then by count
    pattern_anomalies.sort_by(|a, b| {
        let a_is_error = matches!(a.severity.as_str(), "error" | "ERROR" | "err" | "ERR");
        let b_is_error = matches!(b.severity.as_str(), "error" | "ERROR" | "err" | "ERR");
        
        b_is_error.cmp(&a_is_error)
            .then_with(|| b.count.cmp(&a.count))
            .then_with(|| a.template.cmp(&b.template))
    });
    
    // Include ALL patterns with anomalies (no truncation to ensure complete coverage)
    
    
    // Generate actionable insights
    let mut insights = Vec::new();
    
    if error_count > 0 {
        insights.push(format!("Found {} error log entries requiring immediate attention", error_count));
    }
    
    if burst_count > 0 {
        insights.push(format!("{} patterns show burst behavior - check for system stress", burst_count));
    }
    
    if !full_output.anomalies.pattern_anomalies.is_empty() {
        insights.push(format!("Detected {} new/rare patterns - investigate for changes", 
            full_output.anomalies.pattern_anomalies.len()));
    }
    
    // Build detailed field anomalies section
    let mut triage_field_anomalies = Vec::new();
    
    for field_anomaly in &full_output.anomalies.field_anomalies {
        let (impact, description, details) = match field_anomaly.anomaly_type.as_str() {
            "cardinality_explosion" => {
                let cardinality_pct = (field_anomaly.ratio.unwrap_or(0.0) * 100.0) as i32;
                let unique_count = field_anomaly.unique_count.unwrap_or(0);
                let total_count = field_anomaly.total.unwrap_or(0);
                
                (
                    "CRITICAL".to_string(),
                    format!("Field '{}' has {}% unique values", field_anomaly.field, cardinality_pct),
                    vec![
                        format!("{} unique values out of {} total entries", unique_count, total_count),
                        "This indicates potential data leakage or masking failure".to_string(),
                        "Every log entry has a unique value in this field".to_string(),
                        "Consider reviewing data sanitization and templating logic".to_string(),
                    ]
                )
            },
            "numeric_outlier" => {
                (
                    "MEDIUM".to_string(),
                    format!("Numeric outlier detected in field '{}'", field_anomaly.field),
                    vec![
                        "Unusual numeric values detected that deviate from normal patterns".to_string(),
                        "May indicate system stress, configuration changes, or anomalous behavior".to_string(),
                    ]
                )
            },
            _ => {
                (
                    "LOW".to_string(),
                    format!("Field anomaly: {}", field_anomaly.anomaly_type),
                    vec![format!("Anomaly detected in field '{}'", field_anomaly.field)]
                )
            }
        };
        
        triage_field_anomalies.push(TriageFieldAnomaly {
            anomaly_type: field_anomaly.anomaly_type.clone(),
            field: field_anomaly.field.clone(),
            description,
            impact,
            details,
        });
    }
    
    // Add summary insights for field anomalies (keeping the existing insight messages)
    for field_anomaly in &full_output.anomalies.field_anomalies {
        match field_anomaly.anomaly_type.as_str() {
            "cardinality_explosion" => {
                insights.push(format!(
                    "CRITICAL: Field '{}' has {}% unique values ({} distinct) - potential data leak or masking failure", 
                    field_anomaly.field,
                    (field_anomaly.ratio.unwrap_or(0.0) * 100.0) as i32,
                    field_anomaly.unique_count.unwrap_or(0)
                ));
            },
            "numeric_outlier" => {
                insights.push(format!(
                    "Numeric outlier detected in field '{}' - investigate anomalous values", 
                    field_anomaly.field
                ));
            },
            _ => {} // Skip other field anomaly types for now to keep insights concise
        }
    }
    
    // Count patterns with anomalies for status determination
    let anomaly_pattern_count = pattern_anomalies.iter()
        .filter(|p| p.anomaly_type.is_some())
        .count();
    
    if pattern_anomalies.is_empty() {
        insights.push("No critical issues detected - system appears stable".to_string());
    }
    
    // Determine overall status
    let status = if error_count > 10 || burst_count > 3 {
        "CRITICAL"
    } else if error_count > 0 || burst_count > 0 || anomaly_pattern_count > 0 {
        "WARNING"  
    } else {
        "NORMAL"
    };
    
    // Create time range string
    let time_range = match (&full_output.summary.start_date, &full_output.summary.end_date) {
        (Some(start), Some(end)) => Some(format!("{} to {}", start, end)),
        (Some(start), None) => Some(format!("from {}", start)),
        (None, Some(end)) => Some(format!("until {}", end)),
        (None, None) => None,
    };
    
    TriageOutput {
        summary: TriageSummary {
            total_lines: full_output.summary.total_lines,
            error_lines: error_count,
            burst_patterns: burst_count,
            anomaly_count: anomaly_pattern_count,
            time_range,
            status: status.to_string(),
        },
        pattern_anomalies,
        field_anomalies: triage_field_anomalies,
        insights,
    }
}

fn summarize_impl<'a>(lines: &[&'a str], time_keys: &[&'a str], baseline_opt: Option<&HashSet<String>>, opts: &SummarizeOpts) -> AiOutput {
    use std::time::Instant;
    let start_time = Instant::now();
    let mut stage_times = Vec::new();
    
    let total = lines.len();
    let mut min_ts: Option<chrono::DateTime<chrono::Utc>> = None;
    let mut max_ts: Option<chrono::DateTime<chrono::Utc>> = None;
    // Collect per-line data (parallel)
    #[derive(Clone)]
    struct LineDeriv {
        message: String,
        timestamp: Option<chrono::DateTime<chrono::Utc>>,
        base: String,
        level: Option<String>,
        service: Option<String>,
        host: Option<String>,
        malformed_json: bool,
        fingerprint: Option<schema::Fingerprint>,
        flat_fields: Option<std::collections::BTreeMap<String,String>>,
    }

    /// Creates human-friendly templates by replacing generic placeholders with field-specific ones
    /// For example: "api_id = <HEX> org_id = <HEX>" becomes "api_id = <API_ID> org_id = <ORG_ID>"
    fn create_human_friendly_template(drain_template: &str, line_data: &LineDeriv) -> String {
        let mut result = drain_template.to_string();
        
        // Get the flat fields from the structured data
        if let Some(fields) = &line_data.flat_fields {
            // Process field by field, replacing generic placeholders with field-specific ones
            for (field_name, _field_value) in fields {
                // Skip infrastructure fields we don't want to track
                if field_name == "host" || field_name == "hostname" || field_name == "service" ||
                   field_name.starts_with("kubernetes.") || field_name == "pod" || 
                   field_name == "namespace" || field_name == "container" || field_name == "container_id" {
                    continue;
                }
                
                let field_pattern = format!("{} = ", field_name);
                let field_upper = field_name.to_uppercase().replace("-", "_").replace(".", "_");
                
                // Look for patterns like "api_id = <HEX>" and replace with "api_id = <API_ID>"
                if let Some(start_pos) = result.find(&field_pattern) {
                    let after_equals = start_pos + field_pattern.len();
                    if let Some(placeholder_start) = result[after_equals..].find('<') {
                        let abs_placeholder_start = after_equals + placeholder_start;
                        if let Some(placeholder_end) = result[abs_placeholder_start..].find('>') {
                            let abs_placeholder_end = abs_placeholder_start + placeholder_end + 1;
                            let new_placeholder = format!("<{}>", field_upper);
                            result.replace_range(abs_placeholder_start..abs_placeholder_end, &new_placeholder);
                        }
                    }
                }
            }
        }
        
        result
    }

    /// Fast template-only humanizer that works on Drain templates directly
    /// This version infers field names from template structure without needing per-line data
    /// For example: "key = <*>" becomes "key = <KEY>"
    fn create_human_friendly_template_fast(drain_template: &str) -> String {
        let mut result = drain_template.to_string();
        
        // Use regex to find patterns like "field_name = <something>"
        let field_pattern = regex::Regex::new(r"([a-zA-Z_][a-zA-Z0-9_.-]*) = <[^>]*>").unwrap();
        
        // Replace each match
        loop {
            let mut replaced = false;
            result = field_pattern.replace(&result, |caps: &regex::Captures| {
                let field_name = &caps[1];
                let original = &caps[0];
                
                // Skip infrastructure fields we don't want to track
                if field_name == "host" || field_name == "hostname" || field_name == "service" ||
                   field_name.starts_with("kubernetes.") || field_name == "pod" || 
                   field_name == "namespace" || field_name == "container" || field_name == "container_id" {
                    return caps[0].to_string(); // Return original unchanged
                }
                
                let field_upper = field_name.to_uppercase().replace("-", "_").replace(".", "_");
                let replacement = format!("{} = <{}>", field_name, field_upper);
                
                // Only mark as replaced if we're actually changing something
                if replacement != original {
                    replaced = true;
                    replacement
                } else {
                    original.to_string()
                }
            }).to_string();
            
            if !replaced {
                break;
            }
        }
        
        result
    }

    // Stage 1: Parse lines and extract initial data
    let stage_start = Instant::now();
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
            // Extract level from JSON fields or detect in plain text
            let level = rec.flat_fields.as_ref()
                .and_then(|f| f.get("level").cloned())
                .or_else(|| {
                    // For plain text logs, try to detect common log levels
                    let msg_upper = rec.message.to_uppercase();
                    if msg_upper.contains(" ERROR") || msg_upper.contains(" ERR ") {
                        Some("ERROR".to_string())
                    } else if msg_upper.contains(" WARN") || msg_upper.contains(" WARNING") {
                        Some("WARN".to_string())
                    } else if msg_upper.contains(" INFO") {
                        Some("INFO".to_string())
                    } else if msg_upper.contains(" DEBUG") {
                        Some("DEBUG".to_string())
                    } else if msg_upper.contains(" TRACE") {
                        Some("TRACE".to_string())
                    } else {
                        None
                    }
                });
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

            LineDeriv { message: rec.message, timestamp: rec.timestamp, base, level, service: service_opt, host: host_opt, malformed_json, fingerprint, flat_fields: rec.flat_fields.clone() }
        })
        .collect();
    stage_times.push(("Stage 1: Parse lines", stage_start.elapsed()));

    // Combine derived data
    let mut messages: Vec<String> = Vec::with_capacity(total);
    let mut timestamps: Vec<Option<chrono::DateTime<chrono::Utc>>> = Vec::with_capacity(total);
    let mut levels: Vec<Option<String>> = Vec::with_capacity(total);
    let mut templates: Vec<String> = Vec::with_capacity(total);
    let mut json_fps: Vec<(usize, schema::Fingerprint, Option<chrono::DateTime<chrono::Utc>>)> = Vec::new();
    let mut error_samples: Vec<ErrorSample> = Vec::new();
    let mut service_by_tpl: HashMap<String, HashMap<String, usize>> = HashMap::new();
    let mut host_by_tpl: HashMap<String, HashMap<String, usize>> = HashMap::new();
    for (i, d) in derived.iter().enumerate() {
        if let Some(ts) = d.timestamp {
            min_ts = Some(match min_ts { Some(m) => m.min(ts), None => ts });
            max_ts = Some(match max_ts { Some(m) => m.max(ts), None => ts });
        }
        if d.malformed_json && error_samples.len() < 10 {
            error_samples.push(ErrorSample { line_number: i + 1, kind: "malformed_json".into() });
        }
        if let Some(fp) = d.fingerprint.as_ref() { json_fps.push((i, fp.clone(), d.timestamp)); }
        // service/host attribution computed after templates are assigned
        messages.push(d.message.clone());
        timestamps.push(d.timestamp);
        levels.push(d.level.clone());
        templates.push(String::new());
    }
    // Stage 2: Compute templates per line with parameter tracking
    let stage_start = Instant::now();
    let mut drain_templates_raw: Vec<Option<String>> = vec![None; messages.len()];
    let mut line_params: Vec<HashMap<String, Vec<String>>> = vec![HashMap::new(); messages.len()];
    
    // Always use Drain with masking for consistent pattern extraction
    // More aggressive clustering for structured logs
    let mut drain = drain_adapter::DrainAdapter::new_tuned_with_filters(32, 0.1, 512);
    
    // Store canonicalization results to avoid recomputing in Pass 2
    let mut canon_results: Vec<Option<param_extractor::MaskingResult>> = vec![None; messages.len()];
    
    // Pass 1: Optimized two-phase processing with batch deduplication and parallelization
    let pass1_start = Instant::now();
    
    // Phase 1a: Group identical canonicalization keys and canonicalize in parallel
    let phase1a_start = Instant::now();
    
    // Group lines by canonicalization key (message for JSON, base for others)
    use std::collections::{BTreeMap, BTreeSet};
    let mut canon_groups: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    let mut index_to_canon_key: Vec<String> = Vec::with_capacity(derived.len());
    
    for (i, d) in derived.iter().enumerate() {
        // For JSON logs, use original message; for others use base
        let canon_key = if d.flat_fields.is_some() {
            d.message.clone()
        } else {
            d.base.clone()
        };
        index_to_canon_key.push(canon_key.clone());
        canon_groups.entry(canon_key).or_insert_with(Vec::new).push(i);
    }
    
    // Canonicalize unique keys in parallel using Rayon
    let unique_canon_keys: Vec<_> = canon_groups.keys().cloned().collect();
    let canon_results_unique: Vec<_> = unique_canon_keys
        .par_iter()
        .map(|key| param_extractor::canonicalize_for_drain(key))
        .collect();
    
    // Create mapping from canonicalization key to result
    let key_to_canon: BTreeMap<String, param_extractor::MaskingResult> = 
        unique_canon_keys.into_iter().zip(canon_results_unique.into_iter()).collect();
    
    // Fan out canonicalization results to all original indices
    for (canon_key, indices) in canon_groups.iter() {
        if let Some(canon_result) = key_to_canon.get(canon_key) {
            for &i in indices {
                line_params[i] = canon_result.extracted_params.clone();
                canon_results[i] = Some(canon_result.clone());
                
                // Also extract from structured fields if available
                if let Some(ff) = derived[i].flat_fields.as_ref() {
                    let kv_params = param_extractor::extract_kv_params(ff);
                    line_params[i] = param_extractor::merge_params(line_params[i].clone(), kv_params);
                }
            }
        }
    }
    stage_times.push(("  Phase 1a: Batch canonicalization", phase1a_start.elapsed()));
    
    // Phase 1b: Insert only unique masked_text into Drain tree once each
    let phase1b_start = Instant::now();
    
    // Deduplicate masked_text and build Drain tree
    let mut unique_masked: BTreeSet<String> = BTreeSet::new();
    for canon_result in key_to_canon.values() {
        unique_masked.insert(canon_result.masked_text.clone());
    }
    
    // Insert unique masked_text and capture templates
    let mut masked_to_template: BTreeMap<String, String> = BTreeMap::new();
    for masked_text in unique_masked.iter() {
        // For structured JSON logs, skip Drain and use canonical templates directly
        // to avoid corrupting our perfect placeholders like <MSG>, <LEVEL>, etc.
        let is_structured_log = masked_text.contains(" = <") && 
                               (masked_text.contains("level = <") || masked_text.contains("msg = <") || 
                                masked_text.contains("timestamp = <") || masked_text.contains("time = <"));
        
        if is_structured_log {
            // Use canonical template directly - it's already perfect for JSON logs
            masked_to_template.insert(masked_text.clone(), masked_text.clone());
        } else {
            // For unstructured logs, use Drain for pattern extraction
            match drain.insert_masked(masked_text) {
                Ok(template) => {
                    masked_to_template.insert(masked_text.clone(), template);
                },
                Err(_) => {
                    // Fallback for failed insertions
                    masked_to_template.insert(masked_text.clone(), masked_text.clone());
                }
            }
        }
    }
    
    // Fan out templates to all original indices
    for (canon_key, indices) in canon_groups.iter() {
        if let Some(canon_result) = key_to_canon.get(canon_key) {
            if let Some(template) = masked_to_template.get(&canon_result.masked_text) {
                for &i in indices {
                    drain_templates_raw[i] = Some(template.clone());
                }
            }
        }
    }
    
    stage_times.push(("  Phase 1b: Drain tree building", phase1b_start.elapsed()));
    stage_times.push(("  Pass 1: Optimized two-phase", pass1_start.elapsed()));
    
    // Pass 2: OPTIMIZED per-unique template humanization with caching
    let pass2_start = Instant::now();
    
    // Build cache of human-friendly templates per unique raw Drain template
    let mut unique_drain_templates: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for drain_template_opt in &drain_templates_raw {
        if let Some(raw) = drain_template_opt {
            unique_drain_templates.insert(raw.clone());
        }
    }
    
    // Compute human-friendly templates only for unique Drain templates (cache computation)
    let template_cache_start = Instant::now();
    let mut human_template_cache: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    for raw_template in unique_drain_templates {
        let human_friendly = create_human_friendly_template_fast(&raw_template);
        human_template_cache.insert(raw_template, human_friendly);
    }
    stage_times.push(("    Cache computation", template_cache_start.elapsed()));
    
    // Direct in-place writes using par_iter_mut() to avoid intermediate allocation
    let write_start = Instant::now();
    templates.par_iter_mut().enumerate().for_each(|(i, template_slot)| {
        if let Some(raw) = &drain_templates_raw[i] {
            // Use cached human-friendly template
            if let Some(cached_template) = human_template_cache.get(raw) {
                *template_slot = cached_template.clone();
            } else {
                // Fallback to raw template if cache miss
                *template_slot = raw.clone();
            }
        } else {
            // Fallback template for lines without Drain templates
            if let Some(cached_canon) = &canon_results[i] {
                *template_slot = to_generic_template(&cached_canon.masked_text);
            } else {
                // This should be rare as canonicalization was cached in Pass 1
                let canon = param_extractor::canonicalize_for_drain(&derived[i].base);
                *template_slot = to_generic_template(&canon.masked_text);
            }
        }
    });
    stage_times.push(("    Direct writes", write_start.elapsed()));
    
    // Track Drain effectiveness only if verbose mode is enabled (and limit sample size)
    if opts.verbose {
        let effectiveness_start = Instant::now();
        
        // Sample at most 5000 lines to compute effectiveness metrics
        let sample_size = std::cmp::min(5000, messages.len());
        let step = if sample_size < messages.len() {
            messages.len() / sample_size
        } else {
            1
        };
        
        let mut drain_unique_templates = std::collections::HashSet::new();
        let mut masking_unique_templates = std::collections::HashSet::new();
        
        for i in (0..messages.len()).step_by(step).take(sample_size) {
            if let Some(raw) = &drain_templates_raw[i] {
                drain_unique_templates.insert(raw.clone());
            }
            if let Some(canon_result) = &canon_results[i] {
                let masked_template = to_generic_template(&canon_result.masked_text);
                masking_unique_templates.insert(masked_template);
            }
        }
        
        eprintln!("DRAIN EFFECTIVENESS (sampled {} lines): Drain templates: {}, Pure masking templates: {}", 
                  sample_size, drain_unique_templates.len(), masking_unique_templates.len());
        
        stage_times.push(("    Effectiveness sampling", effectiveness_start.elapsed()));
    }
    
    stage_times.push(("  Pass 2: OPTIMIZED Get templates", pass2_start.elapsed()));
    stage_times.push(("Stage 2: Template extraction", stage_start.elapsed()));

    // Now that templates are computed, build source attribution maps using composite keys
    for i in 0..messages.len() {
        let level_suffix = if let Some(level) = &levels[i] {
            format!(" [{}]", level)
        } else {
            String::new()
        };
        let composite_key = format!("{}{}", templates[i], level_suffix);
        
        if let Some(svc) = derived[i].service.clone() {
            *service_by_tpl.entry(composite_key.clone()).or_default().entry(svc).or_insert(0) += 1;
        }
        if let Some(h) = derived[i].host.clone() {
            *host_by_tpl.entry(composite_key.clone()).or_default().entry(h).or_insert(0) += 1;
        }
    }

    // Stage 3: Cluster by template + log level (separate patterns for different log levels)
    let stage_start = Instant::now();
    let mut counts: HashMap<String, usize> = HashMap::new();
    let mut idxs_by_tpl: HashMap<String, Vec<usize>> = HashMap::new();
    let mut times_by_tpl: HashMap<String, Vec<chrono::DateTime<chrono::Utc>>> = HashMap::new();
    for (i, tpl) in templates.iter().enumerate() {
        // Create composite key: template + log level to separate different severities
        let level_suffix = if let Some(level) = &levels[i] {
            format!(" [{}]", level)
        } else {
            String::new()
        };
        let composite_key = format!("{}{}", tpl, level_suffix);
        
        *counts.entry(composite_key.clone()).or_insert(0) += 1;
        idxs_by_tpl.entry(composite_key.clone()).or_default().push(i);
        if let Some(ts) = timestamps[i] {
            times_by_tpl.entry(composite_key.clone()).or_default().push(ts);
        }
    }
    let unique = counts.len();
    let compression_ratio = if unique > 0 {
        (total as f64) / (unique as f64)
    } else {
        0.0
    };
    let start_date = min_ts.map(|t| t.to_rfc3339_opts(chrono::SecondsFormat::Secs, true));
    let end_date = max_ts.map(|t| t.to_rfc3339_opts(chrono::SecondsFormat::Secs, true));
    // Patterns output
    let mut patterns = Vec::new();
    let mut suggestions: Vec<SuggestionOut> = Vec::new();
    stage_times.push(("Stage 3: Clustering", stage_start.elapsed()));
    
    // Stage 4: Build patterns with optimizations
    let stage_start = Instant::now();
    
    // Convert counts to vec for parallel processing
    let counts_vec: Vec<_> = counts.iter().collect();
    let max_examples = if opts.deep { 10 } else { 3 };
    
    // Sampling limits: cap per-pattern analysis for performance
    let sample_limit = if opts.deep { 8192 } else { 2048 };
    
    // Parallel pattern building with optimizations
    let pattern_results: Vec<_> = counts_vec
        .par_iter()
        .map(|(tpl, &cnt)| {
        let idxs = idxs_by_tpl.get(*tpl).unwrap();
        
        // OPTIMIZATION 1: Deterministic sampling for large patterns
        let sampled_idxs = if idxs.len() > sample_limit {
            // Use deterministic stride-based sampling for reproducibility
            let stride = idxs.len() / sample_limit;
            (0..idxs.len()).step_by(stride.max(1)).take(sample_limit)
                .map(|i| idxs[i])
                .collect::<Vec<_>>()
        } else {
            idxs.clone()
        };
        
        // OPTIMIZATION 2: Reuse pre-computed timestamps from times_by_tpl
        // Use existing times_by_tpl instead of re-scanning indices
        let ts_for_tpl = times_by_tpl.get(*tpl).cloned().unwrap_or_default();
        
        // severity = most frequent level (scan sampled indices only)
        let mut lvl_counts: HashMap<String, usize> = HashMap::new();
        let mut exs: Vec<String> = Vec::new();
        for &i in sampled_idxs.iter() {
            if let Some(lv) = levels[i].as_ref() { *lvl_counts.entry(lv.clone()).or_insert(0) += 1; }
            if exs.len() < max_examples { exs.push(messages[i].clone()); }
        }
        let severity = lvl_counts.into_iter().max_by_key(|(_, c)| *c).map(|(l, _)| l);
        
        // Extract start and end times for this pattern
        let start_time = ts_for_tpl.iter().min().map(|t| t.to_rfc3339_opts(chrono::SecondsFormat::Secs, true));
        let end_time = ts_for_tpl.iter().max().map(|t| t.to_rfc3339_opts(chrono::SecondsFormat::Secs, true));
        
        let bursts = temporal::compute_bursts(&ts_for_tpl, chrono::Duration::minutes(1), 3.0);
        let largest_burst = bursts.iter().max_by_key(|b| b.peak_rate).map(|b| b.start_time.to_rfc3339_opts(chrono::SecondsFormat::Secs, true));
        let trend = trend_label(&ts_for_tpl);
        // Skip correlations for simpler analysis
        let related: Vec<CorrelatedOut> = Vec::new();

        // Pattern stability score (0..1): measures how stable/persistent this pattern is
        // Combines: temporal consistency (60% weight) + frequency (40% weight)
        // High score = appears frequently AND consistently over time
        let temporal_consistency = if !ts_for_tpl.is_empty() && min_ts.is_some() && max_ts.is_some() {
            let a = min_ts.unwrap(); let b = max_ts.unwrap();
            let span_secs = (b - a).num_seconds().abs().max(60) as f64;
            let unique_minutes = ts_for_tpl.iter().map(|t| (t.timestamp()/60)).collect::<std::collections::BTreeSet<_>>().len() as f64;
            (unique_minutes * 60.0 / span_secs).min(1.0)
        } else { 0.0 };
        let freq_factor = ((cnt as f64) / (total as f64)).sqrt().min(1.0);
        let pattern_stability = (temporal_consistency * 0.6) + (freq_factor * 0.4);

        // sources breakdown top3
        let mut svc_items: Vec<CountItem> = service_by_tpl.get(*tpl).map(|m| m.iter().map(|(k,v)| CountItem { name: k.clone(), count: *v }).collect()).unwrap_or_else(Vec::new);
        svc_items.sort_by(|a,b| b.count.cmp(&a.count).then_with(|| a.name.cmp(&b.name)));
        if svc_items.len() > 3 { svc_items.truncate(3); }
        let mut host_items: Vec<CountItem> = host_by_tpl.get(*tpl).map(|m| m.iter().map(|(k,v)| CountItem { name: k.clone(), count: *v }).collect()).unwrap_or_else(Vec::new);
        host_items.sort_by(|a,b| b.count.cmp(&a.count).then_with(|| a.name.cmp(&b.name)));
        if host_items.len() > 3 { host_items.truncate(3); }

        // OPTIMIZATION 3 & 4: Optimize parameter aggregation with precomputed placeholders
        let mut param_stats: std::collections::HashMap<String, ParamFieldStats> = std::collections::HashMap::new();
        
        // Extract clean template (remove level suffix) for placeholder extraction
        let clean_template = if let Some(bracket_pos) = tpl.rfind(" [") {
            let suffix = &(*tpl)[bracket_pos..];
            // Check if this looks like a log level suffix (no angle brackets)
            if suffix.ends_with(']') && !suffix.contains('<') && !suffix.contains('>') {
                // This looks like a real log level suffix, remove it
                (*tpl)[..bracket_pos].to_string()
            } else {
                // This contains placeholders or other content, keep it
                (*tpl).clone()
            }
        } else {
            (*tpl).clone()
        };
        
        // OPTIMIZATION 4: Precompute placeholder set once per pattern
        let template_placeholders = extract_placeholders(&clean_template);
        
        // OPTIMIZATION 3: Use AHashMap for hot counting paths - faster than std HashMap
        let mut pattern_params: AHashMap<String, AHashMap<String, usize>> = AHashMap::new();
        
        // Aggregate parameters from sampled lines only (for large patterns)
        // Full accuracy preserved: total_count and frequency use full counts from Stage 3
        // Temporal analysis uses all timestamps via times_by_tpl 
        for &i in sampled_idxs.iter() {
            for (param_type, values) in line_params[i].iter() {
                // Fix empty parameter types by giving them a meaningful name
                let fixed_param_type = if param_type.is_empty() {
                    "NESTED_PATTERN".to_string()
                } else {
                    param_type.clone()
                };
                
                // OPTIMIZATION 4: Use O(1) HashSet membership check instead of contains()
                let should_include = if fixed_param_type == "NESTED_PATTERN" {
                    true  // Always include nested patterns as they're useful anomalies
                } else {
                    template_placeholders.contains(&fixed_param_type)
                };
                
                if should_include {
                    for value in values {
                        *pattern_params.entry(fixed_param_type.clone()).or_default()
                            .entry(value.clone()).or_insert(0) += 1;
                    }
                }
            }
        }
        
        // Compute statistics for each parameter type
        for (param_type, value_counts) in pattern_params.iter() {
            let total: usize = value_counts.values().sum();
            if total == 0 { continue; }
            
            let mut top: Vec<(String, usize)> = value_counts.iter()
                .map(|(k, v)| (k.clone(), *v))
                .collect();
            top.sort_by(|a,b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
            
            let cardinality = top.len();
            
            let top_ratio = if total > 0 { top[0].1 as f64 / total as f64 } else { 0.0 };
            // Include ALL values, not just top 5
            let all_values: Vec<ParamValueCount> = top.iter()
                .map(|(v,c)| ParamValueCount{ value: v.clone(), count: *c })
                .collect();
            
            let stats = ParamFieldStats { 
                total, 
                cardinality, 
                values: all_values.clone(), 
                top_ratio 
            };
            param_stats.insert(param_type.clone(), stats);
        }

        // clean_template already computed above for placeholder extraction
        
        // Optimize template: replace single-cardinality placeholders with actual values
        let optimized_template = optimize_template_with_stats(&clean_template, &param_stats);
        
        // In deep mode, include ALL parameter statistics without filtering; otherwise apply standard filtering
        let filtered_param_stats = if opts.deep {
            param_stats.clone()
        } else {
            let mut filtered = param_stats.clone();
            filtered.retain(|param_name, stats| {
                // Remove empty parameter names
                if param_name.is_empty() {
                    return false;
                }
                // Remove single-cardinality params
                if stats.cardinality <= 1 {
                    return false;
                }
                // Remove TIME-related params (they're now shown as start_time/end_time)
                let name_upper = param_name.to_uppercase();
                if name_upper == "TIME" || name_upper == "TIMESTAMP" || name_upper == "TS" || 
                   name_upper == "DATETIME" || name_upper == "DATE" {
                    return false;
                }
                // Remove high-cardinality numeric parameters (likely timestamps, IDs, etc.)
                // If cardinality is >= 90% of total AND we have > 10 values AND all values are numeric
                if stats.total > 10 && stats.cardinality >= 10 {
                    let cardinality_ratio = stats.cardinality as f64 / stats.total as f64;
                    if cardinality_ratio >= 0.9 {
                        // Check if all values are numeric
                        let all_numeric = stats.values.iter().all(|v| {
                            v.value.chars().all(|c| c.is_ascii_digit())
                        });
                        if all_numeric {
                            return false;
                        }
                    }
                }
                true
            });
            filtered
        };

        // Optional spike analysis
        let spike_analysis = if opts.analyze_spikes && !bursts.is_empty() {
            let spikes: Vec<Spike> = bursts.iter().map(|b| {
                Spike {
                    time: b.start_time.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
                    event_count: b.peak_rate,
                    severity: b.severity,
                }
            }).collect();
            
            if !spikes.is_empty() {
                // Calculate overall rate per minute based on max burst
                let max_rate = bursts.iter().map(|b| b.peak_rate).max().unwrap_or(0) as f64;
                Some(SpikeAnalysis {
                    rate_per_minute: max_rate,
                    spikes,
                })
            } else {
                None
            }
        } else {
            None
        };

        // Use original fast manual approach for non-chunked mode
        Some(PatternOut {
            template: if filtered_param_stats.is_empty() { 
                tpl.to_string() 
            } else { 
                optimize_template_with_stats(&clean_template, &filtered_param_stats)
            },
            frequency: (cnt as f64) / (total as f64),
            total_count: cnt,
            severity,
            start_time,
            end_time,
            spike_analysis,
            temporal: Some(TemporalOut { bursts: bursts.len(), largest_burst, trend }),
            examples: exs,
            correlations: related,
            pattern_stability,
            sources: SourceBreakdown { by_service: svc_items, by_host: host_items },
            drain_template: idxs.get(0).and_then(|&i| drain_templates_raw[i].clone()),
            param_stats: if filtered_param_stats.is_empty() { None } else { Some(filtered_param_stats.clone()) },
            parameter_anomalies: {
                // Fast parameter anomaly detection
                let mut param_anoms = Vec::new();
                for (param_type, stats) in filtered_param_stats.iter() {
                    let total_param = stats.total;
                    if total_param == 0 { continue; }
                    
                    // Skip time-based parameters
                    let is_time_param = param_type == "TIME" || param_type == "TIMESTAMP" || 
                                       param_type == "DATE" || param_type == "DATETIME";
                    let is_high_cardinality_numeric = param_type == "NS" || 
                                                     (param_type == "NUM" && stats.cardinality as f64 / total_param as f64 > 0.9);
                    if is_time_param || is_high_cardinality_numeric { continue; }
                    
                    // Value concentration anomaly
                    if stats.top_ratio >= 0.9 && cnt > 10 && stats.cardinality > 1 {
                        param_anoms.push(ParameterAnomaly {
                            anomaly_type: "value_concentration".to_string(),
                            param: param_type.clone(),
                            value: stats.values.first().map(|v| v.value.clone()).unwrap_or_default(),
                            count: None,
                            ratio: Some(stats.top_ratio),
                            details: format!("{}% of {} '{}' values are '{}'", 
                                (stats.top_ratio * 100.0) as i32, total_param, param_type, 
                                stats.values.first().map(|v| &v.value).unwrap_or(&String::new())),
                        });
                        
                        // Outliers
                        for value_info in stats.values.iter().skip(1) {
                            let ratio = value_info.count as f64 / total_param as f64;
                            if ratio <= 0.1 {
                                param_anoms.push(ParameterAnomaly {
                                    anomaly_type: "outlier".to_string(),
                                    param: param_type.clone(),
                                    value: value_info.value.clone(),
                                    count: Some(value_info.count),
                                    ratio: Some(ratio),
                                    details: format!("Rare '{}' value '{}' appears only {} time(s) out of {} ({}%)",
                                        param_type, value_info.value, value_info.count, total_param, (ratio * 100.0) as i32),
                                });
                            }
                        }
                    }
                    
                    // Low cardinality
                    if stats.cardinality > 1 && stats.cardinality <= 3 && total_param >= 100 {
                        param_anoms.push(ParameterAnomaly {
                            anomaly_type: "low_cardinality".to_string(),
                            param: param_type.clone(),
                            value: format!("{} unique values", stats.cardinality),
                            count: Some(total_param),
                            ratio: None,
                            details: format!("Only {} distinct values seen across {} occurrences of '{}'",
                                stats.cardinality, total_param, param_type),
                        });
                    }
                    
                    // Security alerts
                    if param_type == "IP" && stats.cardinality == 1 && total_param >= 100 {
                        param_anoms.push(ParameterAnomaly {
                            anomaly_type: "SECURITY_ALERT".to_string(),
                            param: param_type.clone(),
                            value: stats.values.first().map(|v| v.value.clone()).unwrap_or_default(),
                            count: Some(total_param),
                            ratio: None,
                            details: format!("All {} requests from single IP: {} - possible bot/attack", 
                                total_param, stats.values.first().map(|v| &v.value).unwrap_or(&String::new())),
                        });
                    }
                }
                if param_anoms.is_empty() { None } else { Some(param_anoms) }
            },
            deep_temporal: if opts.deep && !ts_for_tpl.is_empty() {
                Some(compute_deep_temporal(&ts_for_tpl, &clean_template, &line_params, &idxs))
            } else { None },
            deep_correlations: if opts.deep {
                Some(compute_deep_correlations(&times_by_tpl, tpl))
            } else { None },
        })
        })
        .collect();
    
    // Collect patterns and suggestions
    for pattern_opt in pattern_results {
        if let Some(pattern) = pattern_opt {
            patterns.push(pattern);
        }
    }
    
    // Process suggestions separately after patterns
    for (tpl, _cnt) in counts.iter() {
        let idxs = idxs_by_tpl.get(tpl).unwrap();
        let mut ts_for_tpl: Vec<chrono::DateTime<chrono::Utc>> = Vec::new();
        for &i in idxs.iter() {
            if let Some(ts) = timestamps[i] { ts_for_tpl.push(ts); }
        }
        let bursts = temporal::compute_bursts(&ts_for_tpl, chrono::Duration::minutes(1), 3.0);
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
    
    // Pattern sorting: verbose mode uses importance-based ordering, otherwise count-based
    if opts.verbose {
        patterns.sort_by(|a, b| {
            // Importance-based sorting: severity > stability > count > anomalies/bursts > template
            let importance_a = calculate_pattern_importance(a);
            let importance_b = calculate_pattern_importance(b);
            importance_b.partial_cmp(&importance_a).unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.template.cmp(&b.template))
        });
    } else {
        // Default: sort by count (descending) then by template (ascending) for stable ordering
        patterns.sort_by(|a, b| {
            b.total_count.cmp(&a.total_count)
                .then_with(|| a.template.cmp(&b.template))
        });
    }
    
    // Schema changes (only in streaming mode when baseline is provided)
    let mut schema_changes = Vec::new();
    if baseline_opt.is_some() && json_fps.len() >= 2 {
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
        .map(|a| PatternAnomalyOut { 
            kind: match a.kind { 
                anomaly::AnomalyKind::NewPattern => "NewPattern".into(), 
                anomaly::AnomalyKind::RarePattern => "RarePattern".into() 
            }, 
            template: a.template, 
            frequency: a.frequency,
            count: a.count
        })
        .collect();
    // Field anomalies using robust numeric stats and categorical explosions
    let lines_refs: Vec<&str> = lines.iter().map(|s| *s).collect();
    let num_outliers = crate::field_anomaly::analyze_numeric_outliers(&lines_refs, 3.0);
    let cat_explosions = crate::field_anomaly::analyze_categorical_explosions(&lines_refs, 0.8, 10);
    let mut field_anomalies = Vec::new();
    for o in num_outliers {
        field_anomalies.push(FieldAnomaly {
            anomaly_type: "numeric_outlier".to_string(),
            field: o.field.clone(),
            template: o.template.clone(),
            value: Some(o.value),
            z_score: Some(o.robust_z),
            unique_count: None,
            total: None,
            ratio: None,
        });
    }
    for e in cat_explosions {
        field_anomalies.push(FieldAnomaly {
            anomaly_type: "cardinality_explosion".to_string(),
            field: e.field.clone(),
            template: e.template.clone(),
            value: None,
            z_score: None,
            unique_count: Some(e.unique_count),
            total: Some(e.total),
            ratio: Some(e.ratio),
        });
    }

    // Temporal anomalies: bursts only (gap analysis removed)
    let mut temporal_anomalies = Vec::new();
    for (tpl, ts_list) in times_by_tpl.iter() {
        let bursts = temporal::compute_bursts(ts_list, chrono::Duration::minutes(1), 3.0);
        for b in bursts {
            temporal_anomalies.push(format!("burst template={} start={} end={} peak={}", tpl, b.start_time.to_rfc3339_opts(chrono::SecondsFormat::Secs, true), b.end_time.to_rfc3339_opts(chrono::SecondsFormat::Secs, true), b.peak_rate));
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

    stage_times.push(("Stage 4: Build patterns", stage_start.elapsed()));
    
    // Print timing information
    let total_time = start_time.elapsed();
    eprintln!("\n=== Performance Timing ===");
    eprintln!("Total lines processed: {}", total);
    for (stage_name, duration) in &stage_times {
        eprintln!("{}: {:.3}s", stage_name, duration.as_secs_f64());
    }
    eprintln!("Total time: {:.3}s", total_time.as_secs_f64());
    eprintln!("=======================\n");
    
    AiOutput {
        summary: Summary { total_lines: total, unique_patterns: unique, compression_ratio, start_date, end_date },
        patterns,
        schema_changes,
        anomalies,
        query_interface,
        errors: ErrorsOut { total: error_samples.len(), samples: error_samples },
    }
}

fn to_generic_template(masked: &str) -> String {
    // Replace any <SOMETHING> pattern with <*>
    let re = regex::Regex::new(r"<[^>]+>").unwrap();
    re.replace_all(masked, "<*>").to_string()
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

// Deep analysis functions
pub fn compute_deep_temporal(
    timestamps: &[chrono::DateTime<chrono::Utc>],
    template: &str,
    line_params: &[HashMap<String, Vec<String>>],
    pattern_indices: &[usize],
) -> DeepTemporalOut {
    use chrono::Timelike;
    
    // Hourly distribution
    let mut hourly_counts: [usize; 24] = [0; 24];
    let total_count = timestamps.len();
    
    for ts in timestamps {
        let hour = ts.hour() as usize;
        if hour < 24 {
            hourly_counts[hour] += 1;
        }
    }
    
    let hourly_distribution: Vec<HourlyCount> = hourly_counts
        .iter()
        .enumerate()
        .map(|(hour, &count)| HourlyCount {
            hour: hour as u32,
            count,
            percentage: if total_count > 0 { count as f64 / total_count as f64 * 100.0 } else { 0.0 },
        })
        .collect();
    
    // Time clustering - group into 3-hour windows
    let mut time_clusters = Vec::new();
    for window_start in (0..24).step_by(3) {
        let window_end = (window_start + 3).min(24);
        let window_count: usize = (window_start..window_end)
            .map(|h| hourly_counts[h])
            .sum();
        
        if window_count > 0 {
            let activity_score = window_count as f64 / total_count as f64;
            time_clusters.push(TimeCluster {
                time_window: format!("{:02}:00-{:02}:00", window_start, window_end),
                pattern_count: window_count,
                dominant_pattern: template.to_string(),
                activity_score,
            });
        }
    }
    
    // Pattern evolution - analyze parameter changes over time
    let mut pattern_evolution = Vec::new();
    if timestamps.len() > 10 {
        // Split into time windows and track parameter value changes
        let mut sorted_indices: Vec<_> = pattern_indices.iter()
            .enumerate()
            .filter_map(|(i, &idx)| timestamps.get(i).map(|ts| (idx, *ts)))
            .collect();
        sorted_indices.sort_by_key(|(_, ts)| *ts);
        
        let window_size = sorted_indices.len() / 3; // 3 time windows
        if window_size > 0 {
            for (window_idx, window_indices) in sorted_indices.chunks(window_size).enumerate() {
                let start_time = window_indices.first().map(|(_, ts)| ts);
                let end_time = window_indices.last().map(|(_, ts)| ts);
                
                if let (Some(start), Some(end)) = (start_time, end_time) {
                    let time_window = format!("{} to {}", 
                        start.format("%H:%M:%S"), 
                        end.format("%H:%M:%S")
                    );
                    
                    // Track parameter shifts in this window
                    let mut parameter_shifts = Vec::new();
                    
                    // Analyze parameter value distributions in this window vs previous
                    if window_idx > 0 {
                        // Compare with previous window
                        let prev_window = &sorted_indices[(window_idx - 1) * window_size..window_idx * window_size];
                        
                        // Get parameter distributions for both windows
                        for param_type in ["IP", "NUM", "HEX", "PATH", "URL"].iter() {
                            let mut curr_param_counts = HashMap::new();
                            let mut prev_param_counts = HashMap::new();
                            
                            // Current window parameter values
                            for &(idx, _) in window_indices {
                                if let Some(params) = line_params.get(idx) {
                                    if let Some(values) = params.get(*param_type) {
                                        for value in values {
                                            *curr_param_counts.entry(value.clone()).or_insert(0) += 1;
                                        }
                                    }
                                }
                            }
                            
                            // Previous window parameter values
                            for &(idx, _) in prev_window {
                                if let Some(params) = line_params.get(idx) {
                                    if let Some(values) = params.get(*param_type) {
                                        for value in values {
                                            *prev_param_counts.entry(value.clone()).or_insert(0) += 1;
                                        }
                                    }
                                }
                            }
                            
                            // Find dominant values and detect shifts
                            let curr_dominant = curr_param_counts.iter()
                                .max_by_key(|(_, &count)| count)
                                .map(|(val, _)| val.clone());
                            let prev_dominant = prev_param_counts.iter()
                                .max_by_key(|(_, &count)| count)
                                .map(|(val, _)| val.clone());
                            
                            if let (Some(curr), Some(prev)) = (&curr_dominant, &prev_dominant) {
                                if curr != prev {
                                    let total_curr: usize = curr_param_counts.values().sum();
                                    let total_prev: usize = prev_param_counts.values().sum();
                                    let change_ratio = if total_prev > 0 {
                                        total_curr as f64 / total_prev as f64
                                    } else {
                                        1.0
                                    };
                                    
                                    parameter_shifts.push(ParameterShift {
                                        parameter: param_type.to_string(),
                                        old_dominant_value: prev.clone(),
                                        new_dominant_value: curr.clone(),
                                        change_ratio,
                                    });
                                }
                            }
                        }
                    }
                    
                    pattern_evolution.push(PatternEvolution {
                        time_window,
                        template_changes: vec![template.to_string()],
                        parameter_shifts,
                    });
                }
            }
        }
    }
    
    // Enhanced burst analysis with contributing factors
    let bursts = temporal::compute_bursts(timestamps, chrono::Duration::minutes(1), 3.0);
    let burst_analysis: Vec<BurstDetail> = bursts.iter().map(|b| {
        let mut contributing_factors = Vec::new();
        
        // Analyze what might have caused this burst
        let burst_duration = (b.end_time - b.start_time).num_minutes();
        if burst_duration <= 1 {
            contributing_factors.push("Short spike - possible system event".to_string());
        } else if burst_duration <= 5 {
            contributing_factors.push("Medium burst - possible load increase".to_string());
        } else {
            contributing_factors.push("Extended burst - sustained high activity".to_string());
        }
        
        if b.peak_rate > 100 {
            contributing_factors.push("High volume event".to_string());
        }
        
        // Analyze hour of day
        let hour = b.start_time.hour();
        if hour >= 9 && hour <= 17 {
            contributing_factors.push("Business hours activity".to_string());
        } else if hour <= 6 {
            contributing_factors.push("Off-hours activity - investigate".to_string());
        }
        
        BurstDetail {
            start_time: b.start_time.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
            end_time: b.end_time.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
            peak_rate: b.peak_rate,
            total_events: timestamps.iter()
                .filter(|&&ts| ts >= b.start_time && ts <= b.end_time)
                .count(),
            contributing_factors,
            severity: format!("{:.2}", b.severity),
        }
    }).collect();
    
    DeepTemporalOut {
        hourly_distribution,
        time_clustering: time_clusters,
        pattern_evolution,
        burst_analysis,
    }
}

pub fn compute_deep_correlations(
    times_by_template: &HashMap<String, Vec<chrono::DateTime<chrono::Utc>>>,
    current_template: &str,
) -> Vec<DeepCorrelation> {
    use std::collections::BTreeSet;
    
    let mut correlations = Vec::new();
    
    if let Some(current_times) = times_by_template.get(current_template) {
        for (other_template, other_times) in times_by_template.iter() {
            if other_template == current_template || current_times.is_empty() || other_times.is_empty() {
                continue;
            }
            
            // Convert to sorted sets for efficient operations
            let current_set: BTreeSet<i64> = current_times.iter()
                .map(|ts| ts.timestamp())
                .collect();
            let other_set: BTreeSet<i64> = other_times.iter()
                .map(|ts| ts.timestamp())
                .collect();
            
            // Calculate temporal correlation within different time windows
            let time_windows = [10, 30, 60, 300]; // seconds
            
            for &window in &time_windows {
                let mut co_occurrences = 0;
                let mut current_near_other = 0;
                let mut other_near_current = 0;
                
                for &curr_ts in &current_set {
                    let window_start = curr_ts - window;
                    let window_end = curr_ts + window;
                    
                    // Count how many "other" events are near this current event
                    let near_count = other_set.range(window_start..=window_end).count();
                    if near_count > 0 {
                        co_occurrences += 1;
                        current_near_other += near_count;
                    }
                }
                
                for &other_ts in &other_set {
                    let window_start = other_ts - window;
                    let window_end = other_ts + window;
                    
                    // Count how many "current" events are near this other event
                    let near_count = current_set.range(window_start..=window_end).count();
                    if near_count > 0 {
                        other_near_current += near_count;
                    }
                }
                
                if co_occurrences > 0 {
                    let co_occurrence_rate = co_occurrences as f64 / current_set.len() as f64;
                    
                    // Calculate correlation strength based on mutual proximity
                    let strength = if current_set.len() > 0 && other_set.len() > 0 {
                        let forward_correlation = current_near_other as f64 / (current_set.len() * other_set.len()) as f64;
                        let backward_correlation = other_near_current as f64 / (current_set.len() * other_set.len()) as f64;
                        (forward_correlation + backward_correlation) / 2.0
                    } else {
                        0.0
                    };
                    
                    // Determine analysis type and time lag
                    let (analysis_type, time_lag) = if window <= 30 {
                        ("temporal".to_string(), 0)
                    } else if current_near_other > other_near_current {
                        ("causal".to_string(), window / 2) // Estimate average lag
                    } else if other_near_current > current_near_other {
                        ("causal".to_string(), -(window / 2)) // Other leads current
                    } else {
                        ("inverse".to_string(), 0)
                    };
                    
                    // Only include meaningful correlations
                    if strength > 0.1 && co_occurrence_rate > 0.05 {
                        correlations.push(DeepCorrelation {
                            template_a: current_template.to_string(),
                            template_b: other_template.to_string(),
                            correlation_strength: strength.min(1.0),
                            time_lag_seconds: time_lag as i32,
                            co_occurrence_rate,
                            analysis_type,
                        });
                    }
                }
            }
        }
    }
    
    // Sort by strength and limit to top 10
    correlations.sort_by(|a, b| b.correlation_strength.partial_cmp(&a.correlation_strength).unwrap_or(std::cmp::Ordering::Equal));
    correlations.truncate(10);
    
    correlations
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

/// Streaming / chunked incremental summarizer.
/// Keeps a single Drain + caches across all chunks and aggregates per-pattern stats.
pub struct StreamingSummarizer {
    // --- Drain & caches (shared across chunks) ---
    drain: drain_adapter::DrainAdapter,
    unique_masked: std::collections::BTreeSet<String>,
    masked_to_template: std::collections::BTreeMap<String, String>,
    // Cache base -> canonicalization (MaskingResult) to avoid recompute across chunks
    base_cache: std::collections::HashMap<String, param_extractor::MaskingResult>,
    // Drain template -> human-friendly template cache
    human_template_cache: std::collections::HashMap<String, String>,

    // --- Aggregates ---
    total_lines: usize,
    min_ts: Option<chrono::DateTime<chrono::Utc>>,
    max_ts: Option<chrono::DateTime<chrono::Utc>>,

    // Composite key = "{human_template}{level_suffix}"
    counts: std::collections::HashMap<String, usize>,
    examples: std::collections::HashMap<String, Vec<String>>,
    // severity votes per composite key
    severity_votes: std::collections::HashMap<String, std::collections::HashMap<String, usize>>,
    // source breakdowns
    service_by_tpl: std::collections::HashMap<String, std::collections::HashMap<String, usize>>,
    host_by_tpl: std::collections::HashMap<String, std::collections::HashMap<String, usize>>,
    // param stats: tpl -> (param -> (value -> count))
    param_counts: std::collections::HashMap<String, std::collections::HashMap<String, std::collections::HashMap<String, usize>>>,
    // temporal minute buckets: tpl -> (epoch_minute -> count)
    minute_buckets: std::collections::HashMap<String, std::collections::BTreeMap<i64, usize>>,
    // for deep temporal analysis: store timestamps and params per template (limited to first 1000 to prevent memory issues)
    timestamps_by_tpl: std::collections::HashMap<String, Vec<chrono::DateTime<chrono::Utc>>>,
    line_params_by_tpl: std::collections::HashMap<String, Vec<std::collections::HashMap<String, Vec<String>>>>,

    // schema tracking (first/last JSON fingerprint)
    first_fp: Option<schema::Fingerprint>,
    last_fp: Option<schema::Fingerprint>,
    first_fp_ts: Option<chrono::DateTime<chrono::Utc>>,
    last_fp_ts: Option<chrono::DateTime<chrono::Utc>>,

    // error samples
    error_samples: Vec<ErrorSample>,
}

impl StreamingSummarizer {
    pub fn new() -> Self {
        Self {
            drain: drain_adapter::DrainAdapter::new_tuned_with_filters(32, 0.1, 512),
            unique_masked: std::collections::BTreeSet::new(),
            masked_to_template: std::collections::BTreeMap::new(),
            base_cache: std::collections::HashMap::new(),
            human_template_cache: std::collections::HashMap::new(),
            total_lines: 0,
            min_ts: None,
            max_ts: None,
            counts: std::collections::HashMap::new(),
            examples: std::collections::HashMap::new(),
            severity_votes: std::collections::HashMap::new(),
            service_by_tpl: std::collections::HashMap::new(),
            host_by_tpl: std::collections::HashMap::new(),
            param_counts: std::collections::HashMap::new(),
            minute_buckets: std::collections::HashMap::new(),
            timestamps_by_tpl: std::collections::HashMap::new(),
            line_params_by_tpl: std::collections::HashMap::new(),
            first_fp: None,
            last_fp: None,
            first_fp_ts: None,
            last_fp_ts: None,
            error_samples: Vec::new(),
        }
    }

    /// Fast humanizer for Drain templates (copied from summarize_impl local fn)
    fn humanize_drain_template(&mut self, drain_template: &str) -> String {
        if let Some(h) = self.human_template_cache.get(drain_template) {
            return h.clone();
        }
        let mut result = drain_template.to_string();
        loop {
            let mut replaced = false;
            result = TEMPLATE_FIELD_PATTERN
                .replace_all(&result, |caps: &regex::Captures| {
                    let field_name = &caps[1];
                    if field_name == "host"
                        || field_name == "hostname"
                        || field_name == "service"
                        || field_name.starts_with("kubernetes.")
                        || field_name == "pod"
                        || field_name == "namespace"
                        || field_name == "container"
                        || field_name == "container_id"
                    {
                        return caps[0].to_string();
                    }
                    let field_upper = field_name.to_uppercase().replace("-", "_").replace(".", "_");
                    let replacement = format!("{} = <{}>", field_name, field_upper);
                    let original = &caps[0];
                    
                    // Only mark as replaced if we're actually changing something
                    if replacement != original {
                        replaced = true;
                        replacement
                    } else {
                        original.to_string()
                    }
                })
                .to_string();
            if !replaced {
                break;
            }
        }
        self.human_template_cache
            .insert(drain_template.to_string(), result.clone());
        result
    }

    /// Ingest a chunk of aggregated log records.
    pub fn ingest_chunk<'a>(&mut self, lines: &[String], time_keys: &[&'a str], opts: &SummarizeOpts) {
        use rayon::prelude::*;
        use std::collections::{BTreeMap, BTreeSet, HashMap};

        #[derive(Clone)]
        struct LineDeriv {
            message: String,
            timestamp: Option<chrono::DateTime<chrono::Utc>>,
            base: String,
            level: Option<String>,
            service: Option<String>,
            host: Option<String>,
            malformed_json: bool,
            fingerprint: Option<schema::Fingerprint>,
            flat_fields: Option<std::collections::BTreeMap<String,String>>,
            // params extracted during canonicalization/KV merge
            extracted_params: HashMap<String, Vec<String>>,
            masked_text: String,
        }

        // Stage 1 (per-chunk): parse/derive in parallel
        let derived: Vec<LineDeriv> = lines
            .par_iter()
            .enumerate()
            .map(|(i, l)| {
                let looks_json = l.trim_start().starts_with('{') || l.trim_start().starts_with('[');
                let rec = if time_keys.is_empty() {
                    parser::parse_line(l, i + 1)
                } else {
                    parser::parse_line_with_hints(l, i + 1, time_keys)
                };
                let malformed_json = looks_json && rec.flat_fields.is_none();
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
                let level = rec.flat_fields.as_ref()
                    .and_then(|f| f.get("level").cloned())
                    .or_else(|| {
                        let msg_upper = rec.message.to_uppercase();
                        if msg_upper.contains(" ERROR") || msg_upper.contains(" ERR ") {
                            Some("ERROR".to_string())
                        } else if msg_upper.contains(" WARN") || msg_upper.contains(" WARNING") {
                            Some("WARN".to_string())
                        } else if msg_upper.contains(" INFO") {
                            Some("INFO".to_string())
                        } else if msg_upper.contains(" DEBUG") {
                            Some("DEBUG".to_string())
                        } else if msg_upper.contains(" TRACE") {
                            Some("TRACE".to_string())
                        } else {
                            None
                        }
                    });
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
                LineDeriv {
                    message: rec.message,
                    timestamp: rec.timestamp,
                    base,
                    level,
                    service: service_opt,
                    host: host_opt,
                    malformed_json,
                    fingerprint,
                    flat_fields: rec.flat_fields.clone(),
                    extracted_params: HashMap::new(),
                    masked_text: String::new(),
                }
            })
            .collect();

        // Track min/max timestamps and errors (global)
        for (i, d) in derived.iter().enumerate() {
            if let Some(ts) = d.timestamp {
                self.min_ts = Some(self.min_ts.map(|m| m.min(ts)).unwrap_or(ts));
                self.max_ts = Some(self.max_ts.map(|m| m.max(ts)).unwrap_or(ts));
            }
            if d.malformed_json && self.error_samples.len() < 10 {
                self.error_samples.push(ErrorSample { line_number: i + 1, kind: "malformed_json".into() });
            }
        }

        // Phase 1a: canonicalize unique bases (reuse global cache)
        // IMPORTANT: For JSON logs, we must ALWAYS use the original message as the cache key
        // to avoid issues where the flattened base gets cached separately
        let mut canon_groups: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        
        for (i, d) in derived.iter().enumerate() {
            // Determine the canonicalization key - this MUST be consistent across chunks
            let canon_key = if d.flat_fields.is_some() {
                // JSON log - always use the original message for canonicalization
                d.message.clone()
            } else {
                // Non-JSON log - use the base
                d.base.clone()
            };
            canon_groups.entry(canon_key).or_default().push(i);
        }
        
        // Compute canonicalization for any new entries not in cache
        let unique_canon_keys: Vec<_> = canon_groups.keys().cloned().collect();
        let to_compute: Vec<_> = unique_canon_keys
            .iter()
            .filter(|k| !self.base_cache.contains_key(*k))
            .cloned()
            .collect();
        let computed: Vec<(String, param_extractor::MaskingResult)> = to_compute
            .par_iter()
            .map(|k| (k.clone(), param_extractor::canonicalize_for_drain(k)))
            .collect();
        for (k, res) in computed {
            self.base_cache.insert(k, res);
        }

        // Phase 1b: insert unique masked_text into Drain once
        let mut newly_masked: BTreeSet<String> = BTreeSet::new();
        for canon_key in canon_groups.keys() {
            if let Some(canon) = self.base_cache.get(canon_key) {
                if !self.unique_masked.contains(&canon.masked_text) {
                    newly_masked.insert(canon.masked_text.clone());
                }
            }
        }
        for masked in newly_masked.iter() {
            // For structured JSON logs, skip Drain and use canonical templates directly
            // to avoid corrupting our perfect placeholders like <MSG>, <LEVEL>, etc.
            let is_structured_log = masked.contains(" = <") && 
                                   (masked.contains("level = <") || masked.contains("msg = <") || 
                                    masked.contains("timestamp = <") || masked.contains("time = <"));
            
            let tpl = if is_structured_log {
                // Use canonical template directly - it's already perfect for JSON logs
                masked.clone()
            } else {
                // For unstructured logs, use Drain for pattern extraction
                match self.drain.insert_masked(masked) {
                    Ok(t) => t,
                    Err(_) => masked.clone(),
                }
            };
            self.masked_to_template.insert(masked.clone(), tpl);
        }
        self.unique_masked.extend(newly_masked.into_iter());

        // Fan out: attach masked_text + extract params
        let line_templates_raw: Vec<String> = derived
            .par_iter()
            .enumerate()
            .map(|(_i, d)| {
                // For JSON logs, look up by message; for others by base
                let canon_key = if d.flat_fields.is_some() {
                    &d.message
                } else {
                    &d.base
                };
                let canon = self.base_cache.get(canon_key)
                    .cloned()
                    .unwrap_or_else(|| param_extractor::canonicalize_for_drain(canon_key));
                // store
                let masked = canon.masked_text.clone();
                self.masked_to_template.get(&masked)
                    .cloned()
                    .unwrap_or(masked.clone())
            })
            .collect();

        // Now aggregate per-line into global structures
        for (i, mut d) in derived.into_iter().enumerate() {
            let raw_tpl = &line_templates_raw[i];
            let human_tpl = self.humanize_drain_template(raw_tpl);
            let level_suffix = if let Some(level) = &d.level {
                format!(" [{}]", level)
            } else {
                String::new()
            };
            let composite_key = format!("{}{}", human_tpl, level_suffix);

            // recompute params for this line (single-threaded merge; small cost)
            // For JSON logs, look up by message; for others by base
            let canon_key = if d.flat_fields.is_some() {
                &d.message
            } else {
                &d.base
            };
            let canon = self.base_cache.get(canon_key)
                .cloned()
                .unwrap_or_else(|| param_extractor::canonicalize_for_drain(canon_key));
            let mut params = canon.extracted_params.clone();
            if let Some(ff) = d.flat_fields.as_ref() {
                let kv = param_extractor::extract_kv_params(ff);
                params = param_extractor::merge_params(params, kv);
            }
            d.masked_text = canon.masked_text;
            
            // Store the params for later use in deep analysis
            let extracted_params_for_processing = params.clone();
            let extracted_params_for_deep = params.clone();
            d.extracted_params = params;

            *self.counts.entry(composite_key.clone()).or_insert(0) += 1;
            self.total_lines += 1;
            // keep up to 3 examples (like non-deep mode)
            let exs = self.examples.entry(composite_key.clone()).or_default();
            if exs.len() < 3 { exs.push(d.message.clone()); }
            // severity votes
            if let Some(lv) = d.level.clone() {
                *self.severity_votes.entry(composite_key.clone()).or_default()
                    .entry(lv).or_insert(0) += 1;
            }
            // sources
            if let Some(svc) = d.service.clone() {
                *self.service_by_tpl.entry(composite_key.clone()).or_default()
                    .entry(svc).or_insert(0) += 1;
            }
            if let Some(h) = d.host.clone() {
                *self.host_by_tpl.entry(composite_key.clone()).or_default()
                    .entry(h).or_insert(0) += 1;
            }
            // params: include only ones that appear as placeholders in template
            let clean_template = if let Some(bracket_pos) = composite_key.rfind(" [") {
                let suffix = &composite_key[bracket_pos..];
                if suffix.ends_with(']') && !suffix.contains('<') && !suffix.contains('>') {
                    composite_key[..bracket_pos].to_string()
                } else {
                    composite_key.clone()
                }
            } else { composite_key.clone() };
            let placeholders = extract_placeholders(&clean_template);
            let pc = self.param_counts.entry(composite_key.clone()).or_default();
            for (k, vals) in extracted_params_for_processing.into_iter() {
                let include = k == "NESTED_PATTERN" || placeholders.contains(&k);
                if !include { continue; }
                let m = pc.entry(k).or_default();
                for v in vals { *m.entry(v).or_insert(0) += 1; }
            }
            // temporal minute bucket
            if let Some(ts) = d.timestamp {
                let min_epoch = ts.timestamp() / 60;
                *self.minute_buckets.entry(composite_key.clone()).or_default()
                    .entry(min_epoch).or_insert(0) += 1;
            }
            // schema fingerprints
            if let Some(fp) = d.fingerprint {
                if self.first_fp.is_none() {
                    self.first_fp = Some(fp.clone());
                    self.first_fp_ts = d.timestamp;
                }
                self.last_fp = Some(fp);
                self.last_fp_ts = d.timestamp;
            }
            // collect timestamps and params for deep temporal analysis (limit to prevent memory issues)
            if opts.deep {
                let timestamps = self.timestamps_by_tpl.entry(composite_key.clone()).or_default();
                let line_params = self.line_params_by_tpl.entry(composite_key.clone()).or_default();
                
                // Limit to first 1000 entries per pattern to prevent memory bloat
                if timestamps.len() < 1000 {
                    if let Some(ts) = d.timestamp {
                        timestamps.push(ts);
                        line_params.push(extracted_params_for_deep.clone());
                    }
                }
            }
        }
    }

    /// Finalize aggregated data into AiOutput (no access to original lines).
    pub fn finalize(
        self,
        baseline_opt: Option<&std::collections::HashSet<String>>,
        opts: &SummarizeOpts,
    ) -> AiOutput {
        let total = self.total_lines;
        let unique = self.counts.len();
        let compression_ratio = if unique > 0 { (total as f64) / (unique as f64) } else { 0.0 };
        let start_date = self.min_ts.map(|t| t.to_rfc3339_opts(chrono::SecondsFormat::Secs, true));
        let end_date = self.max_ts.map(|t| t.to_rfc3339_opts(chrono::SecondsFormat::Secs, true));

        let mut patterns = Vec::new();
        let mut suggestions: Vec<SuggestionOut> = Vec::new();

        // Build patterns from aggregates
        for (tpl, cnt) in self.counts.iter() {
            // severity
            let severity = self.severity_votes.get(tpl)
                .and_then(|m| m.iter().max_by_key(|(_,c)| *c).map(|(k,_)| k.clone()));
            // examples
            let examples = self.examples.get(tpl).cloned().unwrap_or_default();
            // sources (top 3)
            let mut svc_items: Vec<CountItem> = self.service_by_tpl.get(tpl)
                .map(|m| m.iter().map(|(k,v)| CountItem{ name: k.clone(), count: *v }).collect())
                .unwrap_or_else(|| Vec::new());
            svc_items.sort_by(|a,b| b.count.cmp(&a.count).then(a.name.cmp(&b.name)));
            if svc_items.len() > 3 { svc_items.truncate(3); }
            let mut host_items: Vec<CountItem> = self.host_by_tpl.get(tpl)
                .map(|m| m.iter().map(|(k,v)| CountItem{ name: k.clone(), count: *v }).collect())
                .unwrap_or_else(|| Vec::new());
            host_items.sort_by(|a,b| b.count.cmp(&a.count).then(a.name.cmp(&b.name)));
            if host_items.len() > 3 { host_items.truncate(3); }
            // param stats
            let param_stats = self.param_counts.get(tpl).map(|pc| {
                let mut out = std::collections::HashMap::new();
                for (param, values) in pc.iter() {
                    let total: usize = values.values().sum();
                    if total == 0 { continue; }
                    let mut top: Vec<(String,usize)> = values.iter().map(|(k,v)| (k.clone(), *v)).collect();
                    top.sort_by(|a,b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
                    let cardinality = top.len();
                    let top_ratio = if total > 0 { top[0].1 as f64 / total as f64 } else { 0.0 };
                    let values_out: Vec<ParamValueCount> = top.into_iter()
                        .map(|(v,c)| ParamValueCount{ value: v, count: c }).collect();
                    out.insert(param.clone(), ParamFieldStats {
                        total,
                        cardinality,
                        values: values_out,
                        top_ratio,
                    });
                }
                out
            });

            let clean_template = if let Some(bracket_pos) = tpl.rfind(" [") {
                let suffix = &tpl[bracket_pos..];
                if suffix.ends_with(']') && !suffix.contains('<') && !suffix.contains('>') {
                    tpl[..bracket_pos].to_string()
                } else { tpl.clone() }
            } else { tpl.clone() };

            // Convert minute buckets to DateTime timestamps for temporal analysis
            let timestamps = if let Some(buckets) = self.minute_buckets.get(tpl) {
                let mut ts = Vec::new();
                for (&minute, &count) in buckets.iter() {
                    if let Some(dt) = chrono::Utc.timestamp_opt(minute * 60, 0).single() {
                        // Add multiple timestamps for each count to represent the frequency
                        for _ in 0..count {
                            ts.push(dt);
                        }
                    }
                }
                ts
            } else {
                self.timestamps_by_tpl.get(tpl).cloned().unwrap_or_default()
            };

            // Compute temporal fields from timestamps
            let start_time = timestamps.iter().min().map(|t| t.to_rfc3339_opts(chrono::SecondsFormat::Secs, true));
            let end_time = timestamps.iter().max().map(|t| t.to_rfc3339_opts(chrono::SecondsFormat::Secs, true));
            
            // Compute temporal analysis
            let bursts = temporal::compute_bursts(&timestamps, chrono::Duration::minutes(1), 3.0);
            let largest_burst = bursts.iter().max_by_key(|b| b.peak_rate)
                .map(|b| b.start_time.to_rfc3339_opts(chrono::SecondsFormat::Secs, true));
            let trend = trend_label(&timestamps);
            let temporal = Some(TemporalOut { 
                bursts: bursts.len(), 
                largest_burst, 
                trend 
            });
            
            // Compute pattern stability
            let temporal_consistency = if !timestamps.is_empty() && self.min_ts.is_some() && self.max_ts.is_some() {
                let a = self.min_ts.unwrap();
                let b = self.max_ts.unwrap();
                let span_secs = (b - a).num_seconds().abs().max(60) as f64;
                let unique_minutes = timestamps.iter()
                    .map(|t| t.timestamp() / 60)
                    .collect::<std::collections::BTreeSet<_>>()
                    .len() as f64;
                (unique_minutes * 60.0 / span_secs).min(1.0)
            } else {
                0.0
            };
            let freq_factor = ((*cnt as f64) / (total as f64)).sqrt().min(1.0);
            let pattern_stability = (temporal_consistency * 0.6) + (freq_factor * 0.4);
            
            // Use unified pattern builder - it will handle all analysis including temporal
            let pattern_data = analyzers::PatternData {
                template: tpl.clone(),
                total_count: *cnt,
                frequency: (*cnt as f64) / (total as f64),
                examples,
                severity,
                start_time,
                end_time,
                spike_analysis: None, // Let build_pattern compute if opts.analyze_spikes
                temporal,
                correlations: Vec::new(),
                pattern_stability,
                service_breakdown: svc_items,
                host_breakdown: host_items,
                drain_template: None,
                param_stats: param_stats,
                timestamps,
                line_params: self.line_params_by_tpl.get(tpl).cloned().unwrap_or_default(),
                pattern_indices: (0..self.timestamps_by_tpl.get(tpl).map(|v| v.len()).unwrap_or(0)).collect(),
            };
            
            patterns.push(analyzers::AnalyzerRegistry::build_pattern(pattern_data, opts, total, None));

            // Suggestion from largest burst if present
            if let Some(buckets) = self.minute_buckets.get(tpl) {
                if let Some((&m, &_c)) = buckets.iter().max_by_key(|(_,c)| *c) {
                    let st = chrono::Utc.timestamp_opt(m * 60, 0).single()
                        .map(|t| t.to_rfc3339_opts(chrono::SecondsFormat::Secs, true));
                    if let Some(st) = st {
                        suggestions.push(SuggestionOut {
                            priority: "HIGH".into(),
                            description: format!("Pattern burst for '{}'", tpl),
                            query: SuggestQuery {
                                command: "GET_LINES_BY_TIME".into(),
                                params: SuggestParams {
                                    start: Some(st.clone()),
                                    end: Some(st), // single-minute window; UI can expand
                                    pattern: Some(tpl.clone()),
                                },
                            },
                        });
                    }
                }
            }
        }

        // Sort patterns similar to default path (by total_count desc)
        patterns.sort_by(|a,b| b.total_count.cmp(&a.total_count).then(a.template.cmp(&b.template)));

        // Schema changes (first/last fp)
        let mut schema_changes = Vec::new();
        if let (Some(first_fp), Some(last_fp), Some(last_ts)) = (self.first_fp.as_ref(), self.last_fp.as_ref(), self.last_fp_ts) {
            for ch in schema::diff_fingerprints(first_fp, last_fp) {
                match ch {
                    schema::SchemaChange::FieldAdded { field, .. } => {
                        schema_changes.push(SchemaChangeOut { timestamp: Some(last_ts.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)), change_type: "field_added".into(), field: field.clone(), impact: None });
                    }
                    schema::SchemaChange::FieldRemoved { field, .. } => {
                        schema_changes.push(SchemaChangeOut { timestamp: Some(last_ts.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)), change_type: "field_removed".into(), field: field.clone(), impact: None });
                    }
                    schema::SchemaChange::TypeChanged { field, .. } => {
                        schema_changes.push(SchemaChangeOut { timestamp: Some(last_ts.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)), change_type: "type_changed".into(), field: field.clone(), impact: None });
                    }
                }
            }
        }

        // Pattern anomalies (New/Rare) using the same helper
        let empty_baseline = std::collections::HashSet::<String>::new();
        let baseline_ref = baseline_opt.unwrap_or(&empty_baseline);
        let pattern_anoms = anomaly::detect_pattern_anomalies(&self.counts, total, baseline_ref, 0.1);
        let pattern_anomalies: Vec<PatternAnomalyOut> = pattern_anoms.into_iter().map(|a| PatternAnomalyOut {
            kind: match a.kind { anomaly::AnomalyKind::NewPattern => "NewPattern".into(), anomaly::AnomalyKind::RarePattern => "RarePattern".into() },
            template: a.template,
            frequency: a.frequency,
            count: a.count,
        }).collect();
        // also seed suggestions from anomalies
        for pa in &pattern_anomalies {
            let priority = if pa.kind == "NewPattern" { "HIGH" } else { "LOW" };
            suggestions.push(SuggestionOut {
                priority: priority.into(),
                description: format!("{}: {}", pa.kind, pa.template),
                query: SuggestQuery {
                    command: "GET_LINES_BY_PATTERN".into(),
                    params: SuggestParams { start: None, end: None, pattern: Some(pa.template.clone()) },
                },
            });
        }
        // De-duplicate suggestions by highest priority
        let mut best: std::collections::HashMap<String, SuggestionOut> = std::collections::HashMap::new();
        fn prio_rank(p: &str) -> i32 { match p { "HIGH" => 3, "MEDIUM" => 2, _ => 1 } }
        for s in suggestions.into_iter() {
            let key = format!("{}|{}|{}|{}", s.query.command, s.query.params.start.clone().unwrap_or_default(), s.query.params.end.clone().unwrap_or_default(), s.query.params.pattern.clone().unwrap_or_default());
            if let Some(existing) = best.get(&key) { if prio_rank(&s.priority) <= prio_rank(&existing.priority) { continue; } }
            best.insert(key, s);
        }
        let mut deduped: Vec<SuggestionOut> = best.into_values().collect();
        deduped.sort_by(|a,b| prio_rank(&b.priority).cmp(&prio_rank(&a.priority)));
        let query_interface = QueryInterfaceOut {
            available_commands: vec!["GET_LINES_BY_PATTERN".into(), "GET_LINES_BY_TIME".into(), "GET_CONTEXT".into()],
            suggested_investigations: deduped,
        };

        // Field/temporal anomalies that require all lines are omitted in streaming finalize to keep memory constant.
        let anomalies = AnomaliesOut {
            pattern_anomalies: pattern_anomalies.clone(),
            field_anomalies: Vec::new(),
            temporal_anomalies: Vec::new(),
        };

        AiOutput {
            summary: Summary { total_lines: total, unique_patterns: unique, compression_ratio, start_date, end_date },
            patterns,
            schema_changes,
            anomalies,
            query_interface,
            errors: ErrorsOut { total: self.error_samples.len(), samples: self.error_samples },
        }
    }
}

/// Pre-compile regex patterns to avoid first-use contention in parallel processing
pub fn prewarm_regexes() {
    // Force initialization of template field pattern
    let _ = &*TEMPLATE_FIELD_PATTERN;
}
