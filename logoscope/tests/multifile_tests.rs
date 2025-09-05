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
    assert_eq!(out.summary.time_span.as_deref(), Some("2024-01-01T00:00:00Z to 2024-01-01T00:03:00Z"));
    // two patterns expected (op=a and op=b)
    assert!(out.patterns.len() >= 2);
}

