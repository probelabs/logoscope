#[test]
fn detects_new_and_rare_patterns() {
    use std::collections::{HashMap, HashSet};
    use logoscope::anomaly::{detect_pattern_anomalies, AnomalyKind};

    let mut counts = HashMap::new();
    counts.insert("A".to_string(), 990);
    counts.insert("B".to_string(), 10);
    let total = 1000usize;

    let mut baseline = HashSet::new();
    baseline.insert("A".to_string());

    let anomalies = detect_pattern_anomalies(&counts, total, &baseline, 0.02); // 2%
    // B is rare (1%) and new
    assert!(anomalies.iter().any(|a| matches!(a.kind, AnomalyKind::NewPattern) && a.template == "B"));
    assert!(anomalies.iter().any(|a| matches!(a.kind, AnomalyKind::RarePattern) && a.template == "B"));
    // A is not rare and not new
    assert!(!anomalies.iter().any(|a| a.template == "A"));
}

