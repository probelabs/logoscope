#[test]
fn masking_plaintext_basic_rules() {
    let input = "User 123 logged in from 192.168.1.1 at 2024-01-01T12:00:00Z contact john.doe@example.com";
    let masked = logoscope::masking::mask_text(input);
    assert_eq!(masked, "User <NUM> logged in from <IP> at <TIMESTAMP> contact <EMAIL>");
}

#[test]
fn masking_json_synthetic_message() {
    let line = r#"{"level":"info","time":"2024-02-03T04:05:06Z","user":{"id":456,"email":"a@b.co"},"ip":"2001:0db8:85a3:0000:0000:8a2e:0370:7334"}"#;
    let rec = logoscope::parser::parse_line(line, 1);
    let syn = rec.synthetic_message.expect("synthetic message");
    let masked = logoscope::masking::mask_text(&syn);
    // keys sorted; values masked
    assert_eq!(masked, "ip=<IP> level=info time=<TIMESTAMP> user.email=<EMAIL> user.id=<NUM>");
}

#[test]
fn masking_extended_rules_uuid_path_url_hex_b64() {
    let input = "uuid=550e8400-e29b-41d4-a716-446655440000 path=/var/log/app/error.log url=https://example.com/a?b=1 hex=deadbeefcafebabe b64=eyJmb28iOiJiYXIifQ==";
    let masked = logoscope::masking::mask_text(input);
    assert!(masked.contains("uuid=<UUID>"));
    assert!(masked.contains("path=<PATH>"));
    assert!(masked.contains("url=<URL>"));
    assert!(masked.contains("hex=<HEX>"));
    assert!(masked.contains("b64=<B64>"));
}
