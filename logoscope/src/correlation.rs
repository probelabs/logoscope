use chrono::{DateTime, Duration, Utc};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub struct Correlation {
    pub a: String,
    pub b: String,
    pub count: usize,
    pub strength: f64,
}

pub fn compute_correlations(
    times_by_template: &HashMap<String, Vec<DateTime<Utc>>>,
    window: Duration,
) -> Vec<Correlation> {
    let mut out = Vec::new();
    let keys: Vec<String> = times_by_template.keys().cloned().collect();
    for i in 0..keys.len() {
        for j in (i + 1)..keys.len() {
            let a = &keys[i];
            let b = &keys[j];
            let mut ta = times_by_template.get(a).cloned().unwrap_or_default();
            let mut tb = times_by_template.get(b).cloned().unwrap_or_default();
            if ta.is_empty() || tb.is_empty() { continue; }
            ta.sort_unstable();
            tb.sort_unstable();
            let count = cooccurrence_count(&ta, &tb, window);
            let union = ta.len() + tb.len() - count;
            let strength = if union > 0 { (count as f64) / (union as f64) } else { 0.0 };
            out.push(Correlation { a: a.clone(), b: b.clone(), count, strength });
        }
    }
    out
}

fn cooccurrence_count(a: &[DateTime<Utc>], b: &[DateTime<Utc>], window: Duration) -> usize {
    let mut i = 0usize;
    let mut j = 0usize;
    let mut count = 0usize;
    while i < a.len() && j < b.len() {
        let dt = a[i] - b[j];
        let diff = dt.num_milliseconds().abs();
        if diff <= window.num_milliseconds() {
            count += 1;
            // advance both to avoid double counting close pairs repeatedly
            i += 1;
            j += 1;
        } else if a[i] < b[j] {
            i += 1;
        } else {
            j += 1;
        }
    }
    count
}

