use std::collections::HashMap;
use chrono::{DateTime, Utc};

// Re-export types from ai module that analyzers need
use crate::ai::{ParameterAnomaly, DeepTemporalOut, DeepCorrelation, ParamFieldStats, CorrelatedOut};

/// Common data structure passed to all analyzers
#[derive(Debug, Clone)]
pub struct AnalysisContext {
    pub template: String,
    pub clean_template: String,
    pub total_count: usize,
    pub timestamps: Vec<DateTime<Utc>>,
    pub line_params: Vec<HashMap<String, Vec<String>>>,
    pub pattern_indices: Vec<usize>,
    pub param_stats: Option<HashMap<String, ParamFieldStats>>,
}

/// Complete pattern data needed to build a PatternOut
#[derive(Debug, Clone)]
pub struct PatternData {
    pub template: String,
    pub total_count: usize,
    pub frequency: f64,
    pub examples: Vec<String>,
    pub severity: Option<String>,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub spike_analysis: Option<crate::ai::SpikeAnalysis>,
    pub temporal: Option<crate::ai::TemporalOut>,
    pub correlations: Vec<CorrelatedOut>,
    pub pattern_stability: f64,
    pub service_breakdown: Vec<crate::ai::CountItem>,
    pub host_breakdown: Vec<crate::ai::CountItem>,
    pub drain_template: Option<String>,
    pub param_stats: Option<HashMap<String, ParamFieldStats>>,
    pub timestamps: Vec<DateTime<Utc>>,
    pub line_params: Vec<HashMap<String, Vec<String>>>,
    pub pattern_indices: Vec<usize>,
}

/// Results from all analyzers combined
#[derive(Debug, Clone, Default)]
pub struct AnalysisResults {
    pub parameter_anomalies: Option<Vec<ParameterAnomaly>>,
    pub deep_temporal: Option<DeepTemporalOut>,
    pub deep_correlations: Option<Vec<DeepCorrelation>>,
}

/// Trait that all analyzers must implement
pub trait Analyzer: Send + Sync {
    fn name(&self) -> &'static str;
    fn analyze(&self, context: &AnalysisContext, opts: &crate::ai::SummarizeOpts) -> Box<dyn AnalysisResult>;
}

/// Base trait for analysis results
pub trait AnalysisResult: Send + Sync {
    fn merge_into(self: Box<Self>, results: &mut AnalysisResults);
}

/// Parameter anomaly analyzer
pub struct ParameterAnomalyAnalyzer;

/// Helper function to get the base parameter type from potentially numbered parameters
/// e.g., "NUM_2" -> "NUM", "IP_3" -> "IP", "NUM" -> "NUM"
pub fn get_base_param_type(param_type: &str) -> &str {
    if let Some(underscore_pos) = param_type.rfind('_') {
        let suffix = &param_type[underscore_pos + 1..];
        // Check if suffix is a number
        if suffix.chars().all(|c| c.is_ascii_digit()) {
            return &param_type[..underscore_pos];
        }
    }
    param_type
}

/// Helper function to check if a parameter should be treated as high-cardinality numeric
fn is_high_cardinality_numeric(param_type: &str, cardinality: usize, total: usize) -> bool {
    let base_type = get_base_param_type(param_type);
    base_type == "NS" || (base_type == "NUM" && cardinality as f64 / total as f64 > 0.9)
}

impl Analyzer for ParameterAnomalyAnalyzer {
    fn name(&self) -> &'static str {
        "parameter_anomaly"
    }

    fn analyze(&self, context: &AnalysisContext, _opts: &crate::ai::SummarizeOpts) -> Box<dyn AnalysisResult> {
        let mut param_anoms = Vec::new();
        
        if let Some(param_stats) = &context.param_stats {
            for (param_type, stats) in param_stats {
                let total = stats.total;
                if total == 0 { continue; }
                
                let cardinality = stats.cardinality;
                let top_ratio = stats.top_ratio;
                
                // Skip anomaly detection for time-based parameters (naturally unique)
                let base_param_type = get_base_param_type(param_type);
                let is_time_param = base_param_type == "TIME" || base_param_type == "TIMESTAMP" || 
                                   base_param_type == "DATE" || base_param_type == "DATETIME";
                
                // Skip anomaly detection for high-cardinality numeric parameters (like nanoseconds)
                let is_high_card_numeric = is_high_cardinality_numeric(param_type, cardinality, total);
                
                // Skip anomaly detection for sequences - use specialized sequence anomaly detection
                if let Some(true) = stats.is_sequence {
                    // Add sequence anomaly detection if needed
                    if let Some(ref seq_info) = stats.sequence_info {
                        // Detect anomalous sequences (e.g., sudden gaps, unexpected jumps)
                        if seq_info.coverage_ratio < 0.9 {
                            param_anoms.push(ParameterAnomaly {
                                anomaly_type: "sequence_anomaly".to_string(),
                                param: param_type.clone(),
                                value: format!("{} → {} ({}% coverage)", 
                                    seq_info.start_value, seq_info.end_value,
                                    (seq_info.coverage_ratio * 100.0) as i32),
                                count: Some(total),
                                ratio: Some(seq_info.coverage_ratio),
                                details: format!("Sequence has gaps: {} to {} with {}% coverage (step: {})",
                                    seq_info.start_value, seq_info.end_value,
                                    (seq_info.coverage_ratio * 100.0) as i32, seq_info.step_size),
                            });
                        }
                        
                        // Detect unusual step sizes (very large jumps)
                        if seq_info.step_size.abs() > 1000 {
                            param_anoms.push(ParameterAnomaly {
                                anomaly_type: "large_sequence_step".to_string(),
                                param: param_type.clone(),
                                value: seq_info.step_size.to_string(),
                                count: Some(total),
                                ratio: None,
                                details: format!("Sequence has unusually large step size: {} (range: {} to {})",
                                    seq_info.step_size, seq_info.start_value, seq_info.end_value),
                            });
                        }
                    }
                    continue; // Skip traditional anomaly detection for sequences
                }
                
                // Skip anomaly detection for time-based or high-cardinality numeric parameters
                if is_time_param || is_high_card_numeric {
                    continue;
                }
                
                // Detect anomalies (removed single_value check as it's normal behavior)
                if top_ratio >= 0.9 && context.total_count > 10 && cardinality > 1 {
                    // Report the concentration
                    param_anoms.push(ParameterAnomaly {
                        anomaly_type: "value_concentration".to_string(),
                        param: param_type.clone(),
                        value: stats.values.first().map(|v| v.value.clone()).unwrap_or_default(),
                        count: None,
                        ratio: Some(top_ratio),
                        details: format!("{}% of {} '{}' values are '{}'", 
                            (top_ratio * 100.0) as i32, total, param_type, 
                            stats.values.first().map(|v| &v.value).unwrap_or(&String::new())),
                    });
                    
                    // ALSO report the minority values as outliers (the other side of concentration)
                    for value_info in stats.values.iter().skip(1) {  // Skip the concentrated value, check all others
                        let ratio = value_info.count as f64 / total as f64;
                        if ratio <= 0.1 {  // If less than 10% (when one value has 90%+)
                            param_anoms.push(ParameterAnomaly {
                                anomaly_type: "outlier".to_string(),
                                param: param_type.clone(),
                                value: value_info.value.clone(),
                                count: Some(value_info.count),
                                ratio: Some(ratio),
                                details: format!("Rare '{}' value '{}' appears only {} time(s) out of {} ({}%)",
                                    param_type, value_info.value, value_info.count, total, (ratio * 100.0) as i32),
                            });
                        }
                    }
                } else if cardinality > 1 && cardinality <= 3 && total >= 100 {
                    // Only flag low cardinality if we have 2-3 unique values (not 1, since single values are replaced in template)
                    param_anoms.push(ParameterAnomaly {
                        anomaly_type: "low_cardinality".to_string(),
                        param: param_type.clone(),
                        value: format!("{cardinality} unique values"),
                        count: Some(total),
                        ratio: None,
                        details: format!("Only {cardinality} distinct values seen across {total} occurrences of '{param_type}'"),
                    });
                } else if cardinality >= 4 && total >= 20 {
                    // Compute stats to check for balanced/imbalanced distribution
                    let has_imbalance = {
                        let mut sorted_counts: Vec<usize> = stats.values.iter().map(|v| v.count).collect();
                        sorted_counts.sort();
                        let highest = sorted_counts.last().copied().unwrap_or(0);
                        let second_highest = if sorted_counts.len() >= 2 { sorted_counts[sorted_counts.len() - 2] } else { 0 };
                        highest > second_highest * 3  // Imbalance if top value is 3x more than second
                    };
                    // Check each value for outlier detection
                    for value_info in &stats.values {
                        let ratio = value_info.count as f64 / total as f64;
                        // Flag as outlier if: appears <= 5% of time AND there's imbalance in distribution
                        // Removed the count <= 2 restriction to catch more outliers
                        if ratio <= 0.05 && has_imbalance {
                            param_anoms.push(ParameterAnomaly {
                                anomaly_type: "outlier".to_string(),
                                param: param_type.clone(),
                                value: value_info.value.clone(),
                                count: Some(value_info.count),
                                ratio: Some(ratio),
                                details: format!("Rare '{}' value '{}' appears only {} time(s) out of {} ({}%)",
                                    param_type, value_info.value, value_info.count, total, (ratio * 100.0) as i32),
                            });
                        }
                    }
                }
                
                // Special alert for security-relevant parameters
                if base_param_type == "IP" && cardinality == 1 && total >= 100 {
                    param_anoms.push(ParameterAnomaly {
                        anomaly_type: "SECURITY_ALERT".to_string(),
                        param: param_type.clone(),
                        value: stats.values.first().map(|v| v.value.clone()).unwrap_or_default(),
                        count: Some(total),
                        ratio: None,
                        details: format!("All {} requests from single IP: {} - possible bot/attack", 
                            total, stats.values.first().map(|v| &v.value).unwrap_or(&String::new())),
                    });
                }
            }
        }
        
        Box::new(ParameterAnomalyResult { anomalies: param_anoms })
    }
}

pub struct ParameterAnomalyResult {
    anomalies: Vec<ParameterAnomaly>,
}

impl AnalysisResult for ParameterAnomalyResult {
    fn merge_into(self: Box<Self>, results: &mut AnalysisResults) {
        if !self.anomalies.is_empty() {
            results.parameter_anomalies = Some(self.anomalies);
        }
    }
}

/// Deep temporal analyzer
pub struct DeepTemporalAnalyzer;

impl Analyzer for DeepTemporalAnalyzer {
    fn name(&self) -> &'static str {
        "deep_temporal"
    }

    fn analyze(&self, context: &AnalysisContext, _opts: &crate::ai::SummarizeOpts) -> Box<dyn AnalysisResult> {
        if !context.timestamps.is_empty() && context.timestamps.len() == context.line_params.len() {
            let deep_temporal = crate::ai::compute_deep_temporal(
                &context.timestamps, 
                &context.clean_template, 
                &context.line_params, 
                &context.pattern_indices
            );
            Box::new(DeepTemporalResult { analysis: Some(deep_temporal) })
        } else {
            Box::new(DeepTemporalResult { analysis: None })
        }
    }
}

pub struct DeepTemporalResult {
    analysis: Option<DeepTemporalOut>,
}

impl AnalysisResult for DeepTemporalResult {
    fn merge_into(self: Box<Self>, results: &mut AnalysisResults) {
        results.deep_temporal = self.analysis;
    }
}

/// Deep correlation analyzer
pub struct DeepCorrelationAnalyzer;

impl Analyzer for DeepCorrelationAnalyzer {
    fn name(&self) -> &'static str {
        "deep_correlation"
    }

    fn analyze(&self, _context: &AnalysisContext, _opts: &crate::ai::SummarizeOpts) -> Box<dyn AnalysisResult> {
        // For correlation analysis, we need access to all templates, not just current one
        // This would need to be passed in the context or handled differently
        // For now, return empty correlations
        Box::new(DeepCorrelationResult { correlations: Vec::new() })
    }
}

pub struct DeepCorrelationResult {
    correlations: Vec<DeepCorrelation>,
}

impl AnalysisResult for DeepCorrelationResult {
    fn merge_into(self: Box<Self>, results: &mut AnalysisResults) {
        if !self.correlations.is_empty() {
            results.deep_correlations = Some(self.correlations);
        }
    }
}

/// Main analyzer registry that manages all analyzers
pub struct AnalyzerRegistry {
    analyzers: Vec<Box<dyn Analyzer>>,
}

impl Default for AnalyzerRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl AnalyzerRegistry {
    pub fn new() -> Self {
        Self {
            analyzers: vec![
                Box::new(ParameterAnomalyAnalyzer),
                Box::new(DeepTemporalAnalyzer),
                Box::new(DeepCorrelationAnalyzer),
            ],
        }
    }

    pub fn analyze(&self, context: &AnalysisContext, opts: &crate::ai::SummarizeOpts) -> AnalysisResults {
        let mut results = AnalysisResults::default();
        
        for analyzer in &self.analyzers {
            let analysis_result = analyzer.analyze(context, opts);
            analysis_result.merge_into(&mut results);
        }
        
        results
    }
    
    /// Unified pattern builder that both chunked and non-chunked modes can use
    pub fn build_pattern(
        pattern_data: PatternData, 
        opts: &crate::ai::SummarizeOpts,
        _total_lines: usize,
        times_by_template: Option<&std::collections::HashMap<String, Vec<DateTime<Utc>>>>
    ) -> crate::ai::PatternOut {
        
        // Create analysis context
        let analysis_context = AnalysisContext {
            template: pattern_data.template.clone(),
            clean_template: pattern_data.template.clone(), // Will be cleaned below
            total_count: pattern_data.total_count,
            timestamps: pattern_data.timestamps.clone(),
            line_params: pattern_data.line_params.clone(),
            pattern_indices: pattern_data.pattern_indices.clone(),
            param_stats: pattern_data.param_stats.clone(),
        };
        
        // Clean template (remove level suffix for analysis)
        let clean_template = if let Some(bracket_pos) = pattern_data.template.rfind(" [") {
            let suffix = &pattern_data.template[bracket_pos..];
            if suffix.ends_with(']') && !suffix.contains('<') && !suffix.contains('>') {
                pattern_data.template[..bracket_pos].to_string()
            } else {
                pattern_data.template.clone()
            }
        } else {
            pattern_data.template.clone()
        };
        
        // DON'T optimize template here - keep the placeholders!
        // Template optimization should only happen in specific output modes (like triage)
        // For standard JSON output, we want to preserve the generic template with placeholders
        let _optimized_template = clean_template.clone();
        
        // Update analysis context with clean template
        let mut final_context = analysis_context;
        final_context.clean_template = clean_template;
        
        // Run all analyzers
        let registry = AnalyzerRegistry::new();
        let analysis_results = registry.analyze(&final_context, opts);
        
        // Build correlations if we have times_by_template data
        let deep_correlations = if opts.deep {
            if let Some(times_map) = times_by_template {
                Some(crate::ai::compute_deep_correlations(times_map, &pattern_data.template))
            } else {
                analysis_results.deep_correlations
            }
        } else {
            analysis_results.deep_correlations
        };
        
        crate::ai::PatternOut {
            template: pattern_data.template.clone(),  // Use original template with level suffix
            frequency: pattern_data.frequency,
            total_count: pattern_data.total_count,
            severity: pattern_data.severity,
            start_time: pattern_data.start_time,
            end_time: pattern_data.end_time,
            spike_analysis: pattern_data.spike_analysis,
            temporal: pattern_data.temporal,
            examples: pattern_data.examples,
            correlations: pattern_data.correlations,
            pattern_stability: pattern_data.pattern_stability,
            sources: crate::ai::SourceBreakdown { 
                by_service: pattern_data.service_breakdown, 
                by_host: pattern_data.host_breakdown 
            },
            drain_template: pattern_data.drain_template,
            param_stats: pattern_data.param_stats,
            parameter_anomalies: analysis_results.parameter_anomalies,
            deep_temporal: analysis_results.deep_temporal,
            deep_correlations,
        }
    }
}