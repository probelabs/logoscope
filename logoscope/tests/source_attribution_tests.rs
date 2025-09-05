#[test]
fn patterns_include_source_breakdown() {
    let lines = vec![
        r#"{"level":"info","time":"2024-01-01T00:00:00Z","service":"auth","host":"h1","op":"A"}"#,
        r#"{"level":"info","time":"2024-01-01T00:01:00Z","service":"auth","host":"h2","op":"A"}"#,
        r#"{"level":"info","time":"2024-01-01T00:02:00Z","service":"billing","host":"h3","op":"B"}"#,
    ];
    let refs: Vec<&str> = lines.iter().map(|s| s.as_ref()).collect();
    let out = logoscope::ai::summarize_lines(&refs);
    let mut found = false;
    for p in &out.patterns {
        if p.template.contains("op=A") {
            found = true;
            let svc = &p.sources.by_service;
            assert!(svc.iter().any(|c| c.name == "auth" && c.count == 2));
            let hosts = &p.sources.by_host;
            assert!(hosts.iter().any(|c| c.name == "h1"));
            assert!(hosts.iter().any(|c| c.name == "h2"));
        }
    }
    assert!(found, "missing pattern for op=A");
}

