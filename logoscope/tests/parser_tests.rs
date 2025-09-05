use chrono::{TimeZone, Utc};

#[test]
fn parse_plaintext_line_detects_plaintext_and_message() {
    let line = "INFO something happened";
    let rec = logoscope::parser::parse_line(line, 42);
    assert_eq!(rec.format, logoscope::parser::LogFormat::Plaintext);
    assert_eq!(rec.line_number, 42);
    assert_eq!(rec.message, "INFO something happened");
    assert!(rec.timestamp.is_none());
    assert!(rec.flat_fields.is_none());
}

#[test]
fn parse_json_line_detects_json_flattens_and_extracts_timestamp() {
    let line = r#"{"level":"error","time":"2024-01-15T14:20:00Z","user":{"id":123,"email":"x@y.z"},"op":"login","status":"fail"}"#;
    let rec = logoscope::parser::parse_line(line, 7);
    assert_eq!(rec.format, logoscope::parser::LogFormat::Json);
    assert_eq!(rec.line_number, 7);
    // flattened field assertions
    let fields = rec.flat_fields.as_ref().expect("expected fields");
    assert_eq!(fields.get("level").unwrap(), "error");
    assert_eq!(fields.get("op").unwrap(), "login");
    assert_eq!(fields.get("status").unwrap(), "fail");
    assert_eq!(fields.get("user.id").unwrap(), "123");
    assert_eq!(fields.get("user.email").unwrap(), "x@y.z");
    // timestamp
    let ts = rec.timestamp.expect("timestamp present");
    assert_eq!(ts, Utc.with_ymd_and_hms(2024, 1, 15, 14, 20, 0).unwrap());
    // synthetic message should be stable key=value sorted by key
    assert_eq!(rec.synthetic_message.as_deref().unwrap(),
        "level=error op=login status=fail time=2024-01-15T14:20:00Z user.email=x@y.z user.id=123");
}

#[test]
fn parse_plaintext_syslog_extracts_timestamp() {
    use chrono::Datelike;
    let year = Utc::now().year();
    let line = "Sep 05 14:20:00 host app[123]: ready";
    let rec = logoscope::parser::parse_line(line, 100);
    let expected = Utc.with_ymd_and_hms(year as i32, 9, 5, 14, 20, 0).unwrap();
    assert_eq!(rec.format, logoscope::parser::LogFormat::Plaintext);
    assert_eq!(rec.timestamp.unwrap(), expected);
}

#[test]
fn parse_plaintext_iso_with_offset_extracts_timestamp() {
    let line = "2024-01-01 12:00:00+01:00 service: started";
    let rec = logoscope::parser::parse_line(line, 1);
    assert!(rec.timestamp.is_some());
}

#[test]
fn parse_json_detects_timestamp_without_time_field() {
    // timestamp provided as 'ts_ms' epoch milliseconds
    let line = r#"{"level":"info","ts_ms":1700000000123,"msg":"ok"}"#;
    let rec = logoscope::parser::parse_line(line, 1);
    assert!(rec.timestamp.is_some());
}

#[test]
fn parse_json_uses_timestamp_hints_prioritized() {
    use chrono::TimeZone;
    let line = r#"{"level":"info","time":"2024-01-01T00:00:10Z","ts":1704067200}"#;
    let rec = logoscope::parser::parse_line_with_hints(line, 1, &["ts"]);
    let expected = chrono::Utc.timestamp_opt(1704067200, 0).unwrap();
    assert_eq!(rec.timestamp.unwrap(), expected);
}
