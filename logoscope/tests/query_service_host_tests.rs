#[test]
fn query_by_service_and_host() {
    let mut idx = logoscope::query::QueryIndex::new();
    let l1 = r#"{"level":"info","time":"2024-01-01T00:00:00Z","service":"auth","host":"h1","msg":"a"}"#;
    let l2 = r#"{"level":"info","time":"2024-01-01T00:01:00Z","service":"auth","host":"h2","msg":"a"}"#;
    let l3 = r#"{"level":"info","time":"2024-01-01T00:02:00Z","service":"billing","host":"h3","msg":"b"}"#;
    idx.push_line(l1);
    idx.push_line(l2);
    idx.push_line(l3);
    let s = idx.get_lines_by_service("auth");
    assert_eq!(s.len(), 2);
    let h = idx.get_lines_by_host("h1");
    assert_eq!(h.len(), 1);
}

