use regex::Regex;
use logoscope::masking;

#[test] 
fn debug_current_timestamp_issues() {
    // Test cases from the requirements
    let test_cases = vec![
        ("09:20:26.851+0000", "Time with milliseconds and timezone +0000"),
        ("2025-08-07T06:41:18.123456Z", "ISO8601 with microseconds and Z timezone"),
        ("2024-12-09 14:30:45.999-0800", "Date time with milliseconds and timezone -0800"),
        ("2025-01-01T12:00:00+00:00", "ISO8601 with +00:00 timezone"),
        ("2025-01-01T12:00:00+0000", "ISO8601 with +0000 timezone (no colon)"),
        ("2025-01-01T12:00:00-05:00", "ISO8601 with negative timezone -05:00"),
        ("2025-01-01T12:00:00-0500", "ISO8601 with negative timezone -0500 (no colon)"),
    ];
    
    // Current regex from param_extractor/masking modules
    let current_regex = Regex::new(r"\b\d{4}-\d{2}-\d{2}[ T]\d{2}:\d{2}:\d{2}(?:\.\d+)?(?:Z|[+-]\d{2}:?\d{2})?\b").unwrap();
    
    println!("=== DEBUGGING CURRENT TIMESTAMP REGEX ===");
    println!("Pattern: {}", current_regex.as_str());
    println!();
    
    for (input, description) in test_cases.iter() {
        println!("Testing: {} ({})", input, description);
        
        let matches: Vec<_> = current_regex.find_iter(input).collect();
        if matches.is_empty() {
            println!("  ❌ NO MATCH");
        } else {
            for (i, m) in matches.iter().enumerate() {
                println!("  ✅ Match {}: '{}'", i + 1, m.as_str());
            }
        }
        
        // Also test with masking function
        let masked = masking::mask_text(input);
        println!("  Masked: '{}'", masked);
        println!();
    }
    
    // Test parser regex as well
    println!("=== PARSER DETECTION ===");
    for (input, description) in test_cases.iter() {
        let detected = logoscope::parser::detect_timestamp_in_text(input);
        println!("Testing: {} ({})", input, description);
        if detected.is_some() {
            println!("  ✅ Parser detected: {}", detected.unwrap().to_rfc3339());
        } else {
            println!("  ❌ Parser did not detect timestamp");
        }
        println!();
    }
}

#[test]
fn test_specific_problematic_cases() {
    let current_regex = Regex::new(r"\b\d{4}-\d{2}-\d{2}[ T]\d{2}:\d{2}:\d{2}(?:\.\d+)?(?:Z|[+-]\d{2}:?\d{2})?\b").unwrap();
    
    // The problematic case from requirements
    let input = "09:20:26.851+0000";
    println!("=== SPECIFIC PROBLEM ANALYSIS ===");
    println!("Input: {}", input);
    println!("Pattern: {}", current_regex.as_str());
    
    let matches: Vec<_> = current_regex.find_iter(input).collect();
    if matches.is_empty() {
        println!("❌ No match - this is the issue!");
        println!("Problem: The regex requires a full date (\\d{{4}}-\\d{{2}}-\\d{{2}}) but input only has time");
        println!("Solution: Need separate pattern for time-only with timezone");
    } else {
        println!("✅ Unexpected match found");
    }
    
    // Test what SHOULD match
    let full_timestamp = "2024-01-01T09:20:26.851+0000";
    let full_matches: Vec<_> = current_regex.find_iter(full_timestamp).collect();
    println!("\nTesting full timestamp: {}", full_timestamp);
    if !full_matches.is_empty() {
        println!("✅ Full timestamp matches: '{}'", full_matches[0].as_str());
    }
}

#[test]
fn test_timezone_format_variations() {
    let current_regex = Regex::new(r"\b\d{4}-\d{2}-\d{2}[ T]\d{2}:\d{2}:\d{2}(?:\.\d+)?(?:Z|[+-]\d{2}:?\d{2})?\b").unwrap();
    
    let timezone_cases = vec![
        ("2025-01-01T12:00:00Z", "Z timezone"),
        ("2025-01-01T12:00:00+00:00", "+00:00 timezone with colon"),
        ("2025-01-01T12:00:00+0000", "+0000 timezone without colon"),  
        ("2025-01-01T12:00:00-05:00", "-05:00 negative timezone with colon"),
        ("2025-01-01T12:00:00-0500", "-0500 negative timezone without colon"),
        ("2025-01-01T12:00:00+12:30", "+12:30 half-hour timezone"),
        ("2025-01-01T12:00:00-0930", "-0930 half-hour timezone without colon"),
    ];
    
    println!("=== TIMEZONE FORMAT TESTING ===");
    println!("Pattern: {}", current_regex.as_str());
    println!("Note: [+-]\\d{{2}}:?\\d{{2}} means timezone digits with optional colon");
    println!();
    
    for (input, description) in timezone_cases.iter() {
        println!("Testing: {} ({})", input, description);
        let matches: Vec<_> = current_regex.find_iter(input).collect();
        if matches.is_empty() {
            println!("  ❌ NO MATCH - Issue with timezone pattern");
        } else {
            println!("  ✅ Match: '{}'", matches[0].as_str());
        }
        println!();
    }
}