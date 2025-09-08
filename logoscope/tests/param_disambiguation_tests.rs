#[test]
fn test_multiple_num_parameter_disambiguation() {
    let input = "Processing 42 items with priority 8 and timeout 300";
    let result = logoscope::param_extractor::mask_and_extract_with_disambiguation(input);
    
    // Check that template correctly disambiguates parameters
    assert_eq!(result.masked_text, "Processing <NUM> items with priority <NUM_2> and timeout <NUM_3>");
    
    // Check that parameters are correctly extracted with proper names
    assert!(result.extracted_params.contains_key("NUM"));
    assert!(result.extracted_params.contains_key("NUM_2"));
    assert!(result.extracted_params.contains_key("NUM_3"));
    
    assert_eq!(result.extracted_params["NUM"], vec!["42"]);
    assert_eq!(result.extracted_params["NUM_2"], vec!["8"]);
    assert_eq!(result.extracted_params["NUM_3"], vec!["300"]);
}

#[test]
fn test_multiple_ip_parameter_disambiguation() {
    let input = "Transfer from 192.168.1.1 to 192.168.1.2 via 192.168.1.100";
    let result = logoscope::param_extractor::mask_and_extract_with_disambiguation(input);
    
    assert_eq!(result.masked_text, "Transfer from <IP> to <IP_2> via <IP_3>");
    
    assert_eq!(result.extracted_params["IP"], vec!["192.168.1.1"]);
    assert_eq!(result.extracted_params["IP_2"], vec!["192.168.1.2"]);
    assert_eq!(result.extracted_params["IP_3"], vec!["192.168.1.100"]);
}

#[test]
fn test_mixed_parameter_disambiguation() {
    let input = "Connection from 10.0.0.1:8080 to 10.0.0.2:9090 took 150ms and processed 42 items";
    let result = logoscope::param_extractor::mask_and_extract_with_disambiguation(input);
    
    assert_eq!(result.masked_text, "Connection from <IP>:<NUM> to <IP_2>:<NUM_2> took <NUM>ms and processed <NUM_3> items");
    
    // Check IPs
    assert_eq!(result.extracted_params["IP"], vec!["10.0.0.1"]);
    assert_eq!(result.extracted_params["IP_2"], vec!["10.0.0.2"]);
    
    // Check numbers with units - units are not disambiguated
    assert_eq!(result.extracted_params["NUM_MS"], vec!["150ms"]);
    
    // Check regular numbers (ports and items)
    assert_eq!(result.extracted_params["NUM"], vec!["8080"]);
    assert_eq!(result.extracted_params["NUM_2"], vec!["9090"]);
    assert_eq!(result.extracted_params["NUM_3"], vec!["42"]);
}

#[test]
fn test_single_parameter_unchanged() {
    let input = "Processing 42 items";
    let result = logoscope::param_extractor::mask_and_extract_with_disambiguation(input);
    
    // Single parameter should not be numbered
    assert_eq!(result.masked_text, "Processing <NUM> items");
    assert_eq!(result.extracted_params["NUM"], vec!["42"]);
    assert!(!result.extracted_params.contains_key("NUM_2"));
}

#[test]
fn test_backward_compatibility() {
    let input = "User 192.168.1.99 failed login at 2024-01-01T14:20:01Z";
    let result = logoscope::param_extractor::mask_and_extract_with_disambiguation(input);
    
    // Should work the same as original for single occurrences
    assert_eq!(result.masked_text, "User <IP> failed login at <TIMESTAMP>");
    assert_eq!(result.extracted_params["IP"], vec!["192.168.1.99"]);
    assert_eq!(result.extracted_params["TIMESTAMP"], vec!["2024-01-01T14:20:01Z"]);
}

#[test]
fn test_canonicalization_uses_disambiguation() {
    let input = "Processing 42 items with priority 8 and timeout 300";
    let result = logoscope::param_extractor::canonicalize_for_drain(input);
    
    // canonicalize_for_drain should now use disambiguation
    assert_eq!(result.masked_text, "Processing <NUM> items with priority <NUM_2> and timeout <NUM_3>");
    
    assert_eq!(result.extracted_params["NUM"], vec!["42"]);
    assert_eq!(result.extracted_params["NUM_2"], vec!["8"]);
    assert_eq!(result.extracted_params["NUM_3"], vec!["300"]);
}

#[test]
fn test_base_param_type_helper() {
    use logoscope::analyzers::get_base_param_type;
    
    assert_eq!(get_base_param_type("NUM"), "NUM");
    assert_eq!(get_base_param_type("NUM_2"), "NUM");
    assert_eq!(get_base_param_type("NUM_10"), "NUM");
    assert_eq!(get_base_param_type("IP_3"), "IP");
    assert_eq!(get_base_param_type("HEX_4"), "HEX");
    
    // Non-numbered parameters should remain unchanged
    assert_eq!(get_base_param_type("NUM_MS"), "NUM_MS");
    assert_eq!(get_base_param_type("TIMESTAMP"), "TIMESTAMP");
    assert_eq!(get_base_param_type("USER_ID"), "USER_ID");
}

#[test]
fn test_anomaly_detection_with_numbered_parameters() {
    use std::collections::HashMap;
    use logoscope::ai::{ParamFieldStats, ParamValueCount};
    use logoscope::analyzers::{AnalysisContext, AnalyzerRegistry};
    
    // Create mock param stats for numbered parameters
    let mut param_stats = HashMap::new();
    
    // NUM with some values
    param_stats.insert("NUM".to_string(), ParamFieldStats {
        total: 100,
        cardinality: 5,
        values: vec![
            ParamValueCount { value: "42".to_string(), count: 50 },
            ParamValueCount { value: "100".to_string(), count: 30 },
            ParamValueCount { value: "200".to_string(), count: 15 },
            ParamValueCount { value: "999".to_string(), count: 3 },
            ParamValueCount { value: "1".to_string(), count: 2 },
        ],
        top_ratio: 0.5,
        is_sequence: None,
        sequence_info: None,
    });
    
    // NUM_2 with different distribution
    param_stats.insert("NUM_2".to_string(), ParamFieldStats {
        total: 100,
        cardinality: 3,
        values: vec![
            ParamValueCount { value: "8".to_string(), count: 90 },
            ParamValueCount { value: "9".to_string(), count: 8 },
            ParamValueCount { value: "10".to_string(), count: 2 },
        ],
        top_ratio: 0.9,
        is_sequence: None,
        sequence_info: None,
    });
    
    let context = AnalysisContext {
        template: "Processing <NUM> items with priority <NUM_2> and timeout <NUM_3>".to_string(),
        clean_template: "Processing <NUM> items with priority <NUM_2> and timeout <NUM_3>".to_string(),
        total_count: 100,
        timestamps: Vec::new(),
        line_params: Vec::new(),
        pattern_indices: Vec::new(),
        param_stats: Some(param_stats),
    };
    
    let registry = AnalyzerRegistry::new();
    let opts = logoscope::ai::SummarizeOpts::default();
    let results = registry.analyze(&context, &opts);
    
    // Should detect value concentration in NUM_2 since it has 90% concentration
    let found_concentration = results.parameter_anomalies
        .as_ref()
        .map(|anomalies| anomalies.iter()
            .any(|a| a.param == "NUM_2" && a.anomaly_type == "value_concentration"))
        .unwrap_or(false);
    assert!(found_concentration, "Should detect value concentration in NUM_2");
}

#[test] 
fn test_complex_real_world_example() {
    let input = "2024-01-15T10:30:45.123Z [INFO] API request from 192.168.1.100:8080 to endpoint /api/v1/users/42 returned status 200 in 150ms, processed 25 records, cache hit ratio 85%";
    let result = logoscope::param_extractor::mask_and_extract_with_disambiguation(input);
    
    // Should properly disambiguate all the different numbers
    let expected_template = "<TIMESTAMP> [INFO] API request from <IP>:<NUM> to endpoint <PATH> returned status <NUM_2> in <NUM>ms, processed <NUM_3> records, cache hit ratio <NUM>%";
    assert_eq!(result.masked_text, expected_template);
    
    // Verify parameter extraction
    assert_eq!(result.extracted_params["TIMESTAMP"], vec!["2024-01-15T10:30:45.123Z"]);
    assert_eq!(result.extracted_params["IP"], vec!["192.168.1.100"]);
    assert_eq!(result.extracted_params["PATH"], vec!["/api/v1/users/42"]);
    
    // Numbers should be properly disambiguated
    assert_eq!(result.extracted_params["NUM"], vec!["8080"]);      // port
    assert_eq!(result.extracted_params["NUM_2"], vec!["200"]);     // status 
    assert_eq!(result.extracted_params["NUM_3"], vec!["25"]);      // record count
    
    // Units should be preserved
    assert_eq!(result.extracted_params["NUM_MS"], vec!["150ms"]);  // duration
    assert_eq!(result.extracted_params["NUM_%"], vec!["85%"]);     // percentage
}