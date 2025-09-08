use logoscope::masking;
use logoscope::parser;

#[test]
fn test_requirements_validation() {
    println!("=== VALIDATING ORIGINAL REQUIREMENTS ===");
    
    // Test the specific examples from the requirements
    let requirement_cases = vec![
        (
            "2025-08-07T06:41:18.123456Z", 
            "Should detect the full timestamp including microseconds and Z timezone"
        ),
        (
            "2024-12-09 14:30:45.999-0800", 
            "Should capture milliseconds and timezone offset -0800"
        ),
        (
            "Server started at 2025-08-07T06:41:18.123456Z with config loaded", 
            "Should detect embedded timestamp with microseconds"
        ),
        (
            "Request from 192.168.1.1 at 2024-12-09 14:30:45.999-0800 completed successfully",
            "Should detect timestamp with space separator and timezone"
        ),
    ];
    
    println!("## Testing Masking (Template Generation)");
    for (input, description) in requirement_cases.iter() {
        let masked = masking::mask_text(input);
        println!("Input:  {}", input);
        println!("Output: {}", masked);
        println!("Test:   {}", description);
        
        assert!(masked.contains("<TIMESTAMP>"), 
               "Failed to mask timestamp in: {}", input);
        println!("✅ PASSED\n");
    }
    
    println!("## Testing Parser (DateTime Extraction)");
    for (input, description) in requirement_cases.iter() {
        let detected = parser::detect_timestamp_in_text(input);
        println!("Input: {}", input);
        println!("Test:  {}", description);
        
        assert!(detected.is_some(), 
               "Failed to parse timestamp in: {}", input);
        println!("Parsed: {}", detected.unwrap().to_rfc3339());
        println!("✅ PASSED\n");
    }
    
    // Additional timezone format tests
    println!("## Testing Enhanced Timezone Support");
    let timezone_cases = vec![
        ("2025-01-01T12:00:00Z", "Z timezone"),
        ("2025-01-01T12:00:00+00:00", "+00:00 timezone with colon"),
        ("2025-01-01T12:00:00+0000", "+0000 timezone without colon"),
        ("2025-01-01T12:00:00-05:00", "-05:00 negative timezone with colon"),
        ("2025-01-01T12:00:00-0500", "-0500 negative timezone without colon"),
        ("2025-01-01T12:00:00+12:30", "+12:30 half-hour timezone"),
        ("2025-01-01T12:00:00-0930", "-0930 half-hour timezone without colon"),
    ];
    
    for (input, description) in timezone_cases.iter() {
        let masked = masking::mask_text(input);
        let parsed = parser::detect_timestamp_in_text(input);
        
        println!("Input:  {} ({})", input, description);
        println!("Masked: {}", masked);
        
        assert!(masked.contains("<TIMESTAMP>"), 
               "Failed to mask timezone format: {}", input);
        assert!(parsed.is_some(), 
               "Failed to parse timezone format: {}", input);
        
        println!("Parsed: {}", parsed.unwrap().to_rfc3339());
        println!("✅ PASSED\n");
    }
    
    // Fractional seconds precision tests  
    println!("## Testing Enhanced Precision Support");
    let precision_cases = vec![
        ("2025-01-01T12:00:00.1Z", "1 digit fractional"),
        ("2025-01-01T12:00:00.12Z", "2 digits fractional"),
        ("2025-01-01T12:00:00.123Z", "3 digits (milliseconds)"),
        ("2025-01-01T12:00:00.123456Z", "6 digits (microseconds)"),
        ("2025-01-01T12:00:00.123456789Z", "9 digits (nanoseconds)"),
    ];
    
    for (input, description) in precision_cases.iter() {
        let masked = masking::mask_text(input);
        let parsed = parser::detect_timestamp_in_text(input);
        
        println!("Input:  {} ({})", input, description);
        println!("Masked: {}", masked);
        
        assert!(masked.contains("<TIMESTAMP>"), 
               "Failed to mask precision format: {}", input);
        assert!(parsed.is_some(), 
               "Failed to parse precision format: {}", input);
        
        println!("Parsed: {}", parsed.unwrap().to_rfc3339());
        println!("✅ PASSED\n");
    }
    
    // Test that time-only is correctly NOT matched (by design)
    println!("## Testing Time-Only Exclusion (By Design)");
    let time_only_cases = vec![
        "09:20:26.851+0000",
        "12:00:00Z", 
        "14:30:45.999-0800",
    ];
    
    for input in time_only_cases.iter() {
        let masked = masking::mask_text(input);
        println!("Input:  {} (time-only, should NOT be detected as timestamp)", input);
        println!("Masked: {}", masked);
        
        assert!(!masked.contains("<TIMESTAMP>"), 
               "Incorrectly detected time-only as timestamp: {}", input);
        println!("✅ PASSED (correctly excluded)\n");
    }
    
    println!("=== ALL REQUIREMENTS VALIDATED SUCCESSFULLY ===");
}

#[test]
fn test_integration_with_real_world_logs() {
    println!("=== TESTING REAL-WORLD LOG INTEGRATION ===");
    
    let real_world_logs = vec![
        // Nginx access log
        r#"192.168.1.100 - - [2025-01-01T12:00:00.123456Z] "GET /api/users HTTP/1.1" 200 1234"#,
        
        // Application log with timestamp
        r#"2024-12-09 14:30:45.999-0800 ERROR [main] com.example.Service: Database connection failed"#,
        
        // ELB log format  
        r#"2025-08-07T06:41:18.123456Z app/my-loadbalancer/50dc6c495c0c9188 192.168.131.39:2817 10.0.0.1:80 0.000023 0.000086 0.000077 200 200 218 932 "GET https://www.example.com:443/ HTTP/1.1" "Mozilla/5.0""#,
        
        // JSON log with embedded timestamp
        r#"{"timestamp":"2025-01-01T12:00:00.123+01:00","level":"INFO","message":"Request processed","duration":"150ms"}"#,
        
        // Mixed format log
        r#"Server started at 2025-08-07T06:41:18.123456Z, listening on port 8080, processing request_id=abc123"#,
    ];
    
    for (i, log_line) in real_world_logs.iter().enumerate() {
        println!("## Real-world Log Example {}", i + 1);
        println!("Input: {}", log_line);
        
        let masked = masking::mask_text(log_line);
        println!("Masked: {}", masked);
        
        // Should contain at least one timestamp placeholder
        assert!(masked.contains("<TIMESTAMP>"), 
               "Failed to detect timestamp in real-world log: {}", log_line);
        
        // Test that parser can extract the timestamp
        let parsed = parser::detect_timestamp_in_text(log_line);
        if let Some(dt) = parsed {
            println!("Parsed timestamp: {}", dt.to_rfc3339());
        }
        
        println!("✅ PASSED\n");
    }
    
    println!("=== REAL-WORLD INTEGRATION TESTS PASSED ===");
}