use chrono::{Datelike, TimeZone, Utc};

#[test]
fn query_by_pattern_and_time_and_context() {
    let mut idx = logoscope::query::QueryIndex::new();
    let l1 = "Sep 05 10:00:00 host app[1]: User 123 logged in from 192.168.1.1";
    let l2 = "Sep 05 10:00:30 host app[1]: User 456 logged in from 10.0.0.5";
    let l3 = "Sep 05 10:01:00 host app[1]: User 789 logged out from 10.0.0.5";
    let _id1 = idx.push_line(l1);
    let id2 = idx.push_line(l2);
    let _id3 = idx.push_line(l3);

    // Pattern query should return first two lines
    let tpl = "User <*> logged in from <*>".to_string();
    let hits = idx.get_lines_by_pattern(&tpl);
    let lines: Vec<&str> = hits.iter().map(|e| e.line.as_str()).collect();
    assert_eq!(lines, vec![l1, l2]);

    // Time range query for first minute returns first two
    let year = Utc::now().year() as i32;
    let day = Utc.with_ymd_and_hms(year, 9, 5, 10, 0, 0).unwrap();
    let hits = idx.get_lines_by_time(day, day + chrono::Duration::minutes(1), None);
    let lines: Vec<&str> = hits.iter().map(|e| e.line.as_str()).collect();
    assert_eq!(lines, vec![l1, l2]);

    // Context around middle line: 1 before, 1 after
    let ctx = idx.get_context(id2, 1, 1);
    let lines: Vec<&str> = ctx.iter().map(|e| e.line.as_str()).collect();
    assert_eq!(lines, vec![l1, l2, l3]);
}
