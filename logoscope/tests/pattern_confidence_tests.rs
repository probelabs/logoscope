#[test]
fn pattern_confidence_higher_for_stable_pattern() {
    use chrono::{TimeZone, Utc, Duration};
    // Stable pattern A across 5 minutes, sporadic pattern B once
    let mut lines: Vec<String> = Vec::new();
    let start = Utc.with_ymd_and_hms(2024,1,1,0,0,0).unwrap();
    for m in 0..5 { lines.push(format!("{{\"level\":\"info\",\"time\":\"{}\",\"op\":\"A\"}}", (start+Duration::minutes(m)).to_rfc3339())); }
    lines.push(format!("{{\"level\":\"info\",\"time\":\"{}\",\"op\":\"B\"}}", (start+Duration::minutes(2)).to_rfc3339()));
    let refs: Vec<&str> = lines.iter().map(|s| s.as_ref()).collect();
    let out = logoscope::ai::summarize_lines(&refs);
    let mut conf_a = None; let mut conf_b = None;
    for p in &out.patterns { if p.template.contains("op=A") { conf_a = Some(p.confidence); } if p.template.contains("op=B") { conf_b = Some(p.confidence); } }
    let a = conf_a.expect("missing A");
    let b = conf_b.expect("missing B");
    assert!(a > b, "expected stable pattern A to have higher confidence");
    assert!(a <= 1.0 && a >= 0.0 && b <= 1.0 && b >= 0.0);
}

