#[test]
fn test_end_to_end_disambiguation() {
    let input_lines = vec![
        "2024-01-15T10:30:00Z Processing 42 items with priority 8 and timeout 300",
        "2024-01-15T10:30:01Z Processing 100 items with priority 5 and timeout 200", 
        "2024-01-15T10:30:02Z Processing 1 items with priority 10 and timeout 500",
    ];
    
    let refs: Vec<&str> = input_lines.iter().map(|s| s.as_ref()).collect();
    let opts = logoscope::ai::SummarizeOpts::default();
    let result = logoscope::ai::summarize_lines_with_opts(&refs, &[], None, &opts);
    
    println!("Template: {}", result.patterns[0].template);
    println!("Total lines: {}", result.summary.total_lines);
    println!("Unique patterns: {}", result.summary.unique_patterns);
    
    // Verify we processed 3 lines, not 1
    assert_eq!(result.summary.total_lines, 3);
    assert_eq!(result.summary.unique_patterns, 1);
    
    // The template should show some form of disambiguation
    // Even if different regex patterns are used, we should see different placeholders for different positions
    let template = &result.patterns[0].template;
    println!("Analyzing template: {}", template);
    
    // Count different types of numeric placeholders
    let num_count = template.matches("<NUM>").count();
    let number_count = template.matches("<NUMBER>").count();
    
    println!("NUM placeholders: {}", num_count);
    println!("NUMBER placeholders: {}", number_count);
    
    // We should have at least 3 numeric placeholders total (excluding TIMESTAMP)
    assert!(num_count + number_count >= 3, 
            "Expected at least 3 numeric placeholders, got {} NUM + {} NUMBER = {}", 
            num_count, number_count, num_count + number_count);
    
    // Test that parameters are actually being extracted separately
    let param_stats = &result.patterns[0].param_stats;
    if let Some(stats) = param_stats {
        println!("Parameter stats: {:?}", stats);
        
        // We should have statistics for the different parameter types
        let has_num = stats.contains_key("NUM");
        let has_number = stats.contains_key("NUMBER");
        
        if has_num {
            let num_stats = &stats["NUM"];
            println!("NUM stats - cardinality: {}, values: {:?}", num_stats.cardinality, num_stats.values);
            // The first position (items count) should have 3 distinct values: 42, 100, 1
            assert_eq!(num_stats.cardinality, 3);
            assert_eq!(num_stats.total, 3);
        }
        
        if has_number {
            let number_stats = &stats["NUMBER"];
            println!("NUMBER stats - cardinality: {}, values: {:?}", number_stats.cardinality, number_stats.values);
        }
        
        assert!(has_num || has_number, "Should have at least one parameter type with statistics");
    } else {
        panic!("No parameter statistics found");
    }
}