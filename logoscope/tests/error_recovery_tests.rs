#[test]
fn collects_malformed_json_errors_but_continues() {
    let lines = vec![
        "{\"level\":\"info\",\"time\":\"2024-01-01T00:00:00Z\",\"msg\":\"ok\"}",
        "{ this is not valid json",
        "{" ,
        "INFO plain line without json",
    ];
    let refs: Vec<&str> = lines.iter().map(|s| s.as_ref()).collect();
    let out = logoscope::ai::summarize_lines(&refs);
    assert!(out.errors.total > 0, "expected errors to be reported");
    // ensure analysis still proceeds
    assert!(out.summary.total_lines >= 1);
}

