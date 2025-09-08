#[test]
fn multi_file_analysis_combines_timespan_and_counts() {
    // Two synthetic files with interleaved timestamps
    let f1 = vec![
        r#"{"level":"info","time":"2024-01-01T00:00:00Z","op":"a"}"#,
        r#"{"level":"info","time":"2024-01-01T00:02:00Z","op":"a"}"#,
    ];
    let f2 = vec![
        r#"{"level":"error","time":"2024-01-01T00:01:00Z","op":"b"}"#,
        r#"{"level":"error","time":"2024-01-01T00:03:00Z","op":"b"}"#,
    ];
    let mut lines = Vec::new();
    lines.extend(f1.iter().copied());
    lines.extend(f2.iter().copied());
    let refs: Vec<&str> = lines.iter().map(|s| s.as_ref()).collect();
    let out = logoscope::ai::summarize_lines(&refs);
    assert_eq!(out.summary.total_lines, 4);
    assert_eq!(out.summary.start_date.as_deref(), Some("2024-01-01T00:00:00Z"));
    assert_eq!(out.summary.end_date.as_deref(), Some("2024-01-01T00:03:00Z"));
    // With canonicalization, logs with same structure should cluster together
    // So we expect good compression ratio instead of separate patterns
    assert!(out.patterns.len() >= 1, "Should have at least one pattern");
    assert!(out.summary.compression_ratio > 1.0, "Should achieve compression through clustering");
}

