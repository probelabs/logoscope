use chrono::{DateTime, Duration, TimeZone, Utc};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq)]
pub struct BurstPeriod {
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub peak_rate: usize,
    pub severity: f64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GapPeriod {
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub duration_seconds: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpikeBucket {
    pub time: DateTime<Utc>,
    pub count: usize,
    pub zscore: f64,
}

pub fn compute_bursts(
    times: &[DateTime<Utc>],
    bucket: Duration,
    burst_multiplier: f64,
) -> Vec<BurstPeriod> {
    if times.is_empty() {
        return vec![];
    }
    // Bucketize counts
    let mut counts: BTreeMap<DateTime<Utc>, usize> = BTreeMap::new();
    for t in times {
        let b = floor_time(*t, bucket);
        *counts.entry(b).or_insert(0) += 1;
    }
    let mut v: Vec<(DateTime<Utc>, usize)> = counts.into_iter().collect();
    v.sort_by_key(|(t, _)| *t);
    let median = median_count(&v);
    let threshold = (median as f64 * burst_multiplier).max(1.0);

    let mut bursts = Vec::new();
    let mut current_start: Option<DateTime<Utc>> = None;
    let mut current_peak: usize = 0;
    let mut current_severity: f64 = 0.0;
    for (i, (t, c)) in v.iter().enumerate() {
        if (*c as f64) >= threshold {
            if current_start.is_none() {
                current_start = Some(*t);
                current_peak = *c;
                current_severity = (*c as f64) / (median.max(1) as f64);
            } else {
                current_peak = current_peak.max(*c);
                current_severity = current_severity.max((*c as f64) / (median.max(1) as f64));
            }
        } else if let Some(start) = current_start {
            // close burst at previous bucket
            let prev_t = v[i - 1].0;
            bursts.push(BurstPeriod {
                start_time: start,
                end_time: prev_t,
                peak_rate: current_peak,
                severity: current_severity,
            });
            current_start = None;
            current_peak = 0;
            current_severity = 0.0;
        }
    }
    if let Some(start) = current_start {
        if let Some((last_t, _)) = v.last() {
            bursts.push(BurstPeriod {
                start_time: start,
                end_time: *last_t,
                peak_rate: current_peak,
                severity: current_severity,
            });
        }
    }

    bursts
}

fn floor_time(t: DateTime<Utc>, bucket: Duration) -> DateTime<Utc> {
    let secs = bucket.num_seconds();
    if secs <= 0 { return t; }
    let ts = t.timestamp();
    let floored = ts - (ts.rem_euclid(secs));
    Utc.timestamp_opt(floored, 0).unwrap()
}

fn median_count(v: &Vec<(DateTime<Utc>, usize)>) -> usize {
    if v.is_empty() { return 0; }
    let mut counts: Vec<usize> = v.iter().map(|(_, c)| *c).collect();
    counts.sort_unstable();
    let mid = counts.len() / 2;
    if counts.len() % 2 == 0 {
        ((counts[mid - 1] + counts[mid]) / 2).max(1)
    } else {
        counts[mid].max(1)
    }
}

pub fn compute_gaps(times: &[DateTime<Utc>], gap_multiplier: f64) -> Vec<GapPeriod> {
    if times.len() < 2 { return vec![]; }
    let mut s = times.to_vec();
    s.sort_unstable();
    let mut gaps_durations: Vec<i64> = Vec::with_capacity(s.len()-1);
    for w in s.windows(2) {
        let d = (w[1] - w[0]).num_seconds();
        gaps_durations.push(d);
    }
    let mut sorted = gaps_durations.clone();
    sorted.sort_unstable();
    let med = if sorted.len() % 2 == 0 {
        (sorted[sorted.len()/2 - 1] + sorted[sorted.len()/2]) / 2
    } else { sorted[sorted.len()/2] };
    let threshold = (med as f64 * gap_multiplier).max(1.0);
    let mut res = Vec::new();
    for i in 0..(s.len()-1) {
        let d = gaps_durations[i] as f64;
        if d >= threshold {
            res.push(GapPeriod { start_time: s[i], end_time: s[i+1], duration_seconds: gaps_durations[i] });
        }
    }
    res
}

pub fn compute_spikes(
    times: &[DateTime<Utc>],
    bucket: Duration,
    z_threshold: f64,
) -> Vec<SpikeBucket> {
    if times.is_empty() { return vec![]; }
    let mut counts: BTreeMap<DateTime<Utc>, usize> = BTreeMap::new();
    for t in times {
        let b = floor_time(*t, bucket);
        *counts.entry(b).or_insert(0) += 1;
    }
    let mut v: Vec<(DateTime<Utc>, usize)> = counts.into_iter().collect();
    v.sort_by_key(|(t, _)| *t);
    let n = v.len() as f64;
    let mean = v.iter().map(|(_, c)| *c as f64).sum::<f64>() / n;
    let var = v
        .iter()
        .map(|(_, c)| {
            let x = *c as f64;
            (x - mean) * (x - mean)
        })
        .sum::<f64>()
        / n;
    let std = var.sqrt();
    let mut res = Vec::new();
    for (t, c) in v {
        if std > 0.0 {
            let z = ((c as f64) - mean) / std;
            if z >= z_threshold {
                res.push(SpikeBucket { time: t, count: c, zscore: z });
            }
        }
    }
    res
}
