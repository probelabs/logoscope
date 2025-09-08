use logoscope::param_extractor;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_canonicalization() {
        let json_log1 = r#"{"api_id": "abc123", "api_name": "Test-A", "level": "debug", "msg": "Init", "mw": "KeyExpired", "org_id": "org456"}"#;
        let json_log2 = r#"{"api_id": "def789", "api_name": "Test-B", "level": "info", "msg": "Init", "mw": "JWTMiddleware", "org_id": "org789"}"#;
        
        let result1 = param_extractor::canonicalize_for_drain(json_log1);
        let result2 = param_extractor::canonicalize_for_drain(json_log2);
        
        // Both should have the same canonicalized structure
        assert_eq!(result1.masked_text, result2.masked_text);
        
        // Should contain expected field placeholders
        assert!(result1.masked_text.contains("api_id = <API_ID>"));
        assert!(result1.masked_text.contains("api_name = <API_NAME>"));
        assert!(result1.masked_text.contains("level = <LEVEL>"));
        assert!(result1.masked_text.contains("msg = <MSG>"));
        assert!(result1.masked_text.contains("mw = <MW>"));
        assert!(result1.masked_text.contains("org_id = <ORG_ID>"));
        
        // Should have extracted the original values
        assert_eq!(result1.extracted_params.get("API_ID").unwrap(), &vec!["abc123"]);
        assert_eq!(result2.extracted_params.get("API_ID").unwrap(), &vec!["def789"]);
    }

    #[test]
    fn test_kv_canonicalization() {
        let kv_log1 = "api_id=abc123 api_name=Test-A level=debug msg=Init mw=KeyExpired org_id=org456";
        let kv_log2 = "api_id=def789 api_name=Test-B level=info msg=Init mw=JWTMiddleware org_id=org789";
        
        let result1 = param_extractor::canonicalize_for_drain(kv_log1);
        let result2 = param_extractor::canonicalize_for_drain(kv_log2);
        
        // Both should have the same canonicalized structure
        assert_eq!(result1.masked_text, result2.masked_text);
        
        // Should contain expected field placeholders
        assert!(result1.masked_text.contains("api_id = <API_ID>"));
        assert!(result1.masked_text.contains("api_name = <API_NAME>"));
        
        // Should have extracted the original values
        assert_eq!(result1.extracted_params.get("API_ID").unwrap(), &vec!["abc123"]);
        assert_eq!(result2.extracted_params.get("API_ID").unwrap(), &vec!["def789"]);
    }

    #[test]
    fn test_unstructured_fallback() {
        let unstructured = "Error connecting to database at 192.168.1.1:5432 with timeout 30s";
        let result = param_extractor::canonicalize_for_drain(unstructured);
        
        // Should fallback to regular masking
        assert!(result.masked_text.contains("<IP>"));
        assert!(result.masked_text.contains("<NUM>s"));
        assert!(result.extracted_params.contains_key("IP"));
        assert!(result.extracted_params.contains_key("NUM_S"));
    }

    #[test]
    fn test_json_flattening() {
        let nested_json = r#"{"level": "info", "service": {"name": "api", "version": "1.0"}, "metrics": {"cpu": 75.5}}"#;
        let result = param_extractor::try_flatten_json(nested_json);
        
        assert!(result.is_some());
        let flattened = result.unwrap();
        
        assert_eq!(flattened.get("level").unwrap(), "info");
        assert_eq!(flattened.get("service.name").unwrap(), "api");
        assert_eq!(flattened.get("service.version").unwrap(), "1.0");
        assert_eq!(flattened.get("metrics.cpu").unwrap(), "75.5");
    }
}