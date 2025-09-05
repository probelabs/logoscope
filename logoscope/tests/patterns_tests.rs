#[test]
fn clusters_plaintext_masked_lines_into_templates() {
    let l1 = "User 123 logged in from 192.168.1.1 at 2024-01-01T12:00:00Z";
    let l2 = "User 456 logged in from 10.0.0.5 at 2024-01-01T14:00:00Z";
    let l3 = "User 789 logged out from 10.0.0.5 at 2024-01-01T14:05:00Z";
    let m1 = logoscope::masking::mask_text(l1);
    let m2 = logoscope::masking::mask_text(l2);
    let m3 = logoscope::masking::mask_text(l3);
    let clusters = logoscope::patterns::cluster_masked(&[m1, m2, m3]);
    // Expect 2 clusters: logged in vs logged out
    assert_eq!(clusters.len(), 2);
    // Find template for logged in
    let mut templates: Vec<_> = clusters.iter().map(|c| (&c.template, c.count)).collect();
    templates.sort_by_key(|(t, _)| t.as_str().to_owned());
    assert_eq!(templates[0].0.as_str(), "User <*> logged in from <*> at <*>");
    assert_eq!(templates[0].1, 2);
    assert_eq!(templates[1].0.as_str(), "User <*> logged out from <*> at <*>");
    assert_eq!(templates[1].1, 1);
}
