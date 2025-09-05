#[test]
fn detects_numeric_outlier_with_median_mad() {
    // Mostly tight latencies, one large outlier
    let base = |ms: i64| format!(r#"{{"level":"info","time":"2024-01-01T00:00:00Z","op":"query","latency_ms":{}}}"#, ms);
    let mut lines: Vec<String> = vec![10, 11, 9, 10, 10, 12].into_iter().map(base).collect();
    lines.push(base(1000));
    let refs: Vec<&str> = lines.iter().map(|s| s.as_str()).collect();

    let anomalies = logoscope::field_anomaly::analyze_numeric_outliers(&refs, 3.5);
    assert_eq!(anomalies.len(), 1);
    let a = &anomalies[0];
    assert_eq!(a.field, "latency_ms");
    assert!(a.value >= 1000.0);
    assert!(a.robust_z >= 3.5);
}

#[test]
fn detects_categorical_cardinality_explosion() {
    // Many unique request_id values under same pattern
    let base = |id: i32| format!(r#"{{"level":"info","time":"2024-01-01T00:00:00Z","op":"get","request_id":"req-{:04}"}}"#, id);
    let mut lines: Vec<String> = (0..20).map(base).collect();
    let refs: Vec<&str> = lines.iter().map(|s| s.as_str()).collect();

    let explosions = logoscope::field_anomaly::analyze_categorical_explosions(&refs, 0.8, 10);
    assert_eq!(explosions.len(), 1);
    let e = &explosions[0];
    assert_eq!(e.field, "request_id");
    assert!(e.ratio >= 0.8);
    assert_eq!(e.total, 20);
}

