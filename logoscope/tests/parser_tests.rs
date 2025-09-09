use chrono::{Datelike, TimeZone, Timelike, Utc};

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

#[test]
fn parse_rfc3339_with_nanoseconds_and_timezone() {
    // Test exact format: 2024-08-02T12:14:28.653404379-04:00
    let line = "2024-08-02T12:14:28.653404379-04:00 service started";
    let rec = logoscope::parser::parse_line(line, 1);
    assert_eq!(rec.format, logoscope::parser::LogFormat::Plaintext);
    assert!(rec.timestamp.is_some());
    let ts = rec.timestamp.unwrap();
    // Should be converted to UTC (16:14:28 UTC from 12:14:28-04:00)
    assert_eq!(ts.year(), 2024);
    assert_eq!(ts.month(), 8);
    assert_eq!(ts.day(), 2);
    assert_eq!(ts.hour(), 16);
    assert_eq!(ts.minute(), 14);
    assert_eq!(ts.second(), 28);
    assert_eq!(ts.nanosecond(), 653404379);
}

#[test]
fn parse_syslog_format_in_plain_text() {
    use chrono::Datelike;
    // Test exact format: Aug 02 16:14:29
    let line = "Aug 02 16:14:29 hostname service: message";
    let rec = logoscope::parser::parse_line(line, 1);
    assert_eq!(rec.format, logoscope::parser::LogFormat::Plaintext);
    assert!(rec.timestamp.is_some());
    let ts = rec.timestamp.unwrap();
    // Should use current year
    assert_eq!(ts.year(), Utc::now().year());
    assert_eq!(ts.month(), 8);
    assert_eq!(ts.day(), 2);
    assert_eq!(ts.hour(), 16);
    assert_eq!(ts.minute(), 14);
    assert_eq!(ts.second(), 29);
}

#[test]
fn parse_syslog_format_in_json_field() {
    use chrono::Datelike;
    // Test syslog format in JSON time field
    let line = r#"{"time":"Aug 02 16:14:29","level":"debug","msg":"test"}"#;
    let rec = logoscope::parser::parse_line(line, 1);
    assert_eq!(rec.format, logoscope::parser::LogFormat::Json);
    assert!(rec.timestamp.is_some());
    let ts = rec.timestamp.unwrap();
    assert_eq!(ts.year(), Utc::now().year());
    assert_eq!(ts.month(), 8);
    assert_eq!(ts.day(), 2);
    assert_eq!(ts.hour(), 16);
    assert_eq!(ts.minute(), 14);
    assert_eq!(ts.second(), 29);
}

#[test]
fn parse_complex_log_with_both_timestamps() {
    use chrono::Datelike;
    // Test the exact example provided by user
    let line = r#"2024-08-02T12:14:29.284151911-04:00 time="Aug 02 16:14:29" level=debug msg="Couldn't get OAuth client" api_id=cHR4LXBlcmZvcm1hbmNlLWFwaW0vcmVwb3J0cw api_name=reports error="key not found" mw=JWTMiddleware org_id=65d5ca03a582c20007a10d2e origin=10.191.250.217 path=/payments-automation/reports/v1/bacs-report-suns"#;
    let rec = logoscope::parser::parse_line(line, 1);
    assert_eq!(rec.format, logoscope::parser::LogFormat::Plaintext);
    assert!(rec.timestamp.is_some());
    // Should pick up the first timestamp (RFC3339)
    let ts = rec.timestamp.unwrap();
    assert_eq!(ts.year(), 2024);
    assert_eq!(ts.month(), 8);
    assert_eq!(ts.day(), 2);
    assert_eq!(ts.hour(), 16); // Converted to UTC from -04:00
    assert_eq!(ts.minute(), 14);
    assert_eq!(ts.second(), 29);
    assert_eq!(ts.nanosecond(), 284151911);
}

#[test]
fn parse_json_with_rfc3339_nanoseconds() {
    // JSON with RFC3339 timestamp with nanoseconds
    let line = r#"{"timestamp":"2024-08-02T12:14:29.284151911-04:00","level":"info","message":"test"}"#;
    let rec = logoscope::parser::parse_line(line, 1);
    assert_eq!(rec.format, logoscope::parser::LogFormat::Json);
    assert!(rec.timestamp.is_some());
    let ts = rec.timestamp.unwrap();
    assert_eq!(ts.year(), 2024);
    assert_eq!(ts.month(), 8);
    assert_eq!(ts.day(), 2);
    assert_eq!(ts.hour(), 16); // UTC from -04:00
    assert_eq!(ts.minute(), 14);
    assert_eq!(ts.second(), 29);
    assert_eq!(ts.nanosecond(), 284151911);
}
