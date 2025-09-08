use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::HashMap;
use std::collections::BTreeMap;

// Re-use the same regexes from masking module for consistency
static RE_TIMESTAMP: Lazy<Regex> = Lazy::new(|| {
    // Enhanced ISO8601/RFC3339 with comprehensive timezone and fractional second support
    // Supports: 2025-08-07T06:41:18Z, 2025-08-07T06:41:18.123456+01:00, 2025-08-07 06:41:18.999-0800
    // Fractional seconds: 1-9 digits (.1 to .123456789)  
    // Timezones: Z, ±HH:MM, ±HHMM, ±HH
    Regex::new(r"\b\d{4}-\d{2}-\d{2}[T ]\d{2}:\d{2}:\d{2}(?:\.\d{1,9})?(?:Z|[+-](?:\d{2}(?::?\d{2})?|\d{4}))\b").unwrap()
});

static RE_NUM_UNIT: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\b-?\d+(?:\.\d+)?(?:\s*)(ms|us|µs|ns|s|m|h|kb|mb|gb|kib|mib|gib|b|%)\b").unwrap()
});

static RE_NUM_PERCENT: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\b-?\d+(?:\.\d+)?%").unwrap()
});

static RE_URL: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"\b[a-zA-Z][a-zA-Z0-9+.-]*://[^\s"']+\b"#).unwrap()
});

static RE_IPV6: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\b(?:[0-9a-fA-F]{1,4}:){7}[0-9a-fA-F]{1,4}\b").unwrap()
});

static RE_IPV4: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\b(?:(?:25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)\.){3}(?:25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)\b").unwrap()
});

static RE_EMAIL: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}\b").unwrap()
});

static RE_UUID: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\b[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}\b").unwrap()
});

static RE_PATH: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?x)
        (?:
            # Standard Unix-style paths (at least 2 components)
            /[\w.\-]+(?:/[\w.\-]+)+
          | \./[\w.\-]+(?:/[\w.\-]+)*
          | \../[\w.\-]+(?:/[\w.\-]+)*
          | ~/[\w.\-]+(?:/[\w.\-]+)*
          # Service/scheme-style paths with double slashes (high priority)
          | \w+//[\w.\-/]+
          # Complex paths with dashes and underscores (at least 3 components)
          | [\w.\-_]+(?:/[\w.\-_]+){2,}
        )
    ").unwrap()
});

static RE_B64: Lazy<Regex> = Lazy::new(|| {
    // More conservative Base64 regex: must end with padding or be followed by whitespace/punctuation
    // This avoids matching path-like structures that contain slashes
    Regex::new(r"\b[A-Za-z0-9+/]{20,}={0,2}\b").unwrap()
});

static RE_HEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\b[0-9a-fA-F]{16,}\b").unwrap()
});

static RE_FLOAT: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\b-?\d+\.\d+\b").unwrap()
});

static RE_INT: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\b-?\d+\b").unwrap()
});

// Regex for null values like (null), [null], null
static RE_NULL: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\b(?:\(|\[)?null(?:\)|\])?\b").unwrap()
});

// Regex for detecting key-value pairs in text (key=value format)
// Safe, linear-time detector that avoids catastrophic backtracking
// Simply checks for pattern: word = non-empty-value (up to comma, space, or end)
static RE_KV_PAIR: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\b\w+\s*=\s*[^,\s=]+").unwrap()
});

// Regex for extracting key-value pairs with capturing groups
// Captures: (key) = (value)
static RE_KV_EXTRACT: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\b(\w+)\s*=\s*([^\s,]+)").unwrap()
});

#[derive(Debug, Clone)]
pub struct MaskingResult {
    pub masked_text: String,
    pub extracted_params: HashMap<String, Vec<String>>,
}

/// Masks text while extracting the original values that were masked
pub fn mask_and_extract(input: &str) -> MaskingResult {
    let mut params: HashMap<String, Vec<String>> = HashMap::new();
    let mut masked = input.to_string();
    
    // Collect all matches with positions, types, and replacements
    let mut all_matches: Vec<(usize, usize, String, String, String)> = Vec::new();
    
    // Timestamps (highest priority)
    for cap in RE_TIMESTAMP.find_iter(input) {
        all_matches.push((cap.start(), cap.end(), cap.as_str().to_string(), 
                         "TIMESTAMP".to_string(), "<TIMESTAMP>".to_string()));
    }
    
    // URLs
    for cap in RE_URL.find_iter(input) {
        all_matches.push((cap.start(), cap.end(), cap.as_str().to_string(), 
                         "URL".to_string(), "<URL>".to_string()));
    }
    
    // IP addresses (before numbers!)
    for cap in RE_IPV6.find_iter(input) {
        all_matches.push((cap.start(), cap.end(), cap.as_str().to_string(), 
                         "IP".to_string(), "<IP>".to_string()));
    }
    
    for cap in RE_IPV4.find_iter(input) {
        all_matches.push((cap.start(), cap.end(), cap.as_str().to_string(), 
                         "IP".to_string(), "<IP>".to_string()));
    }
    
    // Email addresses
    for cap in RE_EMAIL.find_iter(input) {
        all_matches.push((cap.start(), cap.end(), cap.as_str().to_string(), 
                         "EMAIL".to_string(), "<EMAIL>".to_string()));
    }
    
    // UUIDs
    for cap in RE_UUID.find_iter(input) {
        all_matches.push((cap.start(), cap.end(), cap.as_str().to_string(), 
                         "UUID".to_string(), "<UUID>".to_string()));
    }
    
    // Paths (higher priority than Base64)
    for cap in RE_PATH.find_iter(input) {
        all_matches.push((cap.start(), cap.end(), cap.as_str().to_string(), 
                         "PATH".to_string(), "<PATH>".to_string()));
    }
    
    // Null values
    for cap in RE_NULL.find_iter(input) {
        all_matches.push((cap.start(), cap.end(), cap.as_str().to_string(), 
                         "NULL".to_string(), "<NULL>".to_string()));
    }
    
    // Hex strings
    for cap in RE_HEX.find_iter(input) {
        all_matches.push((cap.start(), cap.end(), cap.as_str().to_string(), 
                         "HEX".to_string(), "<HEX>".to_string()));
    }
    
    // Base64
    for cap in RE_B64.find_iter(input) {
        all_matches.push((cap.start(), cap.end(), cap.as_str().to_string(), 
                         "B64".to_string(), "<B64>".to_string()));
    }
    
    // Percentages (handle separately since % doesn't have word boundary)
    for cap in RE_NUM_PERCENT.find_iter(input) {
        all_matches.push((cap.start(), cap.end(), cap.as_str().to_string(),
                         "NUM_%".to_string(), "<NUM>%".to_string()));
    }
    
    // Numbers with units
    for cap in RE_NUM_UNIT.captures_iter(input) {
        let full_match = cap.get(0).unwrap();
        let unit = cap.get(1).map(|m| m.as_str()).unwrap_or("");
        if unit != "%" {  // Skip % here since we handle it separately
            all_matches.push((full_match.start(), full_match.end(), full_match.as_str().to_string(),
                             format!("NUM_{}", unit.to_uppercase()), format!("<NUM>{unit}")));
        }
    }
    
    // Generic floats
    for cap in RE_FLOAT.find_iter(input) {
        all_matches.push((cap.start(), cap.end(), cap.as_str().to_string(), 
                         "NUM".to_string(), "<NUM>".to_string()));
    }
    
    // Generic integers (lowest priority)
    for cap in RE_INT.find_iter(input) {
        all_matches.push((cap.start(), cap.end(), cap.as_str().to_string(), 
                         "NUM".to_string(), "<NUM>".to_string()));
    }
    
    // Sort by start position, then by length (longer matches first for same position)
    all_matches.sort_by(|a, b| {
        a.0.cmp(&b.0).then_with(|| b.1.cmp(&a.1))
    });
    
    // Remove overlapping matches (keep first/longer match)
    let mut filtered_matches: Vec<(usize, usize, String, String, String)> = Vec::new();
    let mut last_end = 0;
    
    for (start, end, value, param_type, replacement) in all_matches {
        if start >= last_end {
            filtered_matches.push((start, end, value.clone(), param_type.clone(), replacement));
            params.entry(param_type).or_default().push(value);
            last_end = end;
        }
    }
    
    // Apply replacements from end to beginning
    for (start, end, _, _, replacement) in filtered_matches.iter().rev() {
        masked.replace_range(*start..*end, replacement);
    }
    
    MaskingResult {
        masked_text: masked,
        extracted_params: params,
    }
}

/// Masks text while extracting parameters with positional disambiguation for repeated types
/// This solves the problem where multiple <NUM> parameters get lumped together
pub fn mask_and_extract_with_disambiguation(input: &str) -> MaskingResult {
    let mut params: HashMap<String, Vec<String>> = HashMap::new();
    let mut masked = input.to_string();
    
    // Collect all matches with positions, types, and replacements
    let mut all_matches: Vec<(usize, usize, String, String, String)> = Vec::new();
    
    // Timestamps (highest priority)
    for cap in RE_TIMESTAMP.find_iter(input) {
        all_matches.push((cap.start(), cap.end(), cap.as_str().to_string(), 
                         "TIMESTAMP".to_string(), "<TIMESTAMP>".to_string()));
    }
    
    // URLs
    for cap in RE_URL.find_iter(input) {
        all_matches.push((cap.start(), cap.end(), cap.as_str().to_string(), 
                         "URL".to_string(), "<URL>".to_string()));
    }
    
    // IP addresses (before numbers!)
    for cap in RE_IPV6.find_iter(input) {
        all_matches.push((cap.start(), cap.end(), cap.as_str().to_string(), 
                         "IP".to_string(), "<IP>".to_string()));
    }
    
    for cap in RE_IPV4.find_iter(input) {
        all_matches.push((cap.start(), cap.end(), cap.as_str().to_string(), 
                         "IP".to_string(), "<IP>".to_string()));
    }
    
    // Email addresses
    for cap in RE_EMAIL.find_iter(input) {
        all_matches.push((cap.start(), cap.end(), cap.as_str().to_string(), 
                         "EMAIL".to_string(), "<EMAIL>".to_string()));
    }
    
    // UUIDs
    for cap in RE_UUID.find_iter(input) {
        all_matches.push((cap.start(), cap.end(), cap.as_str().to_string(), 
                         "UUID".to_string(), "<UUID>".to_string()));
    }
    
    // Paths (higher priority than Base64)
    for cap in RE_PATH.find_iter(input) {
        all_matches.push((cap.start(), cap.end(), cap.as_str().to_string(), 
                         "PATH".to_string(), "<PATH>".to_string()));
    }
    
    // Null values
    for cap in RE_NULL.find_iter(input) {
        all_matches.push((cap.start(), cap.end(), cap.as_str().to_string(), 
                         "NULL".to_string(), "<NULL>".to_string()));
    }
    
    // Hex strings
    for cap in RE_HEX.find_iter(input) {
        all_matches.push((cap.start(), cap.end(), cap.as_str().to_string(), 
                         "HEX".to_string(), "<HEX>".to_string()));
    }
    
    // Base64
    for cap in RE_B64.find_iter(input) {
        all_matches.push((cap.start(), cap.end(), cap.as_str().to_string(), 
                         "B64".to_string(), "<B64>".to_string()));
    }
    
    // Percentages (handle separately since % doesn't have word boundary)
    for cap in RE_NUM_PERCENT.find_iter(input) {
        all_matches.push((cap.start(), cap.end(), cap.as_str().to_string(),
                         "NUM_%".to_string(), "<NUM>%".to_string()));
    }
    
    // Numbers with units
    for cap in RE_NUM_UNIT.captures_iter(input) {
        let full_match = cap.get(0).unwrap();
        let unit = cap.get(1).map(|m| m.as_str()).unwrap_or("");
        if unit != "%" {  // Skip % here since we handle it separately
            all_matches.push((full_match.start(), full_match.end(), full_match.as_str().to_string(),
                             format!("NUM_{}", unit.to_uppercase()), format!("<NUM>{unit}")));
        }
    }
    
    // Generic floats
    for cap in RE_FLOAT.find_iter(input) {
        all_matches.push((cap.start(), cap.end(), cap.as_str().to_string(), 
                         "NUM".to_string(), "<NUM>".to_string()));
    }
    
    // Generic integers (lowest priority)
    for cap in RE_INT.find_iter(input) {
        all_matches.push((cap.start(), cap.end(), cap.as_str().to_string(), 
                         "NUM".to_string(), "<NUM>".to_string()));
    }
    
    // Sort by start position, then by length (longer matches first for same position)
    all_matches.sort_by(|a, b| {
        a.0.cmp(&b.0).then_with(|| b.1.cmp(&a.1))
    });
    
    // Remove overlapping matches (keep first/longer match)
    let mut filtered_matches: Vec<(usize, usize, String, String, String)> = Vec::new();
    let mut last_end = 0;
    
    for (start, end, value, param_type, replacement) in all_matches {
        if start >= last_end {
            filtered_matches.push((start, end, value, param_type, replacement));
            last_end = end;
        }
    }
    
    // Track position counts for disambiguation (process in forward order to get correct numbering)
    let mut position_counts: HashMap<String, usize> = HashMap::new();
    let mut match_replacements = Vec::new();
    
    // First pass: compute position numbers in forward order
    for (start, end, value, param_type, original_replacement) in filtered_matches.iter() {
        // Only disambiguate generic types (NUM, IP, HEX, etc.), not specific unit types (NUM_MS, NUM_%, etc.)
        let should_disambiguate = !param_type.contains("_") || 
                                  (param_type.contains("_") && 
                                   param_type.chars().last().is_some_and(|c| c.is_ascii_digit()));
        
        let (disambiguated_param, template_placeholder) = if should_disambiguate {
            // Increment position counter for this parameter type
            let count = position_counts.entry(param_type.clone()).or_insert(0);
            *count += 1;
            
            if *count == 1 {
                // First occurrence keeps original name
                (param_type.clone(), format!("<{param_type}>"))
            } else {
                // Subsequent occurrences get numbered
                let disambiguated = format!("{param_type}_{count}");
                (disambiguated.clone(), format!("<{disambiguated}>"))
            }
        } else {
            // Don't disambiguate unit types - use original replacement
            (param_type.clone(), original_replacement.clone())
        };
        
        match_replacements.push((*start, *end, value.clone(), disambiguated_param, template_placeholder));
    }
    
    // Second pass: apply replacements from end to beginning to avoid position shifts
    for (start, end, value, disambiguated_param, template_placeholder) in match_replacements.iter().rev() {
        // Apply template replacement
        masked.replace_range(*start..*end, template_placeholder);
        
        // Store parameter value under disambiguated name
        params.entry(disambiguated_param.clone()).or_default().push(value.clone());
    }
    
    MaskingResult {
        masked_text: masked,
        extracted_params: params,
    }
}

/// Extracts parameters from structured key-value pairs
pub fn extract_kv_params(flat_fields: &std::collections::BTreeMap<String, String>) -> HashMap<String, Vec<String>> {
    let mut params = HashMap::new();
    
    // Track ALL fields, using uppercase field name as the parameter type
    // This allows any application-specific fields to be tracked
    for (field_name, value) in flat_fields.iter() {
        // Skip fields we drop from templates
        if field_name == "host" || field_name == "hostname" || field_name == "service" ||
           field_name.starts_with("kubernetes.") || field_name == "pod" || 
           field_name == "namespace" || field_name == "container" || field_name == "container_id" {
            continue;
        }
        
        // Use uppercase field name as parameter type
        let param_type = field_name.to_uppercase().replace("-", "_").replace(".", "_");
        params.entry(param_type).or_insert_with(Vec::new).push(value.clone());
    }
    
    params
}

/// Combines parameters from masking and structured extraction
pub fn merge_params(masked_params: HashMap<String, Vec<String>>, kv_params: HashMap<String, Vec<String>>) -> HashMap<String, Vec<String>> {
    let mut merged = masked_params;
    
    for (key, values) in kv_params {
        merged.entry(key).or_default().extend(values);
    }
    
    // Deduplicate values within each parameter type
    for values in merged.values_mut() {
        values.sort();
        values.dedup();
    }
    
    merged
}

/// Attempts to flatten JSON into sorted key-value pairs
/// Returns None if the input is not valid JSON
pub fn try_flatten_json(input: &str) -> Option<BTreeMap<String, String>> {
    // Try to parse as JSON
    let json_value: serde_json::Value = serde_json::from_str(input.trim()).ok()?;
    
    // Only process JSON objects, not arrays or primitives
    let obj = json_value.as_object()?;
    
    let mut result = BTreeMap::new();
    flatten_json_object("", obj, &mut result);
    
    Some(result)
}

/// Recursively flattens a JSON object into dot-separated key paths
fn flatten_json_object(prefix: &str, obj: &serde_json::Map<String, serde_json::Value>, result: &mut BTreeMap<String, String>) {
    for (key, value) in obj {
        let full_key = if prefix.is_empty() {
            key.clone()
        } else {
            format!("{prefix}.{key}")
        };
        
        match value {
            serde_json::Value::Object(nested_obj) => {
                flatten_json_object(&full_key, nested_obj, result);
            }
            serde_json::Value::Array(arr) => {
                // For arrays, use the array length as a simple representation
                result.insert(full_key, format!("array[{}]", arr.len()));
            }
            _ => {
                // Convert all other types to string
                result.insert(full_key, value_to_simple_string(value));
            }
        }
    }
}

/// Convert JSON value to a simple string representation
fn value_to_simple_string(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Null => "null".to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Array(_) => "[array]".to_string(),
        serde_json::Value::Object(_) => "[object]".to_string(),
    }
}

/// Main canonicalization function that prepares text for Drain clustering
/// This function applies structure-first canonicalization:
/// - For JSON logs: converts to sorted "key = <KEY>" format
/// - For inline KV logs: rewrites "key=value" to "key = <KEY>"
/// - Then applies existing masking for any remaining free text
pub fn canonicalize_for_drain(input: &str) -> MaskingResult {
    // First, try to parse as JSON for structured canonicalization
    if let Some(json_fields) = try_flatten_json(input) {
        return canonicalize_json_structure(&json_fields);
    }
    
    // Fast-path: Check for simple key-value pairs before expensive smart masking
    // This avoids regex compilation overhead for simple KV logs
    if has_kv_pairs(input) {
        return canonicalize_kv_structure(input);
    }
    
    // Try smart masking for known log formats (ELB, Nginx, Apache, etc.)
    // This is expensive on first use due to regex compilation
    let smart_result = crate::smart_masking::smart_mask_line(input);
    if smart_result.confidence > 0.8 {
        return MaskingResult {
            masked_text: smart_result.template,
            extracted_params: smart_result.parameters,
        };
    }
    
    // Fallback to traditional masking for unstructured text with disambiguation
    mask_and_extract_with_disambiguation(input)
}

/// Canonicalizes JSON structure into sorted key=<KEY> format
fn canonicalize_json_structure(fields: &BTreeMap<String, String>) -> MaskingResult {
    let mut canonicalized_parts = Vec::new();
    let mut extracted_params = HashMap::new();
    
    // Process fields in sorted order for consistency
    for (field_name, field_value) in fields.iter() {
        // Skip infrastructure fields we don't want to track
        if should_skip_field(field_name) {
            continue;
        }
        
        // Create field-specific placeholder
        let field_upper = field_name.to_uppercase().replace("-", "_").replace(".", "_");
        let placeholder = format!("<{field_upper}>");
        
        // Add to canonicalized format
        canonicalized_parts.push(format!("{field_name} = {placeholder}"));
        
        // Track the original value
        extracted_params.entry(field_upper).or_insert_with(Vec::new).push(field_value.clone());
    }
    
    let canonicalized_text = canonicalized_parts.join(" ");
    
    // For JSON canonicalization, we don't need additional masking since we've already
    // converted all values to structured placeholders
    MaskingResult {
        masked_text: canonicalized_text,
        extracted_params,
    }
}

/// Canonicalizes key-value pairs found in text into consistent format
/// Handles mixed content - replaces KV pairs with placeholders, keeps other text as-is
fn canonicalize_kv_structure(input: &str) -> MaskingResult {
    let mut extracted_params = HashMap::new();
    let mut result = String::new();
    let mut last_end = 0;
    
    // Use captures_iter for single-pass processing (avoids double regex execution)
    for captures in RE_KV_EXTRACT.captures_iter(input) {
        let mat = captures.get(0).unwrap();
        let key = captures.get(1).unwrap().as_str();
        let value = captures.get(2).unwrap().as_str();
        
        // Add any text before this match
        if mat.start() > last_end {
            result.push_str(&input[last_end..mat.start()]);
        }
        
        // Skip infrastructure fields
        if should_skip_field(key) {
            result.push_str(mat.as_str());
        } else {
            // Replace with placeholder
            let key_upper = key.to_uppercase().replace("-", "_").replace(".", "_");
            let placeholder = format!("{key} = <{key_upper}>");
            result.push_str(&placeholder);
            
            // Track the original value (strip trailing comma if present)
            let clean_value = value.trim_end_matches(',');
            extracted_params.entry(key_upper).or_insert_with(Vec::new).push(clean_value.to_string());
        }
        
        last_end = mat.end();
    }
    
    // Add any remaining text after the last match
    if last_end < input.len() {
        result.push_str(&input[last_end..]);
    }
    
    MaskingResult {
        masked_text: result,
        extracted_params,
    }
}

/// Checks if input contains key-value pairs
fn has_kv_pairs(input: &str) -> bool {
    // Simple check: does it contain '=' and look like key=value?
    // Avoid regex for detection to prevent any catastrophic backtracking
    if !input.contains('=') {
        return false;
    }
    
    // Quick heuristic: check if there's at least one word followed by '='
    for (i, ch) in input.char_indices() {
        if ch == '=' && i > 0 {
            // Check if there's a word character before '='
            let before = &input[..i];
            if before.chars().last().is_some_and(|c| c.is_alphanumeric() || c == '_') {
                return true;
            }
        }
    }
    false
}

/// Determines if a field should be skipped during canonicalization
fn should_skip_field(field_name: &str) -> bool {
    field_name == "host" || field_name == "hostname" || field_name == "service" ||
    field_name.starts_with("kubernetes.") || field_name == "pod" || 
    field_name == "namespace" || field_name == "container" || field_name == "container_id"
}

/// Pre-compile all regex patterns to avoid first-use contention in parallel processing
/// Call this once at startup before any parallel work begins
pub fn prewarm_regexes() {
    // Force initialization of all lazy regex patterns
    let _ = &*RE_TIMESTAMP;
    let _ = &*RE_NUM_UNIT;
    let _ = &*RE_NUM_PERCENT;
    let _ = &*RE_URL;
    let _ = &*RE_IPV6;
    let _ = &*RE_IPV4;
    let _ = &*RE_EMAIL;
    let _ = &*RE_UUID;
    let _ = &*RE_PATH;
    let _ = &*RE_NULL;
    let _ = &*RE_B64;
    let _ = &*RE_HEX;
    let _ = &*RE_FLOAT;
    let _ = &*RE_INT;
    let _ = &*RE_KV_PAIR;
    let _ = &*RE_KV_EXTRACT;
    
    // Also prewarm smart masking regexes
    crate::smart_masking::prewarm_regexes();
    
    // Prewarm AI module regexes  
    crate::ai::prewarm_regexes();
}