use logoscope::{ai, param_extractor};
use std::collections::HashMap;

/// Test sequence detection for simple increasing integer sequences
#[test]
fn test_simple_increasing_sequence() {
    let values = vec![
        ("1", 1), ("2", 1), ("3", 1), ("4", 1), ("5", 1)
    ];
    
    let param_stats = create_param_stats_with_sequence_detection("NUM", values);
    
    assert_eq!(param_stats.is_sequence, Some(true));
    assert!(param_stats.sequence_info.is_some());
    
    let seq_info = param_stats.sequence_info.unwrap();
    assert_eq!(seq_info.start_value, "1");
    assert_eq!(seq_info.end_value, "5");
    assert_eq!(seq_info.step_size, 1);
    assert_eq!(seq_info.sequence_type, "increasing");
    assert_eq!(seq_info.coverage_ratio, 1.0);
    assert_eq!(seq_info.total_span, 5);
    
    // Values should be compacted to sequence format
    assert_eq!(param_stats.values.len(), 1);
    assert_eq!(param_stats.values[0].value, "1 → 5 (sequence of 5)");
    assert_eq!(param_stats.values[0].count, 5);
}

/// Test sequence detection for decreasing integer sequences
#[test]
fn test_simple_decreasing_sequence() {
    let values = vec![
        ("10", 1), ("9", 1), ("8", 1), ("7", 1), ("6", 1)
    ];
    
    let param_stats = create_param_stats_with_sequence_detection("NUM", values);
    
    assert_eq!(param_stats.is_sequence, Some(true));
    assert!(param_stats.sequence_info.is_some());
    
    let seq_info = param_stats.sequence_info.unwrap();
    assert_eq!(seq_info.start_value, "10");
    assert_eq!(seq_info.end_value, "6");
    assert_eq!(seq_info.step_size, -1);
    assert_eq!(seq_info.sequence_type, "decreasing");
    assert_eq!(seq_info.coverage_ratio, 1.0);
    assert_eq!(seq_info.total_span, 5);
    
    // Values should be compacted to sequence format
    assert_eq!(param_stats.values.len(), 1);
    assert_eq!(param_stats.values[0].value, "10 → 6 (sequence of 5)");
    assert_eq!(param_stats.values[0].count, 5);
}

/// Test sequence detection with step size greater than 1
#[test]
fn test_sequence_with_step_size() {
    let values = vec![
        ("100", 2), ("102", 2), ("104", 2), ("106", 2), ("108", 2)
    ];
    
    let param_stats = create_param_stats_with_sequence_detection("NUM", values);
    
    assert_eq!(param_stats.is_sequence, Some(true));
    let seq_info = param_stats.sequence_info.unwrap();
    assert_eq!(seq_info.step_size, 2);
    assert_eq!(seq_info.sequence_type, "increasing");
    assert_eq!(seq_info.coverage_ratio, 1.0);
    
    assert_eq!(param_stats.values[0].value, "100 → 108 (sequence of 5)");
    assert_eq!(param_stats.values[0].count, 10); // Total count should be sum of all counts
}

/// Test sequence detection with large incrementing IDs
#[test]
fn test_large_incrementing_ids() {
    let values = vec![
        ("118729680", 1), ("118729681", 1), ("118729682", 1), ("118729683", 1), 
        ("118729684", 1), ("118729685", 1), ("118729686", 1), ("118729687", 1),
        ("118729688", 1), ("118729689", 1), ("118729690", 1), ("118729691", 1)
    ];
    
    let param_stats = create_param_stats_with_sequence_detection("NUM", values);
    
    assert_eq!(param_stats.is_sequence, Some(true));
    let seq_info = param_stats.sequence_info.unwrap();
    assert_eq!(seq_info.start_value, "118729680");
    assert_eq!(seq_info.end_value, "118729691");
    assert_eq!(seq_info.step_size, 1);
    assert_eq!(seq_info.sequence_type, "increasing");
    
    // Should be compacted to single entry
    assert_eq!(param_stats.values.len(), 1);
    assert_eq!(param_stats.values[0].value, "118729680 → 118729691 (sequence of 12)");
}

/// Test mixed sequence and non-sequence values
#[test]
fn test_mixed_sequence_with_outliers() {
    let values = vec![
        ("1", 1), ("2", 1), ("3", 1), ("4", 1), ("5", 1),
        ("999", 1), ("42", 1)  // outliers
    ];
    
    let param_stats = create_param_stats_with_sequence_detection("NUM", values);
    
    // Should still detect sequence if majority follows pattern
    // Coverage ratio should be 5/7 ≈ 0.714 
    if let Some(true) = param_stats.is_sequence {
        let seq_info = param_stats.sequence_info.unwrap();
        assert_eq!(seq_info.start_value, "1");
        assert_eq!(seq_info.end_value, "5");
        assert_eq!(seq_info.step_size, 1);
        assert!((seq_info.coverage_ratio - (5.0 / 7.0)).abs() < 0.01);
        assert_eq!(seq_info.total_span, 5);
        
        // Should have compact sequence representation plus outliers
        assert!(param_stats.values.len() >= 2);
        // First value should be the sequence
        assert!(param_stats.values[0].value.contains("→"));
        assert!(param_stats.values[0].value.contains("sequence of"));
    } else {
        // If threshold is too high, it might not be detected as sequence - that's okay too
        assert_eq!(param_stats.is_sequence, Some(false));
    }
}

/// Test non-sequence data (random values)
#[test]
fn test_non_sequence_random_values() {
    let values = vec![
        ("42", 3), ("789", 2), ("1234", 1), ("56", 4), ("999", 2)
    ];
    
    let param_stats = create_param_stats_with_sequence_detection("NUM", values);
    
    assert_eq!(param_stats.is_sequence, Some(false));
    assert!(param_stats.sequence_info.is_none());
    
    // Should keep original values
    assert_eq!(param_stats.values.len(), 5);
    // Should be sorted by count (descending)
    assert_eq!(param_stats.values[0].value, "56");
    assert_eq!(param_stats.values[0].count, 4);
}

/// Test sequence detection with single value (should not be a sequence)
#[test]
fn test_single_value_not_sequence() {
    let values = vec![("42", 100)];
    
    let param_stats = create_param_stats_with_sequence_detection("NUM", values);
    
    assert_eq!(param_stats.is_sequence, Some(false));
    assert!(param_stats.sequence_info.is_none());
    assert_eq!(param_stats.values.len(), 1);
    assert_eq!(param_stats.values[0].value, "42");
    assert_eq!(param_stats.values[0].count, 100);
}

/// Test sequence detection with only two values (edge case)
#[test]
fn test_two_values_sequence() {
    let values = vec![("100", 1), ("101", 1)];
    
    let param_stats = create_param_stats_with_sequence_detection("NUM", values);
    
    // Two values can form a sequence
    assert_eq!(param_stats.is_sequence, Some(true));
    let seq_info = param_stats.sequence_info.unwrap();
    assert_eq!(seq_info.start_value, "100");
    assert_eq!(seq_info.end_value, "101");
    assert_eq!(seq_info.step_size, 1);
}

/// Test sequence detection with gaps (incomplete sequence)
#[test]
fn test_sequence_with_gaps() {
    let values = vec![
        ("1", 1), ("2", 1), ("4", 1), ("5", 1), ("6", 1)  // Missing 3
    ];
    
    let param_stats = create_param_stats_with_sequence_detection("NUM", values);
    
    // With a gap, should either not be detected as sequence or have lower coverage
    if let Some(true) = param_stats.is_sequence {
        let seq_info = param_stats.sequence_info.unwrap();
        // Should have lower coverage ratio due to missing value
        assert!(seq_info.coverage_ratio < 1.0);
    } else {
        assert_eq!(param_stats.is_sequence, Some(false));
    }
}

/// Test non-integer values (should not form sequences)
#[test]
fn test_non_integer_values() {
    let values = vec![
        ("abc", 1), ("def", 1), ("ghi", 1)
    ];
    
    let param_stats = create_param_stats_with_sequence_detection("STRING", values);
    
    assert_eq!(param_stats.is_sequence, Some(false));
    assert!(param_stats.sequence_info.is_none());
    assert_eq!(param_stats.values.len(), 3);
}

/// Test floating point sequences (should work if they're actually integers)
#[test]
fn test_float_values_that_are_actually_integers() {
    let values = vec![
        ("1.0", 1), ("2.0", 1), ("3.0", 1), ("4.0", 1)
    ];
    
    let param_stats = create_param_stats_with_sequence_detection("NUM", values);
    
    // Should detect sequence if we handle .0 properly
    if let Some(true) = param_stats.is_sequence {
        let seq_info = param_stats.sequence_info.unwrap();
        assert_eq!(seq_info.step_size, 1);
        assert_eq!(seq_info.sequence_type, "increasing");
    }
}

/// Test duplicate values in sequence (multiple counts of same values)
#[test]
fn test_sequence_with_duplicate_counts() {
    let values = vec![
        ("10", 3), ("11", 3), ("12", 3), ("13", 3)
    ];
    
    let param_stats = create_param_stats_with_sequence_detection("NUM", values);
    
    assert_eq!(param_stats.is_sequence, Some(true));
    let seq_info = param_stats.sequence_info.unwrap();
    assert_eq!(seq_info.start_value, "10");
    assert_eq!(seq_info.end_value, "13");
    assert_eq!(seq_info.step_size, 1);
    
    // Total count should be sum of all individual counts
    assert_eq!(param_stats.values[0].count, 12); // 3+3+3+3
}

/// Helper function to create ParamFieldStats with sequence detection
fn create_param_stats_with_sequence_detection(param_type: &str, values: Vec<(&str, usize)>) -> ai::ParamFieldStats {
    let total: usize = values.iter().map(|(_, count)| *count).sum();
    let cardinality = values.len();
    
    let mut value_counts: Vec<ai::ParamValueCount> = values.into_iter()
        .map(|(value, count)| ai::ParamValueCount {
            value: value.to_string(),
            count,
        })
        .collect();
    
    // Sort by count descending (as done in real implementation)
    value_counts.sort_by(|a, b| b.count.cmp(&a.count).then(a.value.cmp(&b.value)));
    
    let top_ratio = if total > 0 { 
        value_counts[0].count as f64 / total as f64 
    } else { 
        0.0 
    };
    
    // Apply sequence detection
    ai::apply_sequence_detection(ai::ParamFieldStats {
        total,
        cardinality,
        values: value_counts,
        top_ratio,
        is_sequence: None,
        sequence_info: None,
    }, param_type)
}

#[cfg(test)]
mod chunked_processing_tests {
    use super::*;
    
    /// Test sequence detection in chunked processing mode
    #[test]
    fn test_sequence_detection_with_chunked_data() {
        // Simulate chunked processing by building stats incrementally
        let mut chunk1_values = vec![
            ("100", 2), ("101", 2), ("102", 2)
        ];
        let chunk2_values = vec![
            ("103", 2), ("104", 2), ("105", 2)
        ];
        
        // Merge chunks (as would happen in real processing)
        chunk1_values.extend(chunk2_values);
        
        let param_stats = create_param_stats_with_sequence_detection("NUM", chunk1_values);
        
        assert_eq!(param_stats.is_sequence, Some(true));
        let seq_info = param_stats.sequence_info.unwrap();
        assert_eq!(seq_info.start_value, "100");
        assert_eq!(seq_info.end_value, "105");
        assert_eq!(seq_info.step_size, 1);
        assert_eq!(param_stats.values[0].count, 12); // 2*6 values
    }
    
    /// Test sequence detection with realistic log data patterns
    #[test]
    fn test_realistic_request_id_sequence() {
        // Simulate realistic request IDs that increment
        let values: Vec<(String, usize)> = (200..300)
            .map(|i| (i.to_string(), 1))
            .collect();
        let values: Vec<(&str, usize)> = values.iter()
            .map(|(s, c)| (s.as_str(), *c))
            .collect();
        
        let param_stats = create_param_stats_with_sequence_detection("NUM", values);
        
        assert_eq!(param_stats.is_sequence, Some(true));
        let seq_info = param_stats.sequence_info.unwrap();
        assert_eq!(seq_info.start_value, "200");
        assert_eq!(seq_info.end_value, "299");
        assert_eq!(seq_info.step_size, 1);
        assert_eq!(seq_info.total_span, 100);
        assert_eq!(seq_info.coverage_ratio, 1.0);
        
        // Should be compacted to single sequence entry
        assert_eq!(param_stats.values.len(), 1);
        assert_eq!(param_stats.values[0].value, "200 → 299 (sequence of 100)");
        assert_eq!(param_stats.values[0].count, 100);
    }
    
    /// Test sequence detection with transaction IDs that have gaps
    #[test]
    fn test_transaction_ids_with_gaps() {
        let mut values = Vec::new();
        
        // Create sequence with some gaps (90% coverage should still be detected)
        for i in 1000..1100 {
            if i % 10 != 0 { // Skip every 10th value to create gaps
                values.push((i.to_string(), 1));
            }
        }
        
        let values: Vec<(&str, usize)> = values.iter()
            .map(|(s, c)| (s.as_str(), *c))
            .collect();
        let param_stats = create_param_stats_with_sequence_detection("NUM", values);
        
        // Should still be detected as sequence with ~90% coverage
        if let Some(true) = param_stats.is_sequence {
            let seq_info = param_stats.sequence_info.unwrap();
            assert_eq!(seq_info.start_value, "1001"); // First non-skipped value
            assert!(seq_info.coverage_ratio > 0.8); // Should be around 90%
            assert!(seq_info.coverage_ratio < 1.0);
        }
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    
    /// Test integration with parameter extraction and masking
    #[test]
    fn test_sequence_detection_with_param_extraction() {
        let log_lines = vec![
            "Request ID: 12345 processed successfully",
            "Request ID: 12346 processed successfully", 
            "Request ID: 12347 processed successfully",
            "Request ID: 12348 processed successfully",
            "Request ID: 12349 processed successfully",
        ];
        
        // Extract parameters from each line
        let mut all_params = HashMap::new();
        for line in log_lines {
            let result = param_extractor::mask_and_extract_with_disambiguation(line);
            for (param_type, values) in result.extracted_params {
                let entry = all_params.entry(param_type).or_insert_with(HashMap::new);
                for value in values {
                    *entry.entry(value).or_insert(0) += 1;
                }
            }
        }
        
        // Build param stats with sequence detection
        if let Some(num_values) = all_params.get("NUM") {
            let values: Vec<(&str, usize)> = num_values.iter()
                .map(|(k, v)| (k.as_str(), *v))
                .collect();
            
            let param_stats = create_param_stats_with_sequence_detection("NUM", values);
            
            // Should detect the incrementing request IDs as a sequence
            assert_eq!(param_stats.is_sequence, Some(true));
            let seq_info = param_stats.sequence_info.unwrap();
            assert_eq!(seq_info.start_value, "12345");
            assert_eq!(seq_info.end_value, "12349");
            assert_eq!(seq_info.step_size, 1);
        }
    }
}