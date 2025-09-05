// no extra imports needed

#[test]
fn builds_basic_ai_summary() {
    let lines = vec![
        "{\"level\":\"error\",\"time\":\"2024-01-01T00:00:00Z\",\"msg\":\"db fail\"}",
        "{\"level\":\"info\",\"time\":\"2024-01-01T00:01:00Z\",\"msg\":\"ok\"}",
        "{\"level\":\"error\",\"time\":\"2024-01-01T00:02:00Z\",\"msg\":\"db fail\"}",
    ];
    let out = logoscope::ai::summarize_lines(&lines);
    assert_eq!(out.summary.total_lines, 3);
    assert!(out.summary.unique_patterns >= 1);
    assert!(out.summary.compression_ratio >= 1.0);
    assert_eq!(
        out.summary.time_span.as_deref(),
        Some("2024-01-01T00:00:00Z to 2024-01-01T00:02:00Z")
    );
}
