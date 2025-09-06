use crate::{masking, parser, patterns};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct Entry {
    pub id: usize,
    pub line: String,
    pub timestamp: Option<DateTime<Utc>>,
    pub template: String,
    pub service: Option<String>,
    pub host: Option<String>,
}

#[derive(Default)]
pub struct QueryIndex {
    entries: Vec<Entry>,
}

impl QueryIndex {
    pub fn new() -> Self { Self { entries: Vec::new() } }

    pub fn push_line(&mut self, line: &str) -> usize {
        let id = self.entries.len();
        let rec = parser::parse_line(line, id + 1);
        let base = if let Some(syn) = rec.synthetic_message.clone() {
            syn
        } else if let Some(ff) = rec.flat_fields.as_ref() {
            // Build a stable key=value string lazily for JSON
            let mut items: Vec<(&String, &String)> = ff.iter().collect();
            items.sort_by(|a,b| a.0.cmp(b.0));
            let mut s = String::new();
            for (i, (k,v)) in items.into_iter().enumerate() {
                if i>0 { s.push(' '); }
                s.push_str(k);
                s.push('=');
                s.push_str(v);
            }
            s
        } else {
            // Heuristic: strip syslog/app prefix up to last ": "
            if let Some(pos) = rec.message.rfind(": ") {
                rec.message[pos + 2..].to_string()
            } else {
                rec.message.clone()
            }
        };
        let masked = masking::mask_text(&base);
        let clusters = patterns::cluster_masked(&[masked.clone()]);
        let template = clusters.get(0).map(|c| c.template.clone()).unwrap_or(masked);
        let (service, host) = extract_source(&rec, line);
        self.entries.push(Entry { id, line: line.to_string(), timestamp: rec.timestamp, template, service, host });
        id
    }

    pub fn get_lines_by_pattern(&self, template: &str) -> Vec<&Entry> {
        self.entries.iter().filter(|e| e.template == template).collect()
    }

    pub fn get_lines_by_time(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        template: Option<&str>,
    ) -> Vec<&Entry> {
        self.entries
            .iter()
            .filter(|e| match e.timestamp {
                Some(ts) => ts >= start && ts < end,
                None => false,
            })
            .filter(|e| template.map(|t| e.template == t).unwrap_or(true))
            .collect()
    }

    pub fn get_context(&self, id: usize, before: usize, after: usize) -> Vec<&Entry> {
        let start = id.saturating_sub(before);
        let end = (id + after).min(self.entries.len().saturating_sub(1));
        (start..=end).filter_map(|i| self.entries.get(i)).collect()
    }

    pub fn get_lines_by_service(&self, service: &str) -> Vec<&Entry> {
        self.entries.iter().filter(|e| e.service.as_deref() == Some(service)).collect()
    }

    pub fn get_lines_by_host(&self, host: &str) -> Vec<&Entry> {
        self.entries.iter().filter(|e| e.host.as_deref() == Some(host)).collect()
    }
}

fn extract_source(rec: &parser::ParsedRecord, message: &str) -> (Option<String>, Option<String>) {
    if let Some(f) = rec.flat_fields.as_ref() {
        let service_keys = ["service", "app", "application", "kubernetes.labels.app", "kubernetes.container_name"];
        let host_keys = ["host", "hostname", "kubernetes.host", "kubernetes.node_name", "kubernetes.pod_name"];
        let mut svc = None;
        let mut host = None;
        for k in service_keys.iter() { if let Some(v) = f.get(*k) { svc = Some(v.clone()); break; } }
        for k in host_keys.iter() { if let Some(v) = f.get(*k) { host = Some(v.clone()); break; } }
        return (svc, host);
    }
    // plaintext host heuristic
    let parts: Vec<&str> = message.split_whitespace().collect();
    if parts.len() >= 4 {
        let months = ["Jan","Feb","Mar","Apr","May","Jun","Jul","Aug","Sep","Oct","Nov","Dec"]; 
        if months.contains(&parts[0]) && parts[2].contains(':') { return (None, Some(parts[3].to_string())); }
    }
    (None, None)
}
