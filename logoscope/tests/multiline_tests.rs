#[test]
fn consolidates_stack_trace_into_single_entry() {
    let lines = vec![
        "Sep 05 10:00:00 host app[1]: java.lang.RuntimeException: boom",
        "\tat com.example.Main.method(Main.java:10)",
        "\tat com.example.Other.run(Other.java:20)",
        "Caused by: java.io.IOException: no disk",
        "\tat com.example.IO.read(IO.java:30)",
    ];
    let mut agg = logoscope::multiline::MultiLineAggregator::default();
    let mut out = Vec::new();
    for l in &lines { if let Some(e) = agg.push(l) { out.push(e); } }
    if let Some(e) = agg.finish() { out.push(e); }
    assert_eq!(out.len(), 1);
    assert!(out[0].contains("RuntimeException"));
    assert!(out[0].contains("com.example.Main.method"));
    assert!(out[0].contains("Caused by"));
}

#[test]
fn consolidates_multiline_json() {
    let lines = vec![
        "{",
        "  \"level\": \"info\",",
        "  \"time\": \"2024-01-01T00:00:00Z\"",
        "}",
    ];
    let mut agg = logoscope::multiline::MultiLineAggregator::default();
    let mut out = Vec::new();
    for l in &lines { if let Some(e) = agg.push(l) { out.push(e); } }
    if let Some(e) = agg.finish() { out.push(e); }
    assert_eq!(out.len(), 1);
    assert!(out[0].contains("\"level\": \"info\""));
}

