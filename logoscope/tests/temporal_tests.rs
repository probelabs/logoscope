use chrono::{Duration, TimeZone, Utc};

#[test]
fn detects_simple_burst_periods() {
    let start = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
    let mut times = Vec::new();
    for m in 0..20 {
        if (10..=11).contains(&m) {
            // burst minutes: 5 events each
            for s in 0..5 {
                times.push(start + Duration::minutes(m) + Duration::seconds(s));
            }
        } else {
            times.push(start + Duration::minutes(m));
        }
    }

    let bursts = logoscope::temporal::compute_bursts(&times, Duration::minutes(1), 3.0);
    assert_eq!(bursts.len(), 1);
    let b = &bursts[0];
    assert_eq!(b.start_time, start + Duration::minutes(10));
    assert_eq!(b.end_time, start + Duration::minutes(11));
    assert_eq!(b.peak_rate, 5);
    assert!(b.severity >= 5.0); // median=1, severity >= 5x
}

#[test]
fn detects_large_gaps() {
    let start = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
    let times = vec![
        start,
        start + Duration::minutes(1),
        start + Duration::minutes(2),
        start + Duration::minutes(32), // 30-minute gap
        start + Duration::minutes(33),
    ];
    let gaps = logoscope::temporal::compute_gaps(&times, 10.0);
    assert_eq!(gaps.len(), 1);
    let g = &gaps[0];
    assert_eq!(g.start_time, start + Duration::minutes(2));
    assert_eq!(g.end_time, start + Duration::minutes(32));
    assert!(g.duration_seconds >= 1800);
}

#[test]
fn detects_frequency_spikes() {
    let start = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
    let mut times = Vec::new();
    for m in 0..60 {
        if m == 42 {
            for s in 0..10 {
                times.push(start + Duration::minutes(m) + Duration::seconds(s));
            }
        } else {
            times.push(start + Duration::minutes(m));
        }
    }
    let spikes = logoscope::temporal::compute_spikes(&times, Duration::minutes(1), 3.0);
    assert_eq!(spikes.len(), 1);
    let s = &spikes[0];
    assert_eq!(s.time, start + Duration::minutes(42));
    assert!(s.count >= 10);
    assert!(s.zscore >= 3.0);
}
