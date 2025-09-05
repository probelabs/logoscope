use clap::Parser;
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use logoscope::multiline::MultiLineAggregator;
use chrono::{DateTime, Utc, SecondsFormat};
use regex::Regex;

#[derive(Parser, Debug)]
#[command(name = "logoscope", version, about = "AI-optimized log analysis")]
struct Cli {
    /// Input files (`-` for stdin). May be repeated.
    #[arg(required = false)]
    input: Vec<String>,

    /// Timestamp field hints for JSON logs (e.g., time, ts, timestamp)
    #[arg(long = "time-key")]
    time_key: Vec<String>,

    /// Print only a specific section: patterns | logs | summary
    #[arg(long = "only")]
    only: Option<String>,

    // Pattern filters (when --only patterns)
    #[arg(long = "top")] top: Option<usize>,
    #[arg(long = "min-count")] min_count: Option<usize>,
    #[arg(long = "min-frequency")] min_frequency: Option<f64>,
    #[arg(long = "match")] match_re: Option<String>,
    #[arg(long = "exclude")] exclude_re: Option<String>,
    #[arg(long = "level")] level: Option<String>,
    #[arg(long = "examples", default_value_t = 3)] examples: usize,
    #[arg(long = "no-correlations", default_value_t = false)] no_correlations: bool,
    #[arg(long = "no-temporal", default_value_t = false)] no_temporal: bool,
    #[arg(long = "max-patterns")] max_patterns: Option<usize>,

    // Logs view flags (when --only logs)
    #[arg(long = "start")] start: Option<String>,
    #[arg(long = "end")] end: Option<String>,
    #[arg(long = "pattern")] pattern: Option<String>,
    #[arg(long = "before", short = 'B', default_value_t = 0)] before: usize,
    #[arg(long = "after", short = 'A', default_value_t = 0)] after: usize,

    /// Streaming mode: follow stdin and emit periodic summaries
    #[arg(long = "follow", default_value_t = false)] follow: bool,
    /// Streaming summary interval seconds
    #[arg(long = "interval", default_value_t = 5)] interval_secs: u64,
    /// Streaming rolling window seconds (trim old entries by log timestamp)
    #[arg(long = "window", default_value_t = 300)] window_secs: i64,
    /// Max consolidated lines kept in memory (bound)
    #[arg(long = "max-lines", default_value_t = 10000)] max_lines: usize,
    /// Fail fast on parse errors
    #[arg(long = "fail-fast", default_value_t = false)] fail_fast: bool,

    /// Patterns output format: json | table (when --only patterns)
    #[arg(long = "format", default_value = "json")] format: String,
    /// Group patterns by: none | service | level (when --only patterns)
    #[arg(long = "group-by", default_value = "none")] group_by: String,
    /// Sort patterns by: count | freq | bursts | confidence (desc)
    #[arg(long = "sort", default_value = "count")] sort_by: String,
}

fn read_all_lines(paths: &[String]) -> io::Result<Vec<String>> {
    let mut out = Vec::new();
    let mut agg = MultiLineAggregator::default();
    if paths.is_empty() {
        let stdin = io::stdin();
        let reader = stdin.lock();
        for line in reader.lines() {
            let l = line?;
            if let Some(e) = agg.push(&l) { out.push(e); }
        }
        if let Some(e) = agg.finish() { out.push(e); }
        return Ok(out);
    }
    for p in paths {
        if p == "-" {
            let stdin = io::stdin();
            let reader = stdin.lock();
            for line in reader.lines() {
                let l = line?;
                if let Some(e) = agg.push(&l) { out.push(e); }
            }
        } else {
            let f = File::open(p)?;
            let r = BufReader::new(f);
            for line in r.lines() {
                let l = line?;
                if let Some(e) = agg.push(&l) { out.push(e); }
            }
        }
    }
    if let Some(e) = agg.finish() { out.push(e); }
    Ok(out)
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let lines = read_all_lines(&cli.input)?;
    // Streaming mode (stdin only)
    if cli.follow {
        run_streaming(cli.interval_secs, cli.window_secs, cli.max_lines, cli.fail_fast)?;
        return Ok(());
    }
    let refs: Vec<&str> = lines.iter().map(|s| s.as_ref()).collect();
    // Logs-only view
    if matches!(cli.only.as_deref(), Some("logs")) {
        let mut idx = logoscope::query::QueryIndex::new();
        for l in &lines { let _ = idx.push_line(l); }
        let mut results: Vec<&logoscope::query::Entry> = Vec::new();
        if cli.start.is_some() || cli.end.is_some() {
            let s: Option<DateTime<Utc>> = cli.start.as_deref().and_then(|s| DateTime::parse_from_rfc3339(s).ok().map(|d| d.with_timezone(&Utc)));
            let e: Option<DateTime<Utc>> = cli.end.as_deref().and_then(|s| DateTime::parse_from_rfc3339(s).ok().map(|d| d.with_timezone(&Utc)));
            if let (Some(s), Some(e)) = (s, e) {
                results = idx.get_lines_by_time(s, e, cli.pattern.as_deref());
            }
        } else if let Some(p) = cli.pattern.as_ref() {
            results = idx.get_lines_by_pattern(p);
        } else {
            // default: all entries as-is
            results = (0..lines.len()).filter_map(|i| idx.get_context(i, 0, 0).get(0).copied()).collect();
        }
        if (cli.before > 0 || cli.after > 0) && !results.is_empty() {
            let id = results[0].id;
            results = idx.get_context(id, cli.before, cli.after);
        }
        for e in results {
            let ts = e.timestamp.map(|t| t.to_rfc3339_opts(SecondsFormat::Secs, true));
            println!("{}", serde_json::json!({"id": e.id, "timestamp": ts, "line": e.line}));
        }
        return Ok(());
    }

    // Full or patterns-only summary
    let out = if cli.time_key.is_empty() {
        logoscope::ai::summarize_lines(&refs)
    } else {
        let keys: Vec<&str> = cli.time_key.iter().map(|s| s.as_str()).collect();
        logoscope::ai::summarize_lines_with_hints(&refs, &keys)
    };

    if matches!(cli.only.as_deref(), Some("patterns")) {
        // Filter/sort/truncate patterns
        let mut pats = out.patterns;
        // Regex filters
        if let Some(re) = &cli.match_re { if let Ok(rx) = Regex::new(re) { pats.retain(|p| rx.is_match(&p.template)); } }
        if let Some(re) = &cli.exclude_re { if let Ok(rx) = Regex::new(re) { pats.retain(|p| !rx.is_match(&p.template)); } }
        // Level filter
        if let Some(level) = &cli.level { let lv = level.to_lowercase(); pats.retain(|p| p.severity.as_deref().map(|s| s.eq_ignore_ascii_case(&lv)).unwrap_or(false)); }
        // Min filters
        if let Some(mc) = cli.min_count { pats.retain(|p| p.total_count >= mc); }
        if let Some(mf) = cli.min_frequency { pats.retain(|p| p.frequency >= mf); }
        // Sorting
        match cli.sort_by.as_str() {
            "freq" => pats.sort_by(|a,b| b.frequency.partial_cmp(&a.frequency).unwrap().then_with(|| b.total_count.cmp(&a.total_count))),
            "bursts" => pats.sort_by(|a,b| b.temporal.bursts.cmp(&a.temporal.bursts).then_with(|| b.total_count.cmp(&a.total_count))),
            "confidence" => pats.sort_by(|a,b| b.confidence.partial_cmp(&a.confidence).unwrap().then_with(|| b.total_count.cmp(&a.total_count))),
            _ => pats.sort_by(|a,b| b.total_count.cmp(&a.total_count).then_with(|| b.frequency.partial_cmp(&a.frequency).unwrap())),
        }
        // Truncate
        if let Some(top) = cli.top { if pats.len() > top { pats.truncate(top); } }
        if let Some(maxp) = cli.max_patterns { if pats.len() > maxp { pats.truncate(maxp); } }
        // Trim subfields
        for p in &mut pats {
            if cli.no_correlations { p.correlations.clear(); }
            if cli.no_temporal { p.temporal = logoscope::ai::TemporalOut::default(); }
            if p.examples.len() > cli.examples { p.examples.truncate(cli.examples); }
        }
        if cli.format == "table" {
            print_patterns_table(&pats, &cli.group_by);
        } else {
            println!("{}", serde_json::to_string_pretty(&pats)?);
        }
        return Ok(());
    }

    // Default: full JSON summary
    println!("{}", serde_json::to_string_pretty(&out)?);
    Ok(())
}

fn run_streaming(interval_secs: u64, window_secs: i64, max_lines: usize, fail_fast: bool) -> anyhow::Result<()> {
    use std::time::{Duration, Instant};
    use std::collections::{VecDeque, HashMap};
    use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
    let running = Arc::new(AtomicBool::new(true));
    {
        let r = running.clone();
        let _ = ctrlc::set_handler(move || { r.store(false, Ordering::SeqCst); });
    }
    let stdin = io::stdin();
    let mut reader = stdin.lock().lines();
    let mut agg = MultiLineAggregator::default();
    let mut buf: VecDeque<(String, Option<DateTime<Utc>>)> = VecDeque::new();
    let mut last_emit = Instant::now();
    let mut last_counts: HashMap<String, usize> = HashMap::new();
    loop {
        if !running.load(Ordering::SeqCst) {
            emit_summary_with_deltas(&buf, &mut last_counts)?;
            break;
        }
        match reader.next() {
            Some(Ok(line)) => {
                if let Some(entry) = agg.push(&line) {
                    let rec = logoscope::parser::parse_line(&entry, buf.len() + 1);
                    if fail_fast {
                        let looks_json = entry.trim_start().starts_with('{') || entry.trim_start().starts_with('[');
                        if looks_json && rec.flat_fields.is_none() && rec.synthetic_message.is_none() {
                            eprintln!("[stream] parse error; aborting due to --fail-fast");
                            break;
                        }
                    }
                    buf.push_back((entry, rec.timestamp));
                    trim_buffer(&mut buf, window_secs, max_lines);
                    if last_emit.elapsed() >= Duration::from_secs(interval_secs) {
                        emit_summary_with_deltas(&buf, &mut last_counts)?;
                        last_emit = Instant::now();
                    }
                }
            }
            Some(Err(_e)) => {
                // ignore read errors
            }
            None => {
                std::thread::sleep(Duration::from_millis(200));
                if last_emit.elapsed() >= Duration::from_secs(interval_secs) {
                    emit_summary_with_deltas(&buf, &mut last_counts)?;
                    last_emit = Instant::now();
                }
            }
        }
    }
    Ok(())
}

fn trim_buffer(buf: &mut std::collections::VecDeque<(String, Option<DateTime<Utc>>)>, window_secs: i64, max_lines: usize) {
    // trim by window using most recent timestamp if available
    let most_recent_ts = buf.iter().rev().find_map(|(_,ts)| *ts).unwrap_or_else(|| Utc::now());
    let cutoff = most_recent_ts - chrono::Duration::seconds(window_secs);
    while let Some((_, ts)) = buf.front() {
        if let Some(t) = ts { if *t < cutoff { buf.pop_front(); continue; } }
        break;
    }
    while buf.len() > max_lines { buf.pop_front(); }
}

fn emit_summary_with_deltas(buf: &std::collections::VecDeque<(String, Option<DateTime<Utc>>)>, last_counts: &mut std::collections::HashMap<String, usize>) -> anyhow::Result<()> {
    let lines: Vec<&str> = buf.iter().map(|(s, _)| s.as_str()).collect();
    let out = logoscope::ai::summarize_lines(&lines);
    // Compact status to stderr
    eprintln!("[stream] lines={} patterns={}", out.summary.total_lines, out.patterns.len());
    // Deltas JSONL on stdout
    let mut new_counts = std::collections::HashMap::new();
    for p in &out.patterns { new_counts.insert(p.template.clone(), p.total_count); }
    for (tpl, cnt) in new_counts.iter() {
        let prev = last_counts.get(tpl).copied().unwrap_or(0);
        if *cnt != prev {
            println!("{}", serde_json::json!({"template": tpl, "delta": (*cnt as i64) - (prev as i64), "total": cnt}));
        }
    }
    *last_counts = new_counts;
    // Full summary after deltas
    println!("{}", serde_json::to_string_pretty(&out)?);
    Ok(())
}

fn print_patterns_table(pats: &Vec<logoscope::ai::PatternOut>, group_by: &str) {
    println!("{:<6} {:<8} {:<8} {:<10} {:<10} {}", "Count", "Freq", "Bursts", "Confidence", "Level", "Template");
    let mut current_group: Option<String> = None;
    for p in pats {
        let group_val = match group_by {
            "level" => p.severity.clone().unwrap_or_else(|| "".into()),
            "service" => p.sources.by_service.get(0).map(|c| c.name.clone()).unwrap_or_else(|| "".into()),
            _ => String::new(),
        };
        if !group_val.is_empty() && current_group.as_deref() != Some(group_val.as_str()) {
            current_group = Some(group_val.clone());
            println!("\n# {}", group_val);
            println!("{:<6} {:<8} {:<8} {:<10} {:<10} {}", "Count", "Freq", "Bursts", "Confidence", "Level", "Template");
        }
        println!("{:<6} {:<8.4} {:<8} {:<10.3} {:<10} {}",
            p.total_count, p.frequency, p.temporal.bursts, p.confidence, p.severity.clone().unwrap_or_else(|| "".into()), p.template);
    }
}
