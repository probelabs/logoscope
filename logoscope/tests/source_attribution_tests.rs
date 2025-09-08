#[test]
fn patterns_include_source_breakdown() {
    let lines = vec![
        r#"{"level":"info","time":"2024-01-01T00:00:00Z","service":"auth","host":"h1","op":"A","status":"ok"}"#,
        r#"{"level":"info","time":"2024-01-01T00:01:00Z","service":"auth","host":"h2","op":"A","status":"ok"}"#,
        r#"{"level":"info","time":"2024-01-01T00:02:00Z","service":"billing","host":"h3","action":"B","result":"done"}"#,
    ];
    let refs: Vec<&str> = lines.iter().map(|s| s.as_ref()).collect();
    let out = logoscope::ai::summarize_lines(&refs);
    
    // With canonicalization, we should have meaningful source attribution
    // Test that patterns include source breakdowns
    assert!(!out.patterns.is_empty(), "Should have patterns");
    
    // Find a pattern that has multiple sources
    let mut found_multi_source = false;
    for p in &out.patterns {
        if p.sources.by_service.len() > 0 || p.sources.by_host.len() > 0 {
            found_multi_source = true;
            // Verify source information is present and reasonable
            for svc in &p.sources.by_service {
                assert!(!svc.name.is_empty(), "Service name should not be empty");
                assert!(svc.count > 0, "Service count should be positive");
            }
            for host in &p.sources.by_host {
                assert!(!host.name.is_empty(), "Host name should not be empty");
                assert!(host.count > 0, "Host count should be positive");
            }
        }
    }
    assert!(found_multi_source, "Should have source attribution data");
}

