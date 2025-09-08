#[test]
fn test_direct_disambiguation_function() {
    // Test the disambiguation function directly to see if it works
    
    // Create input with identical numbers that should trigger the same regex
    let test_cases = vec![
        "Processing 42 items and 42 more items for total 42 results",
        "IP 192.168.1.1 connected to 192.168.1.1 forwarding to 192.168.1.1",
        "Request 1234567890abcdef1234 handled by 1234567890abcdef1234 forwarding to 1234567890abcdef1234"
    ];
    
    for (i, input) in test_cases.iter().enumerate() {
        println!("\n=== Test case {}: {} ===", i+1, input);
        
        let result = logoscope::param_extractor::mask_and_extract_with_disambiguation(input);
        
        println!("Masked text: {}", result.masked_text);
        println!("Extracted params: {:?}", result.extracted_params);
        
        // Count placeholders in the masked text
        let num_count = result.masked_text.matches("<NUM>").count();
        let num_2_count = result.masked_text.matches("<NUM_2>").count();
        let num_3_count = result.masked_text.matches("<NUM_3>").count();
        
        let ip_count = result.masked_text.matches("<IP>").count();
        let ip_2_count = result.masked_text.matches("<IP_2>").count();
        let ip_3_count = result.masked_text.matches("<IP_3>").count();
        
        let hex_count = result.masked_text.matches("<HEX>").count();
        let hex_2_count = result.masked_text.matches("<HEX_2>").count();
        let hex_3_count = result.masked_text.matches("<HEX_3>").count();
        
        println!("NUM placeholders: {} + {} + {} = {}", 
                 num_count, num_2_count, num_3_count, num_count + num_2_count + num_3_count);
        println!("IP placeholders: {} + {} + {} = {}", 
                 ip_count, ip_2_count, ip_3_count, ip_count + ip_2_count + ip_3_count);
        println!("HEX placeholders: {} + {} + {} = {}", 
                 hex_count, hex_2_count, hex_3_count, hex_count + hex_2_count + hex_3_count);
        
        // For the numbers test case, we should see positional disambiguation
        if i == 0 {
            // Should have disambiguation for identical numbers
            assert!(num_2_count > 0 || num_3_count > 0, 
                    "Expected positional disambiguation for identical numbers, got: {}", 
                    result.masked_text);
        }
        
        // For IP test case
        if i == 1 {
            // Should have disambiguation for identical IPs
            assert!(ip_2_count > 0 || ip_3_count > 0,
                    "Expected positional disambiguation for identical IPs, got: {}",
                    result.masked_text);
        }
        
        // For HEX test case
        if i == 2 {
            // Should have disambiguation for identical hex values
            assert!(hex_2_count > 0 || hex_3_count > 0,
                    "Expected positional disambiguation for identical hex values, got: {}",
                    result.masked_text);
        }
    }
}