use chrono::{TimeZone, Utc, Duration};
use std::collections::HashMap;

#[test]
fn computes_pairwise_correlation_in_window() {
    let start = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
    let mut m: HashMap<String, Vec<_>> = HashMap::new();
    // Pattern A at t=0,5,20
    m.insert("A".into(), vec![start, start + Duration::seconds(5), start + Duration::seconds(20)]);
    // Pattern B at t=4,7,21 (close to A within 10s window)
    m.insert("B".into(), vec![start + Duration::seconds(4), start + Duration::seconds(7), start + Duration::seconds(21)]);
    // Pattern C far away
    m.insert("C".into(), vec![start + Duration::seconds(100)]);

    let cors = logoscope::correlation::compute_correlations(&m, Duration::seconds(10));
    // Find A-B pair
    let ab = cors.iter().find(|c| (c.a.as_str()=="A" && c.b.as_str()=="B") || (c.a.as_str()=="B" && c.b.as_str()=="A")).expect("missing A-B");
    assert!(ab.count >= 3);
    assert!(ab.strength > 0.4); // Jaccard over union should be decent
    // Ensure C is weakly correlated
    let ac = cors.iter().find(|c| (c.a.as_str()=="A" && c.b.as_str()=="C") || (c.a.as_str()=="C" && c.b.as_str()=="A")).unwrap();
    assert!(ac.count <= 1);
    assert!(ac.strength < 0.2);
}

