#[test]
fn test_ip_masking_and_extraction() {
    let input = "User 192.168.1.99 failed login at 2024-01-01T14:20:01Z";
    let result = logoscope::param_extractor::mask_and_extract(input);
    
    assert_eq!(result.masked_text, "User <IP> failed login at <TIMESTAMP>");
    assert!(result.extracted_params.contains_key("IP"));
    assert!(result.extracted_params.contains_key("TIMESTAMP"));
    assert_eq!(result.extracted_params["IP"], vec!["192.168.1.99"]);
    assert_eq!(result.extracted_params["TIMESTAMP"], vec!["2024-01-01T14:20:01Z"]);
}

#[test]
fn test_overlapping_patterns_priority() {
    // IP addresses should be matched before individual numbers
    let input = "Connection from 10.0.0.1 port 8080";
    let result = logoscope::param_extractor::mask_and_extract(input);
    
    assert_eq!(result.masked_text, "Connection from <IP> port <NUM>");
    assert_eq!(result.extracted_params["IP"], vec!["10.0.0.1"]);
    assert_eq!(result.extracted_params["NUM"], vec!["8080"]);
}

#[test]
fn test_timestamp_priority() {
    // Timestamps should be matched as whole units
    let input = "Event at 2024-01-01T14:20:01Z with code 42";
    let result = logoscope::param_extractor::mask_and_extract(input);
    
    assert_eq!(result.masked_text, "Event at <TIMESTAMP> with code <NUM>");
    assert_eq!(result.extracted_params["TIMESTAMP"], vec!["2024-01-01T14:20:01Z"]);
    assert_eq!(result.extracted_params["NUM"], vec!["42"]);
}

#[test]
fn test_multiple_ips() {
    let input = "Transfer from 192.168.1.1 to 192.168.1.2";
    let result = logoscope::param_extractor::mask_and_extract(input);
    
    assert_eq!(result.masked_text, "Transfer from <IP> to <IP>");
    assert!(result.extracted_params["IP"].contains(&"192.168.1.1".to_string()));
    assert!(result.extracted_params["IP"].contains(&"192.168.1.2".to_string()));
}

#[test]
fn test_number_with_units() {
    let input = "Response time 150ms, size 2048KB";
    let result = logoscope::param_extractor::mask_and_extract(input);
    
    assert_eq!(result.masked_text, "Response time <NUM>ms, size <NUM>KB");
    assert!(result.extracted_params.contains_key("NUM_MS"));
    assert!(result.extracted_params.contains_key("NUM_KB"));
}

#[test]
fn test_kv_param_extraction() {
    use std::collections::BTreeMap;
    
    let mut fields = BTreeMap::new();
    fields.insert("client_ip".to_string(), "192.168.1.99".to_string());
    fields.insert("user_id".to_string(), "admin".to_string());
    fields.insert("status_code".to_string(), "401".to_string());
    
    let params = logoscope::param_extractor::extract_kv_params(&fields);
    
    assert_eq!(params["CLIENT_IP"], vec!["192.168.1.99"]);
    assert_eq!(params["USER_ID"], vec!["admin"]);
    assert_eq!(params["STATUS_CODE"], vec!["401"]);
}

#[test]
fn test_param_merging() {
    use std::collections::HashMap;
    
    let mut masked_params = HashMap::new();
    masked_params.insert("IP".to_string(), vec!["192.168.1.1".to_string()]);
    
    let mut kv_params = HashMap::new();
    kv_params.insert("IP".to_string(), vec!["192.168.1.2".to_string(), "192.168.1.1".to_string()]);
    
    let merged = logoscope::param_extractor::merge_params(masked_params, kv_params);
    
    // Should deduplicate and sort
    assert_eq!(merged["IP"].len(), 2);
    assert!(merged["IP"].contains(&"192.168.1.1".to_string()));
    assert!(merged["IP"].contains(&"192.168.1.2".to_string()));
}