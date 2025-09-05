#[test]
fn ai_output_contains_correlated_patterns() {
    use chrono::{Utc, TimeZone, Duration};
    // Create two templates A and B that co-occur in close time
    let start = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
    let fmt = |op: &str, t: &str| format!(r#"{{"level":"info","time":"{}","op":"{}"}}"#, t, op);
    let ts = |dt: chrono::DateTime<chrono::Utc>| dt.to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let lines = vec![
        fmt("A", &ts(start)),
        fmt("B", &ts(start + Duration::seconds(4))),
        fmt("A", &ts(start + Duration::seconds(5))),
        fmt("B", &ts(start + Duration::seconds(7))),
        fmt("A", &ts(start + Duration::seconds(20))),
        fmt("B", &ts(start + Duration::seconds(21))),
    ];
    let refs: Vec<&str> = lines.iter().map(|s| s.as_str()).collect();
    let out = logoscope::ai::summarize_lines(&refs);
    // find pattern templates containing op=A and op=B
    let mut a_corr = None;
    let mut b_corr = None;
    for p in &out.patterns {
        if p.template.contains("op=A") {
            a_corr = Some(p);
        }
        if p.template.contains("op=B") {
            b_corr = Some(p);
        }
    }
    let a = a_corr.expect("missing pattern A");
    let _b = b_corr.expect("missing pattern B");
    // A correlations should include B with decent strength
    let has_b = a.correlations.iter().any(|c| c.template.contains("op=B") && c.strength > 0.3);
    assert!(has_b, "A should correlate with B");
}
