Logoscope — AI‑Optimized Log Analysis
=====================================

Turn gigabytes of logs into kilobytes of structured intelligence that fit comfortably within AI context windows — without losing the ability to drill down. Logoscope mines stable patterns, tracks schema changes, detects temporal bursts/gaps, flags anomalies, and gives you fast, focused queries and summaries.

Progressively discover the signal in your logs: start with a one‑line quick start, then dig into patterns, streaming, and filters as you need them.


Highlights
----------

- Pattern extraction (Drain) with typed masking → generic `<*>` templates
- Multi‑format support: JSON + plaintext (auto-detect per line)
- Multi‑line consolidation: stack traces + bracket‑balanced JSON
- Timestamp autodetect: ISO8601 (Z/offset), syslog, epoch (sec/ms/µs)
- Temporal insights: bursts, gaps, spikes; cross‑file time correlation
- Schema tracking: fingerprint and diff (add/remove/type‑change)
- Anomalies: new/rare patterns, numeric outliers (median/MAD), cardinality explosions
- Correlations: patterns that co‑occur in short time windows
- Source attribution: per‑pattern top services and hosts
- Streaming mode (in‑memory): periodic summaries + JSONL deltas


Quick Start
-----------

1) Download a release (recommended) or build from source.

- Releases: download the correct binary from GitHub and place it on your PATH as `logoscope`.
- Build (Rust 1.75+):
```
cargo build --release
# binary at target/release/logoscope
```

2) Analyze logs
```
logoscope app1.log app2.log > summary.json
```
Outputs a compact JSON summary with patterns, temporal insights, schema changes, anomalies, correlations, suggestions, and errors (if any).

3) Patterns‑only view (table)
```
logoscope --only patterns --format table --group-by level --sort confidence logs/*.log
```

4) Streaming summaries (follow stdin)
```
tail -F /var/log/app.log | logoscope --follow --interval 5 --window 600
```
Emits compact status to stderr, pattern count deltas (JSONL) and full periodic summaries to stdout.


NPM Install (optional)
----------------------

Prefer using from Node/JS? Install our wrapper — it auto‑downloads a platform binary during `postinstall` and exposes a `logoscope` command.
```
npm i -g @logoscope/cli
```
Environment overrides (if you host releases elsewhere):
- LOGOSCOPE_REPO_OWNER (default: your-org)
- LOGOSCOPE_REPO_NAME (default: logoscope)


Core Usage Patterns
-------------------

- Patterns table (top N, with examples capped):
```
logoscope --only patterns --top 20 --examples 2 --format table logs/*.log
```
- Patterns JSON, focused on DB issues and ERRORs:
```
logoscope --only patterns --match 'DB|database' --level error logs/*.log
```
- Logs slice (time window + context):
```
logoscope --only logs --start 2024-01-01T00:00:00Z                      --end   2024-01-01T01:00:00Z                      --before 3 --after 3 logs/*.log
```
- Logs filtered by template and source:
```
logoscope --only logs --pattern "User <*> logged in <*>" --service auth --host h1 app.log
```
- Timestamp hints for JSON (prioritized keys):
```
logoscope --time-key ts --time-key time logs.ndjson
```


Multi‑file & Multi‑line
-----------------------

- Pass multiple files; timestamps are normalized to UTC and correlated across sources.
- Multi‑line stack traces and multi‑line JSON are consolidated before analysis.


Output (compact overview)
-------------------------

The summary is a single JSON document:
- summary: total_lines, unique_patterns, compression_ratio, time_span
- patterns[]:
  - template, total_count, frequency, severity, confidence
  - temporal: bursts, largest_burst, trend
  - correlations[]: { template, count, strength }
  - sources: by_service[], by_host[] (top contributors)
  - examples[]: up to N example lines (configurable)
- schema_changes[]: { timestamp, change_type, field, impact }
- anomalies: pattern_anomalies, field_anomalies, temporal_anomalies
- errors: { total, samples[] } with line numbers and kinds (e.g., malformed_json)
- query_interface: available commands + suggested investigations


Security & Masking
------------------

PII and high-cardinality items are masked before clustering:
- <NUM>, <IP>, <EMAIL>, <TIMESTAMP>, <UUID>, <PATH>, <URL>, <HEX>, <B64>
Templates drop common source keys (e.g., service, host, kubernetes.*) to avoid pattern explosion.


Performance
-----------

- Designed to scale: multi‑file input, per‑line streaming, fixed‑depth Drain tree
- Streaming mode uses rolling windows with bounded memory
- JSON flattening and maskers are fast; burst/gap detection is bucketized


CLI Reference
-------------

Global
- logoscope [FILES...]:
  - Analyze files (or - for stdin). Consolidates multi‑line entries.
- --time-key KEY:
  - Prioritize JSON timestamp fields (repeatable); falls back to autodetection.
- --only patterns | logs | summary:
  - Show a focused view; default is full JSON summary.

Patterns view (--only patterns)
- --top N, --min-count N, --min-frequency F
- --match REGEX, --exclude REGEX, --level LEVEL
- --examples N (default 3)
- --no-correlations, --no-temporal, --max-patterns N
- --format json|table (default json)
- --group-by none|service|level (table)
- --sort count|freq|bursts|confidence (desc)

Logs view (--only logs)
- --start RFC3339, --end RFC3339
- --pattern TEMPLATE (generic <*>)
- --service NAME, --host NAME
- --before N, --after N (context around first match)

Streaming (stdin)
- --follow (enable)
- --interval SEC (summary cadence; default 5)
- --window SEC (rolling window; default 300)
- --max-lines N (cap consolidated entries; default 10000)
- --fail-fast (abort stream on parse error)


Building from Source
--------------------
```
cargo build --release
# binary: target/release/logoscope
```


Roadmap (Next)
--------------

- Patterns UX: richer table, column selection, export formats
- Streaming: delta-only mode, compact stderr summaries, backpressure hints
- Query aggregations: top‑k per service/level/window
- Source attribution: deeper Kubernetes metadata & filters
- Configurable maskers and drop‑keys


Contributing
------------

Issues and PRs welcome. Please keep changes focused and covered by tests. For feature work, open an issue to discuss scope and UX first.


License
-------

MIT
