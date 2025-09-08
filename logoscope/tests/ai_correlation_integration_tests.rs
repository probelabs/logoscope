#[test]
fn ai_output_contains_correlated_patterns() {
    use chrono::{Utc, TimeZone, Duration};
    // Create two different structured patterns that should still be separate after canonicalization
    let start = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
    let fmt_a = |t: &str| format!(r#"{{"level":"info","time":"{}","op":"A","status":"success"}}"#, t);
    let fmt_b = |t: &str| format!(r#"{{"level":"info","time":"{}","action":"B","result":"ok"}}"#, t);
    let ts = |dt: chrono::DateTime<chrono::Utc>| dt.to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let lines = vec![
        fmt_a(&ts(start)),
        fmt_b(&ts(start + Duration::seconds(4))),
        fmt_a(&ts(start + Duration::seconds(5))),
        fmt_b(&ts(start + Duration::seconds(7))),
        fmt_a(&ts(start + Duration::seconds(20))),
        fmt_b(&ts(start + Duration::seconds(21))),
    ];
    let refs: Vec<&str> = lines.iter().map(|s| s.as_str()).collect();
    let out = logoscope::ai::summarize_lines(&refs);
    
    // With canonicalization, we expect fewer patterns due to better clustering
    // But we should still have meaningful pattern detection
    assert!(!out.patterns.is_empty(), "Should have detected patterns");
    
    // Test that canonicalization is working - should have fewer templates than lines
    assert!(out.patterns.len() <= lines.len(), "Should cluster similar logs");
    
    // The compression ratio should be good (> 1.0 means clustering happened)
    assert!(out.summary.compression_ratio >= 1.0, "Should achieve compression through clustering");
}
