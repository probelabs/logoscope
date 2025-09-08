use regex::Regex;
use logoscope::masking;

#[test]
fn test_enhanced_timestamp_patterns() {
    // Enhanced pattern from the updated modules
    let enhanced_regex = Regex::new(r"\b\d{4}-\d{2}-\d{2}[T ]\d{2}:\d{2}:\d{2}(?:\.\d{1,9})?(?:Z|[+-](?:\d{2}(?::?\d{2})?|\d{4}))\b").unwrap();
    
    let test_cases = vec![
        // Original failing cases
        ("2025-08-07T06:41:18.123456Z", "ISO8601 with microseconds and Z", true),
        ("2024-12-09 14:30:45.999-0800", "Date time with milliseconds and timezone -0800", true),
        
        // Timezone variations
        ("2025-01-01T12:00:00Z", "Basic Z timezone", true),
        ("2025-01-01T12:00:00+00:00", "+00:00 timezone with colon", true),
        ("2025-01-01T12:00:00+0000", "+0000 timezone without colon", true),
        ("2025-01-01T12:00:00-05:00", "-05:00 timezone with colon", true),
        ("2025-01-01T12:00:00-0500", "-0500 timezone without colon", true),
        ("2025-01-01T12:00:00+12:30", "+12:30 half-hour timezone", true),
        ("2025-01-01T12:00:00-0930", "-0930 half-hour timezone", true),
        
        // Fractional second variations
        ("2025-01-01T12:00:00.1Z", "Single digit fractional", true),
        ("2025-01-01T12:00:00.12Z", "Two digit fractional", true),
        ("2025-01-01T12:00:00.123Z", "Millisecond precision", true),
        ("2025-01-01T12:00:00.123456Z", "Microsecond precision", true),
        ("2025-01-01T12:00:00.123456789Z", "Nanosecond precision", true),
        
        // Space vs T separator
        ("2025-01-01 12:00:00Z", "Space separator with Z", true),
        ("2025-01-01 12:00:00.123+01:00", "Space separator with timezone", true),
        
        // Edge cases that should NOT match (time-only without date)
        ("09:20:26.851+0000", "Time-only with timezone", false),
        ("12:00:00Z", "Time-only with Z", false),
        
        // Edge cases for partial timezone
        ("2025-01-01T12:00:00+01", "Timezone with only hours", true),
        ("2025-01-01T12:00:00-05", "Negative timezone with only hours", true),
    ];
    
    println!("=== TESTING ENHANCED TIMESTAMP PATTERN ===");
    println!("Pattern: {}", enhanced_regex.as_str());
    println!();
    
    let mut passed = 0;
    let mut failed = 0;
    
    for (input, description, should_match) in test_cases.iter() {
        let matches: Vec<_> = enhanced_regex.find_iter(input).collect();
        let matched = !matches.is_empty();
        
        println!("Testing: {} ({})", input, description);
        
        if matched == *should_match {
            if matched {
                println!("  ✅ Correctly matched: '{}'", matches[0].as_str());
            } else {
                println!("  ✅ Correctly did not match");
            }
            passed += 1;
        } else {
            if *should_match {
                println!("  ❌ FAILED: Expected match but got none");
            } else {
                println!("  ❌ FAILED: Unexpected match: '{}'", matches[0].as_str());
            }
            failed += 1;
        }
        
        // Also test with masking function
        let masked = masking::mask_text(input);
        let has_timestamp_placeholder = masked.contains("<TIMESTAMP>");
        if has_timestamp_placeholder == *should_match {
            println!("  ✅ Masking correct: '{}'", masked);
        } else {
            println!("  ❌ Masking incorrect: '{}'", masked);
        }
        println!();
    }
    
    println!("=== RESULTS ===");
    println!("Passed: {}", passed);
    println!("Failed: {}", failed);
    println!("Success rate: {:.1}%", (passed as f32 / (passed + failed) as f32) * 100.0);
    
    assert_eq!(failed, 0, "Some timestamp pattern tests failed");
}

#[test]
fn test_timezone_format_comprehensive() {
    let enhanced_regex = Regex::new(r"\b\d{4}-\d{2}-\d{2}[T ]\d{2}:\d{2}:\d{2}(?:\.\d{1,9})?(?:Z|[+-](?:\d{2}(?::?\d{2})?|\d{4}))\b").unwrap();
    
    // Test all timezone variations systematically
    let timezones = vec![
        ("Z", "UTC Z notation"),
        ("+00:00", "UTC with colon"),
        ("+0000", "UTC without colon"),
        ("+01:00", "CET with colon"),
        ("+0100", "CET without colon"),
        ("-05:00", "EST with colon"),
        ("-0500", "EST without colon"),
        ("+12:00", "Max positive timezone"),
        ("-12:00", "Max negative timezone"),
        ("+05:30", "India timezone"),
        ("-03:30", "Newfoundland timezone"),
    ];
    
    println!("=== COMPREHENSIVE TIMEZONE TESTING ===");
    
    for (tz, desc) in timezones.iter() {
        let timestamp = format!("2025-01-01T12:00:00{}", tz);
        let matches: Vec<_> = enhanced_regex.find_iter(&timestamp).collect();
        
        println!("Testing: {} ({})", timestamp, desc);
        if matches.is_empty() {
            println!("  ❌ NO MATCH - timezone format issue");
            panic!("Failed to match timezone format: {}", tz);
        } else {
            println!("  ✅ Match: '{}'", matches[0].as_str());
        }
    }
}

#[test]  
fn test_fractional_seconds_comprehensive() {
    let enhanced_regex = Regex::new(r"\b\d{4}-\d{2}-\d{2}[T ]\d{2}:\d{2}:\d{2}(?:\.\d{1,9})?(?:Z|[+-](?:\d{2}(?::?\d{2})?|\d{4}))\b").unwrap();
    
    let fractional_cases = vec![
        ("", "No fractional seconds"),
        (".1", "1 digit"),
        (".12", "2 digits"),
        (".123", "3 digits (milliseconds)"),
        (".1234", "4 digits"),
        (".12345", "5 digits"),
        (".123456", "6 digits (microseconds)"),
        (".1234567", "7 digits"),
        (".12345678", "8 digits"),
        (".123456789", "9 digits (nanoseconds)"),
    ];
    
    println!("=== COMPREHENSIVE FRACTIONAL SECONDS TESTING ===");
    
    for (frac, desc) in fractional_cases.iter() {
        let timestamp = format!("2025-01-01T12:00:00{}Z", frac);
        let matches: Vec<_> = enhanced_regex.find_iter(&timestamp).collect();
        
        println!("Testing: {} ({})", timestamp, desc);
        if matches.is_empty() {
            println!("  ❌ NO MATCH - fractional seconds issue");
            panic!("Failed to match fractional seconds: {}", frac);
        } else {
            println!("  ✅ Match: '{}'", matches[0].as_str());
        }
    }
}