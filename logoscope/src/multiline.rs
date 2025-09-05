use crate::parser;
use once_cell::sync::Lazy;
use regex::Regex;

#[derive(Default)]
pub struct MultiLineAggregator {
    buf: String,
    in_json: bool,
    brace_balance: i32,
}

static RE_CONT: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^(\s+|\tat\s|Caused by:|\.\.\. \d+ more)").unwrap_or_else(|_| Regex::new(r"^\s+").unwrap())
});

fn is_json_start(line: &str) -> bool {
    let t = line.trim_start();
    t.starts_with('{') || t.starts_with('[')
}

fn json_balance_delta(line: &str) -> i32 {
    let mut bal = 0;
    for ch in line.chars() {
        match ch {
            '{' | '[' => bal += 1,
            '}' | ']' => bal -= 1,
            _ => {}
        }
    }
    bal
}

impl MultiLineAggregator {
    pub fn push(&mut self, line: &str) -> Option<String> {
        // JSON accumulation
        if self.in_json {
            if !self.buf.is_empty() { self.buf.push('\n'); }
            self.buf.push_str(line);
            self.brace_balance += json_balance_delta(line);
            if self.brace_balance <= 0 {
                self.in_json = false;
                self.brace_balance = 0;
                return Some(std::mem::take(&mut self.buf));
            }
            return None;
        }

        // Start JSON accumulation
        if is_json_start(line) {
            self.in_json = true;
            self.brace_balance = json_balance_delta(line);
            self.buf.clear();
            self.buf.push_str(line);
            if self.brace_balance <= 0 {
                // single-line json
                self.in_json = false;
                self.brace_balance = 0;
                return Some(std::mem::take(&mut self.buf));
            }
            return None;
        }

        // Stack trace / continuation lines
        let is_new_entry = parser::detect_timestamp_in_text(line).is_some();
        let is_cont = RE_CONT.is_match(line);

        if self.buf.is_empty() {
            self.buf.push_str(line);
            return None;
        }

        if is_new_entry && !is_cont {
            let out = std::mem::take(&mut self.buf);
            self.buf.push_str(line);
            return Some(out);
        }

        // default continuation
        self.buf.push('\n');
        self.buf.push_str(line);
        None
    }

    pub fn finish(&mut self) -> Option<String> {
        if self.buf.is_empty() { None } else { Some(std::mem::take(&mut self.buf)) }
    }
}
