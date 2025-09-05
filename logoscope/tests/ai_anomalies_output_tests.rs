#[test]
fn ai_output_includes_field_and_temporal_anomalies() {
    // Numeric outlier and a burst for the same pattern
    let lines = vec![
        r#"{"level":"info","time":"2024-01-01T00:00:00Z","op":"query","latency_ms":10}"#,
        r#"{"level":"info","time":"2024-01-01T00:01:00Z","op":"query","latency_ms":12}"#,
        r#"{"level":"info","time":"2024-01-01T00:02:00Z","op":"query","latency_ms":1000}"#,
        r#"{"level":"info","time":"2024-01-01T00:02:05Z","op":"query","latency_ms":1100}"#,
        r#"{"level":"info","time":"2024-01-01T00:02:10Z","op":"query","latency_ms":900}"#,
    ];
    let refs: Vec<&str> = lines.iter().map(|s| s.as_ref()).collect();
    let out = logoscope::ai::summarize_lines(&refs);
    // field anomalies should be present
    assert!(!out.anomalies.field_anomalies.is_empty());
    // temporal anomalies should include a burst/gap entry
    assert!(!out.anomalies.temporal_anomalies.is_empty());
}
