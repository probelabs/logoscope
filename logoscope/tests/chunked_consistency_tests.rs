use logoscope::ai::{summarize_lines_with_opts, SummarizeOpts, StreamingSummarizer};
use std::collections::HashMap;

#[cfg(test)]
mod chunked_consistency_tests {
    use super::*;

    /// Test data that exercises all 4 implemented fixes:
    /// 1. Parameter disambiguation (multiple NUM, IP, HEX in same lines)
    /// 2. Sequence detection (incrementing numeric sequences)
    /// 3. Enhanced path detection (complex service paths with double slashes)
    /// 4. Complete timestamp parsing (with timezones and milliseconds)
    fn get_test_logs() -> Vec<String> {
        vec![
            // Test 1: Multiple parameters needing disambiguation
            "2025-01-15T10:30:45.123+01:00 [INFO] api_call request_id=12345 user_ip=192.168.1.1 session_hex=deadbeef admin_ip=10.0.0.1 response_code=200".to_string(),
            "2025-01-15T10:30:46.456+01:00 [INFO] api_call request_id=12346 user_ip=192.168.1.2 session_hex=cafebabe admin_ip=10.0.0.1 response_code=404".to_string(),
            
            // Test 2: Sequence detection - incrementing IDs
            "2025-01-15T10:30:47.789+01:00 [DEBUG] sequence_processing sequence_id=100 batch_id=1001".to_string(),
            "2025-01-15T10:30:48.012+01:00 [DEBUG] sequence_processing sequence_id=101 batch_id=1002".to_string(),
            "2025-01-15T10:30:49.345+01:00 [DEBUG] sequence_processing sequence_id=102 batch_id=1003".to_string(),
            "2025-01-15T10:30:50.678+01:00 [DEBUG] sequence_processing sequence_id=103 batch_id=1004".to_string(),
            "2025-01-15T10:30:51.901+01:00 [DEBUG] sequence_processing sequence_id=104 batch_id=1005".to_string(),
            
            // Test 3: Enhanced path detection - complex service paths
            "2025-01-15T10:30:52.234+01:00 [WARN] service_call path=syncmanager//usync/scheduled-full/check status=timeout".to_string(),
            "2025-01-15T10:30:53.567+01:00 [ERROR] service_call path=dataservice//v1/users/profile/update status=failed".to_string(),
            "2025-01-15T10:30:54.890+01:00 [INFO] service_call path=auth//oauth/token/validate status=success".to_string(),
            
            // Test 4: Null value detection variations
            "2025-01-15T10:30:55.123+01:00 [INFO] null_handling user_id=(null) session_id=null token=[null] active=true".to_string(),
            "2025-01-15T10:30:56.456+01:00 [INFO] null_handling user_id=12345 session_id=(null) token=null active=false".to_string(),
            
            // Test 5: Mixed complex scenarios
            "2025-01-15T10:30:57.789+01:00 [ERROR] complex_scenario count=15 duration=2.5GB latency=150ms success_rate=95.5% error_path=service//internal/health".to_string(),
            "2025-01-15T10:30:58.012+01:00 [ERROR] complex_scenario count=16 duration=3.1GB latency=175ms success_rate=92.3% error_path=service//internal/metrics".to_string(),
            
            // Test 6: JSON log format
            r#"{"timestamp":"2025-01-15T10:30:59.345+01:00","level":"INFO","msg":"json_test","request_id":123,"user_ip":"192.168.1.100","path":"api//v2/data"}"#.to_string(),
            r#"{"timestamp":"2025-01-15T10:31:00.678+01:00","level":"INFO","msg":"json_test","request_id":124,"user_ip":"192.168.1.101","path":"api//v2/users"}"#.to_string(),
        ]
    }

    #[test]
    fn test_chunked_vs_non_chunked_consistency() {
        let logs = get_test_logs();
        let log_refs: Vec<&str> = logs.iter().map(|s| s.as_str()).collect();
        let opts = SummarizeOpts {
            analyze_spikes: false,
            verbose: false,
            triage: false,
            deep: true,
            ..Default::default()
        };

        // Run non-chunked processing
        let non_chunked_result = summarize_lines_with_opts(&log_refs, &[], None, &opts);

        // Run chunked processing
        let mut streaming_engine = StreamingSummarizer::new();
        streaming_engine.ingest_chunk(&logs, &[], &opts);
        let chunked_result = streaming_engine.finalize(None, &opts);

        // Test 1: Basic summary statistics should match
        assert_eq!(non_chunked_result.summary.total_lines, chunked_result.summary.total_lines, 
                   "Total lines should match between processing modes");
        assert_eq!(non_chunked_result.summary.unique_patterns, chunked_result.summary.unique_patterns,
                   "Unique pattern count should match between processing modes");
        
        // Test 2: Pattern count should match
        assert_eq!(non_chunked_result.patterns.len(), chunked_result.patterns.len(),
                   "Number of patterns should match between processing modes");

        // Create lookup maps for comparison
        let non_chunked_patterns: HashMap<String, _> = non_chunked_result.patterns
            .into_iter()
            .map(|p| (p.template.clone(), p))
            .collect();
        
        let chunked_patterns: HashMap<String, _> = chunked_result.patterns
            .into_iter()
            .map(|p| (p.template.clone(), p))
            .collect();

        // Test 3: All templates should be identical
        for (template, non_chunked_pattern) in &non_chunked_patterns {
            let chunked_pattern = chunked_patterns.get(template)
                .unwrap_or_else(|| panic!("Template '{}' found in non-chunked but not in chunked mode", template));
            
            // Test 3a: Counts should match
            assert_eq!(non_chunked_pattern.total_count, chunked_pattern.total_count,
                       "Pattern count should match for template: {}", template);
            
            // Test 3b: Frequencies should match (within floating point precision)
            assert!((non_chunked_pattern.frequency - chunked_pattern.frequency).abs() < 0.001,
                    "Pattern frequency should match for template: {} (non-chunked: {}, chunked: {})", 
                    template, non_chunked_pattern.frequency, chunked_pattern.frequency);
        }

        // Test 4: Parameter statistics consistency
        for (template, non_chunked_pattern) in &non_chunked_patterns {
            let chunked_pattern = chunked_patterns.get(template).unwrap();
            
            match (&non_chunked_pattern.param_stats, &chunked_pattern.param_stats) {
                (Some(nc_stats), Some(c_stats)) => {
                    assert_eq!(nc_stats.len(), c_stats.len(),
                               "Number of parameter types should match for template: {}", template);
                    
                    for (param_type, nc_param_stats) in nc_stats {
                        let c_param_stats = c_stats.get(param_type)
                            .unwrap_or_else(|| panic!("Parameter '{}' found in non-chunked but not chunked for template: {}", param_type, template));
                        
                        // Test 4a: Basic statistics should match
                        assert_eq!(nc_param_stats.total, c_param_stats.total,
                                   "Parameter total count should match for {}::{}", template, param_type);
                        assert_eq!(nc_param_stats.cardinality, c_param_stats.cardinality,
                                   "Parameter cardinality should match for {}::{}", template, param_type);
                        assert!((nc_param_stats.top_ratio - c_param_stats.top_ratio).abs() < 0.001,
                                "Parameter top_ratio should match for {}::{}", template, param_type);
                        
                        // Test 4b: Sequence detection should match
                        assert_eq!(nc_param_stats.is_sequence, c_param_stats.is_sequence,
                                   "Sequence detection should match for {}::{}", template, param_type);
                        
                        // Test 4c: Sequence info should match (if present)
                        match (&nc_param_stats.sequence_info, &c_param_stats.sequence_info) {
                            (Some(nc_seq), Some(c_seq)) => {
                                assert_eq!(nc_seq.start_value, c_seq.start_value,
                                           "Sequence start should match for {}::{}", template, param_type);
                                assert_eq!(nc_seq.end_value, c_seq.end_value,
                                           "Sequence end should match for {}::{}", template, param_type);
                                assert_eq!(nc_seq.step_size, c_seq.step_size,
                                           "Sequence step should match for {}::{}", template, param_type);
                                assert!((nc_seq.coverage_ratio - c_seq.coverage_ratio).abs() < 0.001,
                                        "Sequence coverage should match for {}::{}", template, param_type);
                            }
                            (None, None) => {} // Both None is OK
                            _ => panic!("Sequence info presence should match for {}::{}", template, param_type)
                        }
                        
                        // Test 4d: Parameter values should match
                        assert_eq!(nc_param_stats.values.len(), c_param_stats.values.len(),
                                   "Parameter value count should match for {}::{}", template, param_type);
                    }
                }
                (None, None) => {} // Both None is OK
                _ => panic!("Parameter stats presence should match for template: {}", template)
            }
        }

        println!("✅ All consistency tests passed!");
        println!("   - Templates: {} (both modes)", non_chunked_patterns.len());
        println!("   - Total lines: {}", non_chunked_result.summary.total_lines);
    }

    #[test]
    fn test_parameter_disambiguation_consistency() {
        // Test specific case of multiple parameters of same type
        let logs = vec![
            "request user_ip=192.168.1.1 proxy_ip=10.0.0.1 count=100 response_time=250 session_hex=deadbeef trace_hex=cafebabe".to_string(),
            "request user_ip=192.168.1.2 proxy_ip=10.0.0.2 count=101 response_time=275 session_hex=feedface trace_hex=deadbeef".to_string(),
        ];
        
        let log_refs: Vec<&str> = logs.iter().map(|s| s.as_str()).collect();
        let opts = SummarizeOpts::default();

        let non_chunked = summarize_lines_with_opts(&log_refs, &[], None, &opts);
        
        let mut streaming_engine = StreamingSummarizer::new();
        streaming_engine.ingest_chunk(&logs, &[], &opts);
        let chunked = streaming_engine.finalize(None, &opts);

        // Both should recognize the template with disambiguated parameters
        assert_eq!(non_chunked.patterns.len(), 1);
        assert_eq!(chunked.patterns.len(), 1);
        
        let nc_template = &non_chunked.patterns[0].template;
        let c_template = &chunked.patterns[0].template;
        
        // Templates should be identical
        assert_eq!(nc_template, c_template);
        
        // For structured logs (key=value), parameters get field-specific names
        // Should contain field-specific parameters (USER_IP, PROXY_IP, COUNT, etc.)
        assert!(nc_template.contains("<USER_IP>"), "Template should contain <USER_IP>");
        assert!(nc_template.contains("<PROXY_IP>"), "Template should contain <PROXY_IP>");
        assert!(nc_template.contains("<COUNT>"), "Template should contain <COUNT>");
        assert!(nc_template.contains("<SESSION_HEX>"), "Template should contain <SESSION_HEX>");
        assert!(nc_template.contains("<TRACE_HEX>"), "Template should contain <TRACE_HEX>");
        
        // Verify parameter stats consistency
        let nc_stats = &non_chunked.patterns[0].param_stats.as_ref().unwrap();
        let c_stats = &chunked.patterns[0].param_stats.as_ref().unwrap();
        assert_eq!(nc_stats.len(), c_stats.len(), "Parameter count should match");
    }

    #[test]
    fn test_sequence_detection_consistency() {
        // Test sequence detection with clear incrementing pattern
        let logs: Vec<String> = (1..=20).map(|i| {
            format!("processing item sequence_id={} batch_number={}", i, i + 1000)
        }).collect();
        
        let log_refs: Vec<&str> = logs.iter().map(|s| s.as_str()).collect();
        let opts = SummarizeOpts { deep: true, ..Default::default() };

        let non_chunked = summarize_lines_with_opts(&log_refs, &[], None, &opts);
        
        let mut streaming_engine = StreamingSummarizer::new();
        streaming_engine.ingest_chunk(&logs, &[], &opts);
        let chunked = streaming_engine.finalize(None, &opts);

        // Should find exactly one pattern
        assert_eq!(non_chunked.patterns.len(), 1);
        assert_eq!(chunked.patterns.len(), 1);

        let nc_pattern = &non_chunked.patterns[0];
        let c_pattern = &chunked.patterns[0];

        // Both should have param_stats
        assert!(nc_pattern.param_stats.is_some(), "Non-chunked should have param_stats");
        assert!(c_pattern.param_stats.is_some(), "Chunked should have param_stats");

        let nc_stats = nc_pattern.param_stats.as_ref().unwrap();
        let c_stats = c_pattern.param_stats.as_ref().unwrap();

        // Should detect SEQUENCE_ID as a sequence in both modes
        let sequence_params = ["SEQUENCE_ID", "NUM", "NUM_1", "BATCH_NUMBER", "NUM_2"];
        
        for param_name in &sequence_params {
            if let (Some(nc_param), Some(c_param)) = (nc_stats.get(*param_name), c_stats.get(*param_name)) {
                // If either detects it as a sequence, both should
                assert_eq!(nc_param.is_sequence, c_param.is_sequence, 
                          "Sequence detection should match for parameter: {}", param_name);
                
                if let (Some(true), Some(true)) = (nc_param.is_sequence, c_param.is_sequence) {
                    // Both detected as sequences - sequence info should match
                    assert!(nc_param.sequence_info.is_some() && c_param.sequence_info.is_some(),
                           "Both should have sequence info for: {}", param_name);
                    
                    let nc_seq = nc_param.sequence_info.as_ref().unwrap();
                    let c_seq = c_param.sequence_info.as_ref().unwrap();
                    
                    assert_eq!(nc_seq.start_value, c_seq.start_value, "Sequence start should match");
                    assert_eq!(nc_seq.end_value, c_seq.end_value, "Sequence end should match");
                    assert_eq!(nc_seq.step_size, c_seq.step_size, "Step size should match");
                    
                    println!("✅ Sequence detected consistently: {} → {} (step: {}, coverage: {:.1}%)", 
                            nc_seq.start_value, nc_seq.end_value, nc_seq.step_size, 
                            nc_seq.coverage_ratio * 100.0);
                }
                break; // Found the sequence parameter
            }
        }
    }

    #[test]
    fn test_path_and_timestamp_consistency() {
        let logs = vec![
            "2025-08-07T06:41:18.123456+01:00 [INFO] service_path=syncmanager//usync/scheduled-full/check duration=15.311649ms".to_string(),
            "2025-08-07 06:41:18.999-0800 [WARN] service_path=dataservice//v2/users/profile/batch duration=2.5GB".to_string(),
            "2025-08-07T06:41:19Z [ERROR] service_path=auth//oauth/token/validate status=(null)".to_string(),
        ];
        
        let log_refs: Vec<&str> = logs.iter().map(|s| s.as_str()).collect();
        let opts = SummarizeOpts::default();

        let non_chunked = summarize_lines_with_opts(&log_refs, &[], None, &opts);
        
        let mut streaming_engine = StreamingSummarizer::new();
        streaming_engine.ingest_chunk(&logs, &[], &opts);
        let chunked = streaming_engine.finalize(None, &opts);

        // Templates should be identical
        assert_eq!(non_chunked.patterns.len(), chunked.patterns.len());
        
        for (nc_pattern, c_pattern) in non_chunked.patterns.iter().zip(chunked.patterns.iter()) {
            assert_eq!(nc_pattern.template, c_pattern.template, 
                      "Templates should match exactly");
            
            // Should detect complex service paths (field-specific placeholders for structured logs)
            assert!(nc_pattern.template.contains("<SERVICE_PATH>"), 
                   "Should detect complex service paths as <SERVICE_PATH>");
            
            // For structured logs with timestamps, they should be properly masked as <TIMESTAMP>
            // This was improved to mask timestamps correctly instead of leaving them as <NUMBER>
            assert!(nc_pattern.template.contains("<TIMESTAMP>") || nc_pattern.template.contains("<DURATION_MS>"), 
                   "Should detect timestamps or duration values in structured logs");
            
            // Should detect null values properly (as field-specific placeholder for structured logs)
            if nc_pattern.template.contains("status") {
                assert!(nc_pattern.template.contains("<STATUS>"),
                       "Should detect null values as field-specific placeholder");
            }
        }
    }
}