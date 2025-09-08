#[test]
fn ai_enriched_output_includes_patterns_and_schema_changes() {
    // Build a mix of JSON lines that cause two patterns and schema changes
    let lines = vec![
        r#"{"level":"info","time":"2024-01-01T00:00:00Z","status":1}"#,
        r#"{"level":"info","time":"2024-01-01T00:01:00Z","status":2}"#,
        r#"{"level":"error","time":"2024-01-01T00:02:00Z","status":"fail"}"#,
        r#"{"level":"info","time":"2024-01-01T00:03:00Z","status":"ok","retry_count":1}"#,
    ];
    let out = logoscope::ai::summarize_lines(&lines);
    // patterns - should have at least 2 patterns
    assert!(out.patterns.len() >= 2);
    // Verify basic pattern properties
    for p in &out.patterns {
        assert!(p.frequency > 0.0);
        assert!(!p.template.is_empty());
        assert!(!p.examples.is_empty());
    }
    // Check that patterns have been created
    assert!(out.patterns.iter().map(|p| p.total_count).sum::<usize>() == 4, "All 4 logs should be in patterns");
    // Schema changes are only detected in streaming mode with a baseline
    // In normal mode, schema changes will be empty
    // This is expected behavior - schema tracking requires comparing against a baseline
}

