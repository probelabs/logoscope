use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::HashMap;
use std::sync::Mutex;
use std::cell::RefCell;

// Global cache for smart masking results - used as fallback when thread-local cache misses
static SMART_MASK_CACHE: Lazy<Mutex<lru::LruCache<String, SmartMaskingResult>>> = Lazy::new(|| {
    Mutex::new(lru::LruCache::new(std::num::NonZeroUsize::new(1000).unwrap()))
});

// Thread-local LRU cache (8K entries) for better performance in parallel processing
thread_local! {
    static THREAD_LOCAL_CACHE: RefCell<lru::LruCache<String, SmartMaskingResult>> = RefCell::new(
        lru::LruCache::new(std::num::NonZeroUsize::new(8192).unwrap())
    );
}

// Quick rejection patterns - lines that clearly don't match any known format
static QUICK_REJECT_PATTERNS: Lazy<Vec<Regex>> = Lazy::new(|| {
    vec![
        // Lines with no IP addresses, timestamps, or HTTP patterns at all
        Regex::new(r"^[a-zA-Z\s\-_.,;:!?@#$%^&*()+=<>\[\]{}|\\`~]*$").unwrap(), // Only letters and basic punctuation
    ]
});

// Known log format patterns
static ELB_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"^(\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d+Z)\s+(\S+)\s+([0-9.:]+)\s+([0-9.:]+)\s+([-\d.]+)\s+([-\d.]+)\s+([-\d.]+)\s+(\d+)\s+(\d+)\s+(\d+)\s+(\d+)\s+"([^"]+)"\s+"([^"]+)""#).unwrap()
});

static NGINX_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"^([0-9.:]+)\s+(\S+)\s+(\S+)\s+\[([^\]]+)\]\s+"([^"]+)"\s+(\d+)\s+(\d+)\s+"([^"]*)"\s+"([^"]+)""#).unwrap()
});

static APACHE_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"^([0-9.:]+)\s+-\s+(\S+)\s+\[([^\]]+)\]\s+"([^"]+)"\s+(\d+)\s+(\d+)"#).unwrap()
});

// Smart component patterns for fallback detection
static IP_PORT_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(\d+\.\d+\.\d+\.\d+):(\d+)").unwrap()
});

static IP_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\b(\d+\.\d+\.\d+\.\d+)\b").unwrap()
});

static HTTP_REQUEST_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#""(GET|POST|PUT|DELETE|HEAD|OPTIONS|PATCH|TRACE)\s+([^\s"]+)\s+(HTTP/[\d.]+)""#).unwrap()
});

static STATUS_CODE_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\b([1-5]\d{2})\b").unwrap()
});

// Optimized user agent pattern - much simpler and faster
// Uses a two-stage approach: quick check then detailed extraction
static USER_AGENT_QUICK_CHECK: Lazy<Regex> = Lazy::new(|| {
    // Fast check for quoted strings that likely contain user agents
    Regex::new(r#""([^"]{10,})"#).unwrap() // At least 10 chars in quotes
});

static USER_AGENT_PATTERN: Lazy<Regex> = Lazy::new(|| {
    // Simplified pattern focusing on common prefixes - much faster than 20+ alternations
    Regex::new(r#""([^"]*(?:Mozilla|curl|wget|bot|Bot|spider|HealthCheck|Monitor|Apache-HttpClient|python-requests|Go-http-client|Java|Ruby|PHP|Node\.js)[^"]*)"#).unwrap()
});

static TIMESTAMP_PATTERNS: Lazy<Vec<Regex>> = Lazy::new(|| {
    vec![
        // Enhanced ISO8601/RFC3339 with comprehensive timezone and fractional second support
        Regex::new(r"(\d{4}-\d{2}-\d{2}[T ]\d{2}:\d{2}:\d{2}(?:\.\d{1,9})?(?:Z|[+-](?:\d{2}(?::?\d{2})?|\d{4})))").unwrap(),
        // Apache/Nginx bracket format with timezone
        Regex::new(r"\[(\d{2}/\w{3}/\d{4}:\d{2}:\d{2}:\d{2}\s+[+-]\d{4})\]").unwrap(),
        // Syslog format (month day time)
        Regex::new(r"\b((Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec)\s+\d{1,2}\s+\d{2}:\d{2}:\d{2})\b").unwrap(),
        // Time with timezone (context-aware - only when no full date available)
        Regex::new(r"\b(\d{2}:\d{2}:\d{2}(?:\.\d{1,9})?[+-](?:\d{2}(?::?\d{2})?|\d{4}))\b").unwrap(),
    ]
});

// Note: Browser and OS patterns removed - we now treat the entire user agent as a single semantic unit

#[derive(Debug, Clone)]
pub enum LogFormat {
    ElasticLoadBalancer,
    NginxAccess,
    ApacheAccess,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct SmartMaskingResult {
    pub template: String,
    pub parameters: HashMap<String, Vec<String>>,
    pub format: LogFormat,
    pub confidence: f64,
}

pub fn smart_mask_line(line: &str) -> SmartMaskingResult {
    // First check thread-local cache for better performance in parallel processing
    let cached_result = THREAD_LOCAL_CACHE.with(|cache| {
        cache.borrow_mut().get(line).cloned()
    });
    
    if let Some(result) = cached_result {
        return result;
    }
    
    // Fallback to global cache via try_lock (non-blocking) to avoid contention
    if let Ok(mut global_cache) = SMART_MASK_CACHE.try_lock() {
        if let Some(cached_result) = global_cache.get(line) {
            let result = cached_result.clone();
            // Fill thread-local cache for future accesses
            THREAD_LOCAL_CACHE.with(|cache| {
                cache.borrow_mut().put(line.to_string(), result.clone());
            });
            return result;
        }
    }

    let result = smart_mask_line_impl(line);
    
    // Cache in both thread-local and global caches on miss
    THREAD_LOCAL_CACHE.with(|cache| {
        cache.borrow_mut().put(line.to_string(), result.clone());
    });
    
    // Only try to fill global cache if we can get the lock without blocking
    if let Ok(mut global_cache) = SMART_MASK_CACHE.try_lock() {
        global_cache.put(line.to_string(), result.clone());
    }
    
    result
}

fn smart_mask_line_impl(line: &str) -> SmartMaskingResult {
    // Early rejection: skip lines that clearly don't match any known patterns
    if should_quick_reject(line) {
        return quick_fallback_mask(line);
    }

    // Try to detect known formats first - these are anchored regexes so they're fast
    if let Some(result) = try_elb_format(line) {
        return result;
    }
    
    if let Some(result) = try_nginx_format(line) {
        return result;
    }
    
    if let Some(result) = try_apache_format(line) {
        return result;
    }
    
    // Fallback to intelligent component detection
    fallback_smart_mask(line)
}

fn should_quick_reject(line: &str) -> bool {
    // Quick checks for obvious non-matches
    if line.len() < 20 {
        return true; // Too short to be a real log entry
    }
    
    // Check if line contains any digits or IPs (most logs have these)
    if !line.chars().any(|c| c.is_ascii_digit()) {
        return true;
    }
    
    // Check against rejection patterns
    for pattern in QUICK_REJECT_PATTERNS.iter() {
        if pattern.is_match(line) {
            return true;
        }
    }
    
    false
}

fn quick_fallback_mask(line: &str) -> SmartMaskingResult {
    // Minimal processing for obviously non-matching lines
    let number_pattern = Regex::new(r"\b\d+\.?\d*\b").unwrap();
    let masked_line = number_pattern.replace_all(line, "<NUM>").to_string();
    
    SmartMaskingResult {
        template: masked_line,
        parameters: HashMap::new(),
        format: LogFormat::Unknown,
        confidence: 0.1, // Very low confidence
    }
}

fn try_elb_format(line: &str) -> Option<SmartMaskingResult> {
    if let Some(caps) = ELB_PATTERN.captures(line) {
        let mut parameters = HashMap::new();
        
        // Extract all components with semantic names
        if let Some(timestamp) = caps.get(1) {
            parameters.insert("TIMESTAMP".to_string(), vec![timestamp.as_str().to_string()]);
        }
        if let Some(lb_name) = caps.get(2) {
            parameters.insert("LOAD_BALANCER".to_string(), vec![lb_name.as_str().to_string()]);
        }
        if let Some(client_addr) = caps.get(3) {
            let parts: Vec<&str> = client_addr.as_str().split(':').collect();
            if parts.len() == 2 {
                parameters.insert("CLIENT_IP".to_string(), vec![parts[0].to_string()]);
                parameters.insert("CLIENT_PORT".to_string(), vec![parts[1].to_string()]);
            }
            parameters.insert("CLIENT_ADDR".to_string(), vec![client_addr.as_str().to_string()]);
        }
        if let Some(target_addr) = caps.get(4) {
            let parts: Vec<&str> = target_addr.as_str().split(':').collect();
            if parts.len() == 2 {
                parameters.insert("TARGET_IP".to_string(), vec![parts[0].to_string()]);
                parameters.insert("TARGET_PORT".to_string(), vec![parts[1].to_string()]);
            }
            parameters.insert("TARGET_ADDR".to_string(), vec![target_addr.as_str().to_string()]);
        }
        
        // Timing metrics
        if let Some(req_time) = caps.get(5) {
            parameters.insert("REQUEST_TIME".to_string(), vec![req_time.as_str().to_string()]);
        }
        if let Some(target_time) = caps.get(6) {
            parameters.insert("TARGET_TIME".to_string(), vec![target_time.as_str().to_string()]);
        }
        if let Some(resp_time) = caps.get(7) {
            parameters.insert("RESPONSE_TIME".to_string(), vec![resp_time.as_str().to_string()]);
        }
        
        // Status codes
        if let Some(elb_status) = caps.get(8) {
            parameters.insert("ELB_STATUS".to_string(), vec![elb_status.as_str().to_string()]);
        }
        if let Some(target_status) = caps.get(9) {
            parameters.insert("TARGET_STATUS".to_string(), vec![target_status.as_str().to_string()]);
        }
        
        // Byte counts
        if let Some(received_bytes) = caps.get(10) {
            parameters.insert("RECEIVED_BYTES".to_string(), vec![received_bytes.as_str().to_string()]);
        }
        if let Some(sent_bytes) = caps.get(11) {
            parameters.insert("SENT_BYTES".to_string(), vec![sent_bytes.as_str().to_string()]);
        }
        
        // HTTP request parsing
        if let Some(request) = caps.get(12) {
            let (method, path, version) = parse_http_request(request.as_str());
            parameters.insert("HTTP_METHOD".to_string(), vec![method.clone()]);
            parameters.insert("REQUEST_PATH".to_string(), vec![path]);
            parameters.insert("HTTP_VERSION".to_string(), vec![version.clone()]);
        }
        
        // User agent - store as single unit
        if let Some(user_agent) = caps.get(13) {
            parameters.insert("USER_AGENT".to_string(), vec![user_agent.as_str().to_string()]);
        }
        
        let template = "<TIMESTAMP> <LOAD_BALANCER> <CLIENT_IP>:<CLIENT_PORT> <TARGET_IP>:<TARGET_PORT> <REQUEST_TIME> <TARGET_TIME> <RESPONSE_TIME> <ELB_STATUS> <TARGET_STATUS> <RECEIVED_BYTES> <SENT_BYTES> \"<HTTP_METHOD> <REQUEST_PATH> <HTTP_VERSION>\" \"<USER_AGENT>\"".to_string();
        
        return Some(SmartMaskingResult {
            template,
            parameters,
            format: LogFormat::ElasticLoadBalancer,
            confidence: 0.95,
        });
    }
    
    None
}

fn try_nginx_format(line: &str) -> Option<SmartMaskingResult> {
    if let Some(caps) = NGINX_PATTERN.captures(line) {
        let mut parameters = HashMap::new();
        
        if let Some(client_ip) = caps.get(1) {
            parameters.insert("CLIENT_IP".to_string(), vec![client_ip.as_str().to_string()]);
        }
        if let Some(remote_logname) = caps.get(2) {
            if remote_logname.as_str() != "-" {
                parameters.insert("REMOTE_LOGNAME".to_string(), vec![remote_logname.as_str().to_string()]);
            }
        }
        if let Some(remote_user) = caps.get(3) {
            if remote_user.as_str() != "-" {
                parameters.insert("REMOTE_USER".to_string(), vec![remote_user.as_str().to_string()]);
            }
        }
        if let Some(timestamp) = caps.get(4) {
            parameters.insert("TIMESTAMP".to_string(), vec![timestamp.as_str().to_string()]);
        }
        if let Some(request) = caps.get(5) {
            let (method, path, version) = parse_http_request(request.as_str());
            parameters.insert("HTTP_METHOD".to_string(), vec![method]);
            parameters.insert("REQUEST_PATH".to_string(), vec![path]);
            parameters.insert("HTTP_VERSION".to_string(), vec![version]);
        }
        if let Some(status) = caps.get(6) {
            parameters.insert("STATUS_CODE".to_string(), vec![status.as_str().to_string()]);
        }
        if let Some(size) = caps.get(7) {
            parameters.insert("RESPONSE_SIZE".to_string(), vec![size.as_str().to_string()]);
        }
        if let Some(referer) = caps.get(8) {
            if referer.as_str() != "-" {
                parameters.insert("REFERER".to_string(), vec![referer.as_str().to_string()]);
            }
        }
        if let Some(user_agent) = caps.get(9) {
            parameters.insert("USER_AGENT".to_string(), vec![user_agent.as_str().to_string()]);
        }
        
        let template = "<CLIENT_IP> <REMOTE_LOGNAME> <REMOTE_USER> [<TIMESTAMP>] \"<HTTP_METHOD> <REQUEST_PATH> <HTTP_VERSION>\" <STATUS_CODE> <RESPONSE_SIZE> \"<REFERER>\" \"<USER_AGENT>\"".to_string();
        
        return Some(SmartMaskingResult {
            template,
            parameters,
            format: LogFormat::NginxAccess,
            confidence: 0.90,
        });
    }
    
    None
}

fn try_apache_format(line: &str) -> Option<SmartMaskingResult> {
    if let Some(caps) = APACHE_PATTERN.captures(line) {
        let mut parameters = HashMap::new();
        
        if let Some(client_ip) = caps.get(1) {
            parameters.insert("CLIENT_IP".to_string(), vec![client_ip.as_str().to_string()]);
        }
        if let Some(user) = caps.get(2) {
            if user.as_str() != "-" {
                parameters.insert("REMOTE_USER".to_string(), vec![user.as_str().to_string()]);
            }
        }
        if let Some(timestamp) = caps.get(3) {
            parameters.insert("TIMESTAMP".to_string(), vec![timestamp.as_str().to_string()]);
        }
        if let Some(request) = caps.get(4) {
            let (method, path, version) = parse_http_request(request.as_str());
            parameters.insert("HTTP_METHOD".to_string(), vec![method]);
            parameters.insert("REQUEST_PATH".to_string(), vec![path]);
            parameters.insert("HTTP_VERSION".to_string(), vec![version]);
        }
        if let Some(status) = caps.get(5) {
            parameters.insert("STATUS_CODE".to_string(), vec![status.as_str().to_string()]);
        }
        if let Some(size) = caps.get(6) {
            parameters.insert("RESPONSE_SIZE".to_string(), vec![size.as_str().to_string()]);
        }
        
        let template = "<CLIENT_IP> - <REMOTE_USER> [<TIMESTAMP>] \"<HTTP_METHOD> <REQUEST_PATH> <HTTP_VERSION>\" <STATUS_CODE> <RESPONSE_SIZE>".to_string();
        
        return Some(SmartMaskingResult {
            template,
            parameters,
            format: LogFormat::ApacheAccess,
            confidence: 0.85,
        });
    }
    
    None
}

fn fallback_smart_mask(line: &str) -> SmartMaskingResult {
    let mut parameters = HashMap::new();
    let mut masked_line = line.to_string();
    
    // Extract IP:PORT combinations first (more specific)
    for caps in IP_PORT_PATTERN.captures_iter(line) {
        if let (Some(ip), Some(port)) = (caps.get(1), caps.get(2)) {
            parameters.entry("CLIENT_IP".to_string()).or_insert_with(Vec::new).push(ip.as_str().to_string());
            parameters.entry("PORT".to_string()).or_insert_with(Vec::new).push(port.as_str().to_string());
            masked_line = masked_line.replace(&caps[0], "<IP>:<PORT>");
        }
    }
    
    // Extract standalone IPs
    for caps in IP_PATTERN.captures_iter(line) {
        if let Some(ip) = caps.get(1) {
            // Skip if already processed as part of IP:PORT
            if !parameters.get("CLIENT_IP").unwrap_or(&vec![]).contains(&ip.as_str().to_string()) {
                parameters.entry("IP".to_string()).or_insert_with(Vec::new).push(ip.as_str().to_string());
                masked_line = masked_line.replace(ip.as_str(), "<IP>");
            }
        }
    }
    
    // Extract HTTP requests
    if let Some(caps) = HTTP_REQUEST_PATTERN.captures(line) {
        if let (Some(method), Some(path), Some(version)) = (caps.get(1), caps.get(2), caps.get(3)) {
            parameters.insert("HTTP_METHOD".to_string(), vec![method.as_str().to_string()]);
            parameters.insert("REQUEST_PATH".to_string(), vec![path.as_str().to_string()]);
            parameters.insert("HTTP_VERSION".to_string(), vec![version.as_str().to_string()]);
            masked_line = masked_line.replace(&caps[0], "\"<HTTP_METHOD> <REQUEST_PATH> <HTTP_VERSION>\"");
        }
    }
    
    // Extract status codes
    for caps in STATUS_CODE_PATTERN.captures_iter(line) {
        if let Some(status) = caps.get(1) {
            parameters.entry("STATUS_CODE".to_string()).or_insert_with(Vec::new).push(status.as_str().to_string());
            masked_line = masked_line.replace(status.as_str(), "<STATUS_CODE>");
        }
    }
    
    // Extract user agents as a single unit - use two-stage approach for performance
    if USER_AGENT_QUICK_CHECK.is_match(line) {
        if let Some(caps) = USER_AGENT_PATTERN.captures(line) {
            if let Some(ua) = caps.get(1) {
                parameters.insert("USER_AGENT".to_string(), vec![ua.as_str().to_string()]);
                masked_line = masked_line.replace(&caps[0], "\"<USER_AGENT>\"");
            }
        }
    }
    
    // Extract timestamps
    for pattern in TIMESTAMP_PATTERNS.iter() {
        if let Some(caps) = pattern.captures(line) {
            if let Some(timestamp) = caps.get(1) {
                parameters.insert("TIMESTAMP".to_string(), vec![timestamp.as_str().to_string()]);
                masked_line = masked_line.replace(timestamp.as_str(), "<TIMESTAMP>");
                break; // Only match first timestamp pattern
            }
        }
    }
    
    // Finally, replace remaining numbers
    let number_pattern = Regex::new(r"\b\d+\.?\d*\b").unwrap();
    masked_line = number_pattern.replace_all(&masked_line, "<NUM>").to_string();
    
    SmartMaskingResult {
        template: masked_line,
        parameters,
        format: LogFormat::Unknown,
        confidence: 0.5,
    }
}

fn parse_http_request(request: &str) -> (String, String, String) {
    if let Some(caps) = HTTP_REQUEST_PATTERN.captures(&format!("\"{request}\"")) {
        let method = caps.get(1).map(|m| m.as_str()).unwrap_or("UNKNOWN").to_string();
        let path = caps.get(2).map(|m| m.as_str()).unwrap_or("/").to_string();
        let version = caps.get(3).map(|m| m.as_str()).unwrap_or("HTTP/1.0").to_string();
        (method, path, version)
    } else {
        // Fallback parsing
        let parts: Vec<&str> = request.split_whitespace().collect();
        let method = parts.first().unwrap_or(&"UNKNOWN").to_string();
        let path = parts.get(1).unwrap_or(&"/").to_string();
        let version = parts.get(2).unwrap_or(&"HTTP/1.0").to_string();
        (method, path, version)
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_elb_smart_masking() {
        let line = r#"2024-03-05T11:09:51.074031Z awseb-e-m-AWSEBLoa-BKP6LS5P8QLF 172.30.1.251:48530 172.30.1.4:9000 0.000017 0.000791 0.000009 200 200 0 215 "GET http://teamauthapiproduction.cloud.tyk.io:9000/assets/plugins/FitVids/?918138%40 HTTP/1.0" "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/117.0.0.0 Safari/537.36""#;
        
        let result = smart_mask_line(line);
        assert!(matches!(result.format, LogFormat::ElasticLoadBalancer));
        assert!(result.confidence > 0.9);
        assert!(result.parameters.contains_key("CLIENT_IP"));
        assert!(result.parameters.contains_key("HTTP_METHOD"));
        assert!(result.parameters.contains_key("USER_AGENT"));
        
        // Verify specific extracted values
        assert_eq!(result.parameters.get("CLIENT_IP").unwrap(), &vec!["172.30.1.251".to_string()]);
        assert_eq!(result.parameters.get("HTTP_METHOD").unwrap(), &vec!["GET".to_string()]);
        assert!(result.parameters.get("USER_AGENT").unwrap()[0].contains("Chrome"));
        assert_eq!(result.parameters.get("ELB_STATUS").unwrap(), &vec!["200".to_string()]);
        assert_eq!(result.parameters.get("TARGET_STATUS").unwrap(), &vec!["200".to_string()]);
    }

    #[test]
    fn test_elb_different_user_agents() {
        // Test curl user agent
        let line = r#"2024-03-05T11:09:51.074031Z my-lb 10.0.0.1:12345 10.0.0.2:80 0.001 0.002 0.003 200 200 500 1024 "POST /api/data HTTP/1.1" "curl/7.68.0""#;
        let result = smart_mask_line(line);
        assert!(matches!(result.format, LogFormat::ElasticLoadBalancer));
        assert_eq!(result.parameters.get("USER_AGENT").unwrap(), &vec!["curl/7.68.0".to_string()]);

        // Test health checker
        let line2 = r#"2024-03-05T11:09:51.074031Z my-lb 10.0.0.1:12345 10.0.0.2:80 0.001 0.002 0.003 200 200 500 1024 "GET /health HTTP/1.1" "HealthChecker/2.0""#;
        let result2 = smart_mask_line(line2);
        assert_eq!(result2.parameters.get("USER_AGENT").unwrap(), &vec!["HealthChecker/2.0".to_string()]);
    }

    #[test]
    fn test_nginx_smart_masking() {
        let line = r#"192.168.1.100 - - [05/Mar/2024:11:09:51 +0000] "GET /api/v1/users HTTP/1.1" 200 1234 "https://example.com/dashboard" "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36""#;
        
        let result = smart_mask_line(line);
        assert!(matches!(result.format, LogFormat::NginxAccess));
        assert!(result.confidence > 0.8);
        assert!(result.parameters.contains_key("CLIENT_IP"));
        assert!(result.parameters.contains_key("HTTP_METHOD"));
        assert!(result.parameters.contains_key("STATUS_CODE"));
        
        // Verify specific extracted values
        assert_eq!(result.parameters.get("CLIENT_IP").unwrap(), &vec!["192.168.1.100".to_string()]);
        assert_eq!(result.parameters.get("HTTP_METHOD").unwrap(), &vec!["GET".to_string()]);
        assert_eq!(result.parameters.get("STATUS_CODE").unwrap(), &vec!["200".to_string()]);
        assert_eq!(result.parameters.get("REQUEST_PATH").unwrap(), &vec!["/api/v1/users".to_string()]);
        assert_eq!(result.parameters.get("RESPONSE_SIZE").unwrap(), &vec!["1234".to_string()]);
        assert_eq!(result.parameters.get("REFERER").unwrap(), &vec!["https://example.com/dashboard".to_string()]);
    }

    #[test]
    fn test_nginx_with_authenticated_user() {
        let line = r#"10.0.0.1 - john_doe [05/Mar/2024:11:09:51 +0000] "POST /api/upload HTTP/1.1" 201 512 "-" "MyApp/1.2.3""#;
        let result = smart_mask_line(line);
        assert!(matches!(result.format, LogFormat::NginxAccess));
        assert_eq!(result.parameters.get("REMOTE_USER").unwrap(), &vec!["john_doe".to_string()]);
        assert_eq!(result.parameters.get("HTTP_METHOD").unwrap(), &vec!["POST".to_string()]);
        assert_eq!(result.parameters.get("STATUS_CODE").unwrap(), &vec!["201".to_string()]);
    }

    #[test]
    fn test_apache_smart_masking() {
        let line = r#"127.0.0.1 - frank [10/Mar/2024:13:55:36 +0100] "GET /apache_pb.gif HTTP/1.0" 200 2326"#;
        let result = smart_mask_line(line);
        assert!(matches!(result.format, LogFormat::ApacheAccess));
        assert!(result.confidence > 0.8);
        assert_eq!(result.parameters.get("CLIENT_IP").unwrap(), &vec!["127.0.0.1".to_string()]);
        assert_eq!(result.parameters.get("REMOTE_USER").unwrap(), &vec!["frank".to_string()]);
        assert_eq!(result.parameters.get("HTTP_METHOD").unwrap(), &vec!["GET".to_string()]);
        assert_eq!(result.parameters.get("REQUEST_PATH").unwrap(), &vec!["/apache_pb.gif".to_string()]);
        assert_eq!(result.parameters.get("STATUS_CODE").unwrap(), &vec!["200".to_string()]);
        assert_eq!(result.parameters.get("RESPONSE_SIZE").unwrap(), &vec!["2326".to_string()]);
    }

    #[test]
    fn test_apache_without_user() {
        let line = r#"203.0.113.12 - - [10/Mar/2024:13:55:36 +0100] "HEAD /index.html HTTP/1.1" 404 0"#;
        let result = smart_mask_line(line);
        assert!(matches!(result.format, LogFormat::ApacheAccess));
        assert_eq!(result.parameters.get("CLIENT_IP").unwrap(), &vec!["203.0.113.12".to_string()]);
        assert_eq!(result.parameters.get("HTTP_METHOD").unwrap(), &vec!["HEAD".to_string()]);
        assert_eq!(result.parameters.get("STATUS_CODE").unwrap(), &vec!["404".to_string()]);
        // Should not have REMOTE_USER since it's "-"
        assert!(!result.parameters.contains_key("REMOTE_USER"));
    }

    #[test]
    fn test_confidence_scoring() {
        // High confidence: perfect ELB match
        let elb_line = r#"2024-03-05T11:09:51.074031Z my-lb 1.2.3.4:80 5.6.7.8:443 0.1 0.2 0.3 200 200 100 200 "GET /test HTTP/1.1" "curl/7.0""#;
        let elb_result = smart_mask_line(elb_line);
        assert_eq!(elb_result.confidence, 0.95);

        // Medium confidence: Nginx match
        let nginx_line = r#"1.2.3.4 - - [05/Mar/2024:11:09:51 +0000] "GET / HTTP/1.1" 200 100 "-" "curl/7.0""#;
        let nginx_result = smart_mask_line(nginx_line);
        assert_eq!(nginx_result.confidence, 0.90);

        // Lower confidence: fallback pattern matching
        let custom_line = "Some custom log with 192.168.1.1:8080 and status 404";
        let custom_result = smart_mask_line(custom_line);
        assert_eq!(custom_result.confidence, 0.5);
    }

    #[test]
    fn test_fallback_smart_masking() {
        let line = "Custom log 192.168.1.100:8080 GET /api/test 404 Mozilla/5.0";
        
        let result = smart_mask_line(line);
        assert!(matches!(result.format, LogFormat::Unknown));
        assert!(result.parameters.contains_key("CLIENT_IP"));
        assert!(result.parameters.contains_key("PORT"));
        assert!(result.template.contains("<IP>:<PORT>"));
    }

    #[test]
    fn test_fallback_with_multiple_components() {
        let line = r#"App log: 10.0.0.1:443 -> 10.0.0.2:80 "POST /upload HTTP/1.1" 201 "Chrome/91.0""#;
        let result = smart_mask_line(line);
        
        assert!(matches!(result.format, LogFormat::Unknown));
        assert!(result.parameters.contains_key("CLIENT_IP"));
        assert!(result.parameters.contains_key("PORT"));
        assert!(result.parameters.contains_key("HTTP_METHOD"));
        assert!(result.parameters.contains_key("STATUS_CODE"));
        
        // Should have extracted both IPs
        let ips = result.parameters.get("CLIENT_IP").unwrap();
        assert_eq!(ips.len(), 2);
        assert!(ips.contains(&"10.0.0.1".to_string()));
        assert!(ips.contains(&"10.0.0.2".to_string()));
    }

    #[test]
    fn test_malformed_inputs() {
        // Empty line - should be quick rejected
        let result = smart_mask_line("");
        assert!(matches!(result.format, LogFormat::Unknown));
        assert_eq!(result.confidence, 0.1); // Quick rejection gives 0.1 confidence

        // Only whitespace - should be quick rejected
        let result = smart_mask_line("   \t  \n  ");
        assert!(matches!(result.format, LogFormat::Unknown));
        assert_eq!(result.confidence, 0.1); // Quick rejection gives 0.1 confidence

        // Partial ELB line (missing components) - not quick rejected, goes to fallback
        let result = smart_mask_line("2024-03-05T11:09:51.074031Z incomplete");
        assert!(matches!(result.format, LogFormat::Unknown));
        assert_eq!(result.confidence, 0.5); // Fallback gives 0.5 confidence
    }

    #[test]
    fn test_edge_cases() {
        // Line with only IPs
        let line = "192.168.1.1 10.0.0.1 172.16.0.1";
        let result = smart_mask_line(line);
        assert!(result.parameters.contains_key("IP"));
        let ips = result.parameters.get("IP").unwrap();
        assert_eq!(ips.len(), 3);

        // Line with only timestamps
        let line = "2024-03-05T11:09:51.074031Z [05/Mar/2024:11:09:51 +0000] 2024-03-05 11:09:51";
        let result = smart_mask_line(line);
        assert!(result.parameters.contains_key("TIMESTAMP"));
        // Should match the first timestamp pattern
        assert_eq!(result.parameters.get("TIMESTAMP").unwrap().len(), 1);

        // Line with mixed numbers and status codes
        let line = "Processing 12345 items, got status 404, took 1.2 seconds";
        let result = smart_mask_line(line);
        assert!(result.parameters.contains_key("STATUS_CODE"));
        assert_eq!(result.parameters.get("STATUS_CODE").unwrap(), &vec!["404".to_string()]);
        assert!(result.template.contains("<NUM>"));
    }

    #[test]
    fn test_user_agent_detection() {
        // Test that user agents are detected as single units
        let line = r#"192.168.1.100 - - [05/Mar/2024:11:09:51 +0000] "GET /api/v1/users HTTP/1.1" 200 1234 "https://example.com" "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36""#;
        let result = fallback_smart_mask(line);
        assert!(result.parameters.contains_key("USER_AGENT"));
        assert_eq!(result.parameters["USER_AGENT"][0], "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36");

        // Test various user agent formats
        let agents = vec![
            "curl/7.68.0",
            "Googlebot/2.1 (+http://www.google.com/bot.html)",
            "HealthChecker/2.0 (monitoring)",
            "Apache-HttpClient/4.5.13 (Java/11.0.16)",
            "python-requests/2.28.1",
        ];

        for agent in agents {
            let line = format!(r#"test log "{}" rest"#, agent);
            let caps = USER_AGENT_PATTERN.captures(&line);
            assert!(caps.is_some(), "Failed to match user agent: {}", agent);
        }
    }

    #[test]
    fn test_http_request_parsing() {
        // Standard HTTP request
        let (method, path, version) = parse_http_request("GET /api/v1/users HTTP/1.1");
        assert_eq!(method, "GET");
        assert_eq!(path, "/api/v1/users");
        assert_eq!(version, "HTTP/1.1");

        // Request with query parameters
        let (method, path, version) = parse_http_request("POST /submit?id=123&type=data HTTP/2.0");
        assert_eq!(method, "POST");
        assert_eq!(path, "/submit?id=123&type=data");
        assert_eq!(version, "HTTP/2.0");

        // Malformed request (fallback parsing)
        let (method, path, version) = parse_http_request("INVALID");
        assert_eq!(method, "INVALID");
        assert_eq!(path, "/");
        assert_eq!(version, "HTTP/1.0");

        // Empty request
        let (method, path, version) = parse_http_request("");
        assert_eq!(method, "UNKNOWN");
        assert_eq!(path, "/");
        assert_eq!(version, "HTTP/1.0");
    }

    #[test]
    fn test_integration_with_param_extractor() {
        // Test that smart masking results integrate properly
        let line = r#"192.168.1.100 - - [05/Mar/2024:11:09:51 +0000] "GET /api/v1/users HTTP/1.1" 200 1234 "https://example.com/dashboard" "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36""#;
        
        let result = smart_mask_line(line);
        
        // Verify the template is suitable for further processing
        assert!(!result.template.is_empty());
        assert!(result.template.contains("<CLIENT_IP>"));
        assert!(result.template.contains("<HTTP_METHOD>"));
        assert!(result.template.contains("<REQUEST_PATH>"));
        
        // Verify parameters can be used for anomaly detection
        assert!(!result.parameters.is_empty());
        for (param_type, values) in &result.parameters {
            assert!(!param_type.is_empty());
            assert!(!values.is_empty());
            for value in values {
                assert!(!value.is_empty());
            }
        }
    }

    #[test]
    fn test_drain_bypass_conditions() {
        // High confidence result should bypass Drain (0.95 > 0.8)
        let elb_line = r#"2024-03-05T11:09:51.074031Z my-lb 1.2.3.4:80 5.6.7.8:443 0.1 0.2 0.3 200 200 100 200 "GET /test HTTP/1.1" "curl/7.0""#;
        let result = smart_mask_line(elb_line);
        assert!(result.confidence > 0.8, "ELB should have high confidence for Drain bypass");

        // Medium confidence should still bypass (0.90 > 0.8)
        let nginx_line = r#"1.2.3.4 - - [05/Mar/2024:11:09:51 +0000] "GET / HTTP/1.1" 200 100 "-" "curl/7.0""#;
        let result = smart_mask_line(nginx_line);
        assert!(result.confidence > 0.8, "Nginx should have high confidence for Drain bypass");

        // Low confidence should not bypass (0.5 <= 0.8)
        let unknown_line = "Random log entry with no pattern";
        let result = smart_mask_line(unknown_line);
        assert!(result.confidence <= 0.8, "Unknown format should not bypass Drain");
    }
}

/// Pre-compile all regex patterns to avoid first-use contention in parallel processing
pub fn prewarm_regexes() {
    // Force initialization of all lazy regex patterns
    let _ = &*QUICK_REJECT_PATTERNS;
    let _ = &*ELB_PATTERN;
    let _ = &*NGINX_PATTERN;
    let _ = &*APACHE_PATTERN;
    let _ = &*IP_PORT_PATTERN;
    let _ = &*IP_PATTERN;
    let _ = &*HTTP_REQUEST_PATTERN;
    let _ = &*STATUS_CODE_PATTERN;
    let _ = &*USER_AGENT_QUICK_CHECK;
    let _ = &*USER_AGENT_PATTERN;
    let _ = &*TIMESTAMP_PATTERNS;
    
    // Note: SMART_MASK_CACHE is not a regex, it's an LRU cache
}