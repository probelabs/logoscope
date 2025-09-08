use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnomalyKind {
    NewPattern,
    RarePattern,
}

#[derive(Debug, Clone)]
pub struct PatternAnomaly {
    pub kind: AnomalyKind,
    pub template: String,
    pub frequency: f64,
    pub count: usize,
}

pub fn detect_pattern_anomalies(
    counts: &HashMap<String, usize>,
    total: usize,
    baseline_templates: &HashSet<String>,
    rare_threshold: f64,
) -> Vec<PatternAnomaly> {
    let mut out = Vec::new();
    if total == 0 { return out; }
    for (tpl, &count) in counts.iter() {
        let freq = (count as f64) / (total as f64);
        // Only emit NewPattern when there is a non-empty baseline; otherwise batch mode would mark everything new.
        if !baseline_templates.is_empty() && !baseline_templates.contains(tpl) {
            out.push(PatternAnomaly { kind: AnomalyKind::NewPattern, template: tpl.clone(), frequency: freq, count });
        }
        // Check for rare patterns independently (a pattern can be both new and rare)
        if freq < rare_threshold {
            out.push(PatternAnomaly { kind: AnomalyKind::RarePattern, template: tpl.clone(), frequency: freq, count });
        }
    }
    out
}
