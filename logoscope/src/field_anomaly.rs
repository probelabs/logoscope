use crate::{masking, parser};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct NumericOutlier {
    pub template: String,
    pub field: String,
    pub value: f64,
    pub median: f64,
    pub mad: f64,
    pub robust_z: f64,
    pub line_index: usize,
}

#[derive(Debug, Clone)]
pub struct CategoricalExplosion {
    pub template: String,
    pub field: String,
    pub unique_count: usize,
    pub total: usize,
    pub ratio: f64,
}

pub fn analyze_numeric_outliers(lines: &[&str], z_threshold: f64) -> Vec<NumericOutlier> {
    // Group numeric field values by (template, field)
    let mut values: HashMap<(String, String), Vec<(usize, f64)>> = HashMap::new();
    let mut templates: Vec<String> = Vec::with_capacity(lines.len());
    for (i, l) in lines.iter().enumerate() {
        let rec = parser::parse_line(l, i + 1);
        // Build template from JSON synthetic message if present, else from masked message
        let base = if let Some(syn) = rec.synthetic_message {
            syn
        } else {
            rec.message
        };
        let masked = masking::mask_text(&base);
        let template = to_generic_template(&masked);
        templates.push(template.clone());
        if let Some(fields) = rec.flat_fields {
            for (k, v) in fields.iter() {
                if let Some(num) = parse_number(v) {
                    values.entry((template.clone(), k.clone())).or_default().push((i, num));
                }
            }
        }
    }

    let mut anomalies = Vec::new();
    for ((template, field), series) in values.into_iter() {
        if series.len() < 5 { continue; } // need minimal series
        let mut xs: Vec<f64> = series.iter().map(|(_, x)| *x).collect();
        xs.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let median = percentile(&xs, 50.0);
        // compute MAD: median(|x - median|)
        let mut absdev: Vec<f64> = xs.iter().map(|x| (x - median).abs()).collect();
        absdev.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let mad_raw = percentile(&absdev, 50.0);
        let mad = if mad_raw == 0.0 { 1e-9 } else { mad_raw };
        for (idx, x) in series.iter() {
            let z = 0.6745 * (x - median).abs() / mad;
            if z >= z_threshold {
                anomalies.push(NumericOutlier { template: template.clone(), field: field.clone(), value: *x, median, mad: mad_raw, robust_z: z, line_index: *idx });
            }
        }
    }
    anomalies
}

pub fn analyze_categorical_explosions(
    lines: &[&str],
    ratio_threshold: f64,
    min_total: usize,
) -> Vec<CategoricalExplosion> {
    // Count unique categorical values per (template, field)
    let mut sets: HashMap<(String, String), HashSet<String>> = HashMap::new();
    let mut totals: HashMap<(String, String), usize> = HashMap::new();
    for (i, l) in lines.iter().enumerate() {
        let rec = parser::parse_line(l, i + 1);
        let base = if let Some(syn) = rec.synthetic_message { syn } else { rec.message };
        let masked = masking::mask_text(&base);
        let template = to_generic_template(&masked);
        if let Some(fields) = rec.flat_fields {
            for (k, v) in fields.iter() {
                // Only categorical: strings that are not numbers
                if parse_number(v).is_none() {
                    sets.entry((template.clone(), k.clone())).or_default().insert(v.clone());
                    *totals.entry((template.clone(), k.clone())).or_default() += 1;
                }
            }
        }
    }
    let mut out = Vec::new();
    for ((template, field), set) in sets.into_iter() {
        let total = *totals.get(&(template.clone(), field.clone())).unwrap_or(&0);
        if total >= min_total {
            let ratio = (set.len() as f64) / (total as f64);
            if ratio >= ratio_threshold {
                out.push(CategoricalExplosion { template, field, unique_count: set.len(), total, ratio });
            }
        }
    }
    out
}

fn parse_number(s: &str) -> Option<f64> {
    if let Ok(i) = s.parse::<i64>() { return Some(i as f64); }
    if let Ok(f) = s.parse::<f64>() { return Some(f); }
    None
}

fn percentile(xs: &[f64], p: f64) -> f64 {
    if xs.is_empty() { return 0.0; }
    let rank = ((p / 100.0) * ((xs.len() - 1) as f64)) as usize;
    xs[rank]
}

fn to_generic_template(masked: &str) -> String {
    masked
        .replace("<NUM>", "<*>")
        .replace("<IP>", "<*>")
        .replace("<EMAIL>", "<*>")
        .replace("<TIMESTAMP>", "<*>")
}
