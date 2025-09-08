#[test]
fn debug_unit_regex() {
    use regex::Regex;
    
    let re = Regex::new(r"(?i)\b-?\d+(?:\.\d+)?(?:\s*)(ms|us|µs|ns|s|m|h|kb|mb|gb|kib|mib|gib|b|%)\b").unwrap();
    
    let test_cases = vec!["150ms", "85%", "100KB", "2s"];
    
    for case in test_cases {
        println!("Testing: {}", case);
        if let Some(caps) = re.captures(case) {
            println!("  Full match: {}", caps.get(0).unwrap().as_str());
            if let Some(unit) = caps.get(1) {
                println!("  Unit: {}", unit.as_str());
            }
        } else {
            println!("  No match");
        }
    }
    
    // Now test the actual function
    println!("\nActual function results:");
    let result = logoscope::param_extractor::mask_and_extract_with_disambiguation("cache hit ratio 85%");
    println!("Template: {}", result.masked_text);
    println!("Parameters: {:?}", result.extracted_params);
}