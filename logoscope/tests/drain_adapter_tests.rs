#[test]
fn drain_clusters_masked_lines() {
    let lines = [
        "User 123 logged in from 192.168.1.1 at 2024-01-01T12:00:00Z",
        "User 456 logged in from 10.0.0.5 at 2024-01-01T12:01:00Z",
        "User 789 logged out from 10.0.0.5 at 2024-01-01T12:02:00Z",
    ];
    let masked: Vec<String> = lines.iter().map(|l| logoscope::masking::mask_text(l)).collect();
    let mut drain = logoscope::drain_adapter::DrainAdapter::new_default();
    for m in &masked {
        drain.insert(m).unwrap();
    }
    let mut clusters = drain.clusters();
    clusters.sort_by(|a, b| a.template.cmp(&b.template));
    assert_eq!(clusters.len(), 2);
    assert_eq!(clusters[0].template.as_str(), "User <*> logged in from <*> at <*>");
    assert_eq!(clusters[0].size, 2);
    assert_eq!(clusters[1].template.as_str(), "User <*> logged out from <*> at <*>");
    assert_eq!(clusters[1].size, 1);
}

