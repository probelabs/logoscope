#[test]
fn suggestions_include_burst_time_query() {
    // Construct lines with a clear burst for a single pattern
    let mut lines: Vec<String> = Vec::new();
    lines.push("{\"level\":\"info\",\"time\":\"2024-01-01T00:00:00Z\",\"msg\":\"ok\"}".into());
    lines.push("{\"level\":\"info\",\"time\":\"2024-01-01T00:01:00Z\",\"msg\":\"ok\"}".into());
    // burst window
    lines.push("{\"level\":\"info\",\"time\":\"2024-01-01T00:02:00Z\",\"msg\":\"ok\"}".into());
    lines.push("{\"level\":\"info\",\"time\":\"2024-01-01T00:02:10Z\",\"msg\":\"ok\"}".into());
    lines.push("{\"level\":\"info\",\"time\":\"2024-01-01T00:02:20Z\",\"msg\":\"ok\"}".into());
    let refs: Vec<&str> = lines.iter().map(|s| s.as_str()).collect();
    let out = logoscope::ai::summarize_lines(&refs);
    let qi = out.query_interface;
    assert!(qi.available_commands.contains(&"GET_LINES_BY_TIME".to_string()));
    let any_time = qi.suggested_investigations.iter().any(|s| s.query.command == "GET_LINES_BY_TIME" && s.query.params.start.is_some() && s.query.params.end.is_some());
    assert!(any_time);
}

