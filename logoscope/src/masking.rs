use once_cell::sync::Lazy;
use regex::Regex;

static RE_TIMESTAMP: Lazy<Regex> = Lazy::new(|| {
    // Enhanced ISO8601/RFC3339 with comprehensive timezone and fractional second support
    // Supports: 2025-08-07T06:41:18Z, 2025-08-07T06:41:18.123456+01:00, 2025-08-07 06:41:18.999-0800
    // Fractional seconds: 1-9 digits (.1 to .123456789)  
    // Timezones: Z, ±HH:MM, ±HHMM, ±HH
    Regex::new(r"\b\d{4}-\d{2}-\d{2}[T ]\d{2}:\d{2}:\d{2}(?:\.\d{1,9})?(?:Z|[+-](?:\d{2}(?::?\d{2})?|\d{4}))\b").unwrap()
});

// Numbers with common unit suffixes (duration/size/percent). Preserve suffix.
static RE_NUM_UNIT: Lazy<Regex> = Lazy::new(|| {
    // Case-insensitive suffix match; optional fractional part; optional thin space before unit
    // Examples: 15ms, 15.3ms, 2s, 1.5h, 120KB, 10MiB, 99%
    Regex::new(r"(?i)\b-?\d+(?:\.\d+)?(?:\s*)(ms|us|µs|ns|s|m|h|kb|mb|gb|kib|mib|gib|b|%)\b").unwrap()
});

static RE_URL: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"\b[a-zA-Z][a-zA-Z0-9+.-]*://[^\s"']+\b"#).unwrap()
});

static RE_IPV6: Lazy<Regex> = Lazy::new(|| {
    // Basic full IPv6 (no shorthand). Covers test case.
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
    // Match absolute and common relative paths; conservative to avoid overmatching
    Regex::new(r"(?x)
        (?:
            /[\w.\-]+(?:/[\w.\-]+)*
          | \./[\w.\-]+(?:/[\w.\-]+)*
          | \../[\w.\-]+(?:/[\w.\-]+)*
          | ~/[\w.\-]+(?:/[\w.\-]+)*
        )
    ").unwrap()
});

static RE_B64: Lazy<Regex> = Lazy::new(|| {
    // Base64 tokens length >= 16, allow padding
    Regex::new(r"\b[A-Za-z0-9+/]{16,}={0,2}\b").unwrap()
});

static RE_HEX: Lazy<Regex> = Lazy::new(|| {
    // Long hex sequences length >= 16
    Regex::new(r"\b[0-9a-fA-F]{16,}\b").unwrap()
});

static RE_FLOAT: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\b-?\d+\.\d+\b").unwrap()
});

static RE_INT: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\b-?\d+\b").unwrap()
});

pub fn mask_text(input: &str) -> String {
    // Order matters: timestamps, IPs, emails, then numbers
    let s = RE_TIMESTAMP.replace_all(input, "<TIMESTAMP>");
    let s = RE_URL.replace_all(&s, "<URL>");
    let s = RE_IPV6.replace_all(&s, "<IP>");
    let s = RE_IPV4.replace_all(&s, "<IP>");
    let s = RE_EMAIL.replace_all(&s, "<EMAIL>");
    let s = RE_UUID.replace_all(&s, "<UUID>");
    let s = RE_PATH.replace_all(&s, "<PATH>");
    let s = RE_HEX.replace_all(&s, "<HEX>");
    let s = RE_B64.replace_all(&s, "<B64>");
    // Replace number+unit tokens before generic float/int to avoid partial masking
    let s = RE_NUM_UNIT.replace_all(&s, "<NUM>$1");
    let s = RE_FLOAT.replace_all(&s, "<NUM>");
    let s = RE_INT.replace_all(&s, "<NUM>");
    s.into_owned()
}
