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
    // patterns
    assert!(out.patterns.len() >= 2);
    // find a pattern with severity "info" and count 2
    let mut info_ok = false;
    for p in &out.patterns {
        if p.severity.as_deref() == Some("info") && p.total_count == 2 {
            info_ok = true;
        }
        assert!(p.frequency > 0.0);
        assert!(!p.template.is_empty());
        assert!(!p.examples.is_empty());
    }
    assert!(info_ok);
    // schema changes: status type changed, retry_count added
    let mut type_changed = false;
    let mut field_added = false;
    for c in &out.schema_changes {
        match c.change_type.as_str() {
            "type_changed" => {
                if c.field == "status" { type_changed = true; }
            }
            "field_added" => {
                if c.field == "retry_count" { field_added = true; }
            }
            _ => {}
        }
    }
    assert!(type_changed);
    assert!(field_added);
}

