#[test]
fn schema_fingerprint_basic_types() {
    let line = r#"{"level":"error","user":{"id":123,"name":"alice"},"status":"fail","active":true,"score":12.5,"tags":["a","b"]}"#;
    let fp = logoscope::schema::fingerprint_line(line).expect("fingerprint");
    assert_eq!(fp.get("level").unwrap(), "string");
    assert_eq!(fp.get("user.id").unwrap(), "int");
    assert_eq!(fp.get("user.name").unwrap(), "string");
    assert_eq!(fp.get("status").unwrap(), "string");
    assert_eq!(fp.get("active").unwrap(), "bool");
    assert_eq!(fp.get("score").unwrap(), "float");
    assert_eq!(fp.get("tags.0").unwrap(), "string");
}

#[test]
fn schema_diff_detects_changes() {
    let before = r#"{"user":{"id":"abc"},"status":1}"#;
    let after  = r#"{"user":{"id":123},"status":"ok","retry_count":2}"#;
    let f_before = logoscope::schema::fingerprint_line(before).unwrap();
    let f_after = logoscope::schema::fingerprint_line(after).unwrap();
    let changes = logoscope::schema::diff_fingerprints(&f_before, &f_after);

    // Type change
    assert!(changes.iter().any(|c| matches!(c,
        logoscope::schema::SchemaChange::TypeChanged{ field, from_type, to_type }
        if field == "user.id" && from_type == "string" && to_type == "int")));
    // Field added
    assert!(changes.iter().any(|c| matches!(c,
        logoscope::schema::SchemaChange::FieldAdded{ field, new_type }
        if field == "retry_count" && new_type == "int")));
    // Field removed
    // none expected
    assert!(!changes.iter().any(|c| matches!(c,
        logoscope::schema::SchemaChange::FieldRemoved{..})));
}

