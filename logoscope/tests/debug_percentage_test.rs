#[test]
fn debug_percentage_parsing() {
    let input = "cache hit ratio 85%";
    let result = logoscope::param_extractor::mask_and_extract_with_disambiguation(input);
    
    println!("Input: {}", input);
    println!("Template: {}", result.masked_text);
    println!("Parameters: {:?}", result.extracted_params);
    
    assert!(result.extracted_params.contains_key("NUM_%"));
}