# Product Requirements Document: AI-Optimized Log Analysis System

## Executive Summary

A high-performance log analysis system that compresses large log files (5MB-1GB+) into structured, AI-digestible summaries while preserving critical debugging information. The system enables AI agents to efficiently debug production issues by providing compressed pattern analysis, temporal insights, and targeted query capabilities to drill into specific problems.

**Core Value Proposition**: Transform millions of log lines into kilobytes of structured intelligence that fits within AI context windows, while maintaining the ability to query back to original logs for detailed analysis.

## Problem Statement

### Current Challenges
1. **Context Window Limitations**: AI agents cannot process multi-megabyte log files directly
2. **Information Overload**: 99% of logs are repetitive; AI agents waste tokens processing duplicate patterns
3. **Temporal Blindness**: Logs without temporal analysis miss critical burst/gap patterns
4. **Unstructured Chaos**: Modern JSON logs mixed with plaintext create parsing complexity
5. **Lost Context**: Schema changes and field evolution often cause production issues but aren't visible in traditional log analysis

### Target Users
- **Primary**: AI debugging agents and automated systems
- **Secondary**: DevOps engineers using AI assistants for production debugging
- **Tertiary**: Direct human operators needing quick log insights

## Core Requirements

### 1. Pattern Extraction & Compression

**Requirement**: Achieve 100-1000x compression while preserving all unique patterns

**Technical Approach**:
- Implement Drain algorithm (fixed-depth parse tree) for O(log n) pattern extraction
- Similarity threshold: 0.4-0.5 (configurable) for clustering
- Maintain pattern templates with masked variables: `User <*> logged in from <IP> at <TIMESTAMP>`

**Key Metrics**:
- Process 1GB logs in <10 seconds
- Memory usage <100MB for 1M unique patterns
- Pattern accuracy >85% (validated against LogPAI benchmarks)

### 2. Multi-Format Log Support

**Requirement**: Handle JSON, plaintext, and mixed-format logs seamlessly

**Technical Approach**:

**JSON Logs** (Hybrid Strategy):
```
Input: {"level":"error","user_id":1234,"op":"login","status":"fail"}
↓
Derived: "level=error op=login user_id=<NUM> status=fail"
↓
Template: "level=error op=login user_id=<*> status=<*>"
```

**Processing Pipeline**:
1. Detect format (JSON vs plaintext) per line
2. For JSON: Flatten nested objects to dot-notation paths
3. Generate synthetic message for pattern extraction
4. Apply masking before Drain processing

**Masking Rules** (Configurable):
- `<NUM>`: Integers and floats
- `<IP>`: IPv4/IPv6 addresses
- `<UUID>`: Standard UUIDs
- `<TIMESTAMP>`: ISO 8601, Unix epoch, common formats
- `<PATH>`: File paths
- `<EMAIL>`: Email addresses
- `<B64>`: Base64 encoded strings
- `<HEX>`: Hex strings
- `<URL>`: URLs

### 3. Schema Evolution Tracking

**Requirement**: Detect structural changes in JSON logs that often correlate with production issues

**Technical Approach**:
- Compute schema fingerprint: sorted set of `(field_path → type)` pairs
- Track schema evolution over time
- Detect:
  - New fields appearing
  - Fields disappearing
  - Type changes (string → int)
  - Cardinality explosions

**Schema Fingerprint Example**:
```
Before: {("user.id","string"), ("status","int")}
After:  {("user.id","string"), ("status","string"), ("retry_count","int")}
Changes: status type changed, retry_count added
```

### 4. Temporal Analysis

**Requirement**: Identify time-based patterns invisible in static analysis

**Technical Components**:

**Timestamp Extraction**:
- Support multiple formats (ISO 8601, Unix epoch, syslog, custom)
- Fallback to line numbers if no timestamp found
- Maintain microsecond precision where available

**Temporal Patterns**:
- **Burst Detection**: Identify periods with >3x median activity
- **Gap Analysis**: Find unusual silences (>10x median gap)

**Time-Based Metrics**:
```
Per Pattern:
- frequency_by_minute: Distribution over time
- burst_periods: [{start_time, end_time, peak_rate, severity}]
- gaps: [{start_time, end_time, duration_seconds}]
```
```

### 5. Statistical Anomaly Detection

**Requirement**: Flag outliers in both patterns and field values

**Pattern Anomalies**:
- New patterns (never seen before)
- Rare patterns (configurable frequency threshold)
- Frequency spikes (z-score or simple multiplier)

### 6. AI Agent Interface

**Requirement**: Provide structured, queryable interface optimized for AI consumption

**Output Format**:
```json
{
  "summary": {
    "total_lines": 500000,
    "unique_patterns": 127,
    "compression_ratio": 3937.0,
    "time_span": "2024-01-15T00:00:00Z to 2024-01-15T23:59:59Z"
  },
  "patterns": [
    {
      "template": "ERROR Database connection failed: <*>",
      "frequency": 0.0234,
      "total_count": 11700,
      "severity": "ERROR",
      "confidence": 0.82,
      "temporal": {
        "bursts": 2,
        "largest_burst": "2024-01-15T14:20:00Z",
        "trend": "increasing"
      },
      "examples": ["ERROR Database connection failed: timeout"],
      "correlations": [
        { "template": "WARN Retry connection <*>", "count": 3200, "strength": 0.41 }
      ],
      "sources": {
        "by_service": [{"name":"auth","count":2}],
        "by_host": [{"name":"host-a","count":1}]
      }
    }
  ],
  "schema_changes": [
    {
      "timestamp": "2024-01-15T14:19:45Z",
      "change_type": "field_added",
      "field": "retry_count",
      "impact": "Correlates with error spike"
    }
  ],
  "anomalies": {
    "pattern_anomalies": [ { "kind": "NewPattern", "template": "<...>", "frequency": 0.0008 } ],
    "field_anomalies": [ "numeric_outlier field=latency_ms value=12000.0 z=7.8 template=..." ],
    "temporal_anomalies": [ "burst template=... start=... end=... peak=..." ]
  },
  "errors": {
    "total": 3,
    "samples": [ {"line_number": 42, "kind": "malformed_json"} ]
  },
  "query_interface": {
    "available_commands": ["GET_LINES_BY_PATTERN", "GET_LINES_BY_TIME", "GET_CONTEXT"],
    "suggested_investigations": [
      {
        "priority": "HIGH",
        "description": "Pattern burst",
        "query": { "command": "GET_LINES_BY_TIME", "params": { "start": "...", "end": "...", "pattern": "..." } }
      }
    ]
  }
}
```

**Query Capabilities**:
- Retrieve original logs by pattern
- Time-range queries with optional pattern filter
- Context retrieval (N lines before/after)

**Context Window Management**:
- Token counting to honor a budget
- Priority-based truncation (critical > summaries > examples)

### 7. Streaming (No Persistence)

Note: Persistence is currently removed per updated priorities. Streaming remains planned as in-memory processing without state checkpoints.

**Streaming Mode (implemented, in-memory)**:
- `--follow` reads from stdin continuously and consolidates multi-line entries.
- Bounded memory via rolling window (`--window` seconds) and `--max-lines` cap.
- Periodic summaries every `--interval` seconds printed as JSON.
- Timestamp hints and masking applied; patterns/anomalies updated over time.
- Emits compact status to stderr and JSONL deltas of pattern count changes to stdout between summaries.

### 8. Multi-Line Log Support (High Priority)

**Requirement**: Consolidate multi-line entries (stack traces, SQL queries) into single logical events.

**Technical Approach (Implemented MVP)**:
- Stack trace consolidation: heuristic continuation when lines start with whitespace, `at `, `Caused by:`, or `... N more`.
- Multi-line JSON detection and recovery using bracket-balance accumulation (`{}`/`[]`).
- Consolidation applied in CLI `analyze` input pipeline before analysis.
- Configurable patterns and max-lines per entry are planned (MVP uses built-in heuristics).

### 9. Error Recovery & Partial Processing (High Priority)

**Requirement**: Continue analysis despite malformed or corrupted input while providing precise diagnostics.

**Technical Approach**:
- Graceful handling of malformed JSON lines (best-effort parse, quarantine bucket)
- Recovery from mid-file corruption with resynchronization heuristics
- Partial results emission when inputs are incomplete
- Line number tracking with file offsets for error reporting
- Configurable error tolerance (skip vs fail-fast), with per-source policy

### 10. CLI Interface Specifications (High Priority)

**Command (single)**:
- `logoscope [FILES...]` — Default: full JSON summary.
- `--only patterns` — Emit only patterns with filters: `--top`, `--min-count`, `--min-frequency`, `--match`, `--exclude`, `--level`, `--examples`, `--no-correlations`, `--no-temporal`, `--max-patterns`.
- `--only logs` — Emit raw lines filtered by `--start`, `--end`, `--pattern`, `--service`, `--host` with `--before/--after` context.
 - Patterns view options: `--format table` for compact terminal output, `--group-by level|service`, `--sort count|freq|bursts|confidence`.

**Global Flags**:
- `--config PATH`, `--format json|markdown|html`
- `--start-time`, `--end-time`, `--pattern-id`, `--service`, `--min-confidence`
- `--input PATH|'-'` (stdin), `--output PATH`, `--follow`, `--tail N`
- `--verbose`, `--quiet`, `--color auto|always|never`
- `--time-key KEY` (repeatable): JSON timestamp field hints in priority order
- `--service NAME`, `--host NAME` filters (patterns/logs views)

**Exit Codes**:
- `0` success; `1` unexpected error; `2` anomalies exceed threshold; `3` input/parse error
- `4` state corruption detected; `5` incompatible state/version

**Streaming Mode**:
- Reads NDJSON/plaintext from stdin with backpressure-aware processing
- Periodic incremental summaries to stderr; machine-readable deltas to stdout (JSONL)
- Checkpoint on signals (SIGINT/SIGTERM) for safe resume

**Examples**:
- `kubectl logs -n prod deploy/api --since=1h | logoscope tail --format json`
- `logoscope analyze --input /var/log/app.log --format json > summary.json`
- `logoscope logs --pattern "User <*> logged in <*>" --before 2 --after 2 file1.log file2.log`
- `logoscope patterns --min-confidence 0.8 | head`
- `logoscope logs --pattern-id p_123 --start-time 2024-01-15T14:00:00Z --end-time 2024-01-15T15:00:00Z --context-before 3 --context-after 3`
- `logoscope query --level ERROR --since 1h --group-by pattern --agg count --limit 20 --format markdown`

**Platform Integrations**:
- Kubernetes/Docker log format support; extract pod/namespace/container where present
- CloudWatch/Stackdriver export compatibility (file/STDIN inputs)
- Prometheus/Grafana annotations/metrics emitted via CLI (`metrics` subcommand)
- OpenTelemetry context propagation from fields (trace_id/span_id) for correlation

This is a CLI-only tool; no network APIs are provided. Stateful options (checkpoints/state path) are deferred.

### 11. Basic Pattern Confidence (High Priority)

**Requirement**: Provide a single confidence signal for each pattern.

**Status (MVP)**:
- Confidence computed per pattern with presence over time (minutes with events across span) and frequency factor; 0..1 scale.

### 12. Log Source Attribution (Medium Priority)

**Requirement**: Track origin of each log for distributed debugging and correlation.

**Technical Approach**:
- Hostname/container ID extraction and normalization
- Service mesh metadata (pod, namespace, node)
- File source tracking during multi-file processing
- Preservation of log shipper metadata
- Correlation IDs across services and hops

### 13. Minimal Querying (Medium Priority)

**Requirement**: Provide simple, flag-based filtering and aggregation.

**Capabilities**:
- Filter by time range, level, service, pattern id/template
- Aggregations: count, top-k by pattern/field
- Output as JSON/CSV for piping (e.g., jq)

### 14. Incremental Processing & Caching (Medium Priority)

**Requirement**: Optimize for repeated and continuous analyses at scale.

**Strategy**:
- Incremental markers (last processed position / offsets)
- Pattern cache with TTL and warm starts
- Deduplication of repeated log blocks
- Delta compression between runs
- Safe resumption after interruptions

### 15. Pattern Lifecycle (Defer)

Out of MVP scope. Future: versioning, merge, retire, import/export.

### 16. Testing Strategy (Medium Priority)

Focus on correctness and regressions for MVP.

**Tests**:
- Golden tests for masking and pattern extraction
- Multi-line consolidation cases (stacks, SQL, max-lines)
- Error recovery (malformed JSON, mid-file corruption)
- Basic performance smoke (throughput on sample corpus)

### 17. Distributed Processing (Defer)

Out of MVP scope. Future: sharding, distributed state, consensus.

### 18. Internationalization (Defer)

MVP: UTF-8 throughout, common timestamp formats.

### 19. Compliance (Defer)

MVP: Redaction/masking only. Broader compliance deferred.

### 20. Feedback Loops (Defer)

Out of MVP scope.

## Technical Architecture

### Core Components

```
┌─────────────────────────────────────────────┐
│                 Input Layer                  │
│  (JSON / Plaintext / Compressed / Stream)   │
└─────────────────┬───────────────────────────┘
                  │
┌─────────────────▼───────────────────────────┐
│            Format Detection                  │
│         & Timestamp Extraction               │
└─────────────────┬───────────────────────────┘
                  │
        ┌─────────┴─────────┬─────────────┐
        ▼                   ▼             ▼
┌───────────────┐  ┌────────────┐  ┌────────────┐
│Schema Tracker │  │   Masking  │  │  Temporal  │
│  (JSON only)  │  │   Engine   │  │  Analyzer  │
└───────┬───────┘  └─────┬──────┘  └─────┬──────┘
        │                 │                │
        └─────────┬───────┴────────────────┘
                  │
┌─────────────────▼───────────────────────────┐
│          Drain Pattern Extraction           │
│         (Fixed-depth Parse Tree)            │
└─────────────────┬───────────────────────────┘
                  │
┌─────────────────▼───────────────────────────┐
│          Statistical Analysis               │
│   (Outliers, Distributions, Correlations)   │
└─────────────────┬───────────────────────────┘
                  │
┌─────────────────▼───────────────────────────┐
│            AI Interface Layer               │
│           (Compression, Queries)            │
└─────────────────┬───────────────────────────┘
                  │
┌─────────────────▼───────────────────────────┐
│              Output Formats                 │
│     (CLI / JSON / Streaming / HTML)         │
└─────────────────────────────────────────────┘
```

### Data Flow

1. **Ingestion**: Read logs line-by-line (streaming-capable)
2. **Parsing**: Extract timestamp, detect format, parse structure
3. **Masking**: Apply configurable maskers to create stable patterns
4. **Pattern Extraction**: Feed to Drain for template mining
5. **Analysis**: Compute statistics, detect anomalies, find correlations
6. **Compression**: Generate AI-optimized summary
7. **Query Interface**: Provide drill-down capabilities

## Implementation Roadmap

### MVP Roadmap (Weeks 1–6)
- M1: Project scaffold; JSON/plaintext detection; masking engine
- M2: Schema fingerprinting (JSON); timestamp extraction
- M3: Drain integration; basic pattern mining
- M4: Multi-line consolidation (generic continuation + caps)
- M5: Error handling (line numbers, offsets; skip/fail); checkpoints/resume
- M6: Temporal bursts/gaps; basic anomalies (new/rare/spikes)
- M7: Basic pattern confidence (stability + similarity)
- M8: CLI subcommands and JSON output; context window budgeting
- M9: Minimal querying (flag-based filters + counts/top-k)

### Post-MVP (Later)
- Incremental processing polish; warm starts
- CLI polish (completions, man pages)
- Pattern lifecycle basics (export/import)
- Extended testing/benchmarks and cost instrumentation

## Success Metrics

### Performance (Report + Iterate)
- Report throughput (MB/s) on sample corpora; track over time
- Report memory usage per unique pattern; track over time
- Compression ratio (lines→patterns) per dataset

### Correctness
- Pattern extraction accuracy vs. golden fixtures
- Multi-line consolidation correctness on representative stacks
- Schema evolution detection accuracy on crafted changes

### UX
- Output fits within configured token budget
- CLI commands succeed with actionable exit codes and messages

## Resource Reporting (MVP)

- Report CPU time, throughput, and memory snapshots
- Track state size growth and compaction ratios

## Operational Monitoring

- Processing latency and throughput counters
- Memory usage and state size snapshots
- Compression ratio trends
- CLI command latency and error rates

## Disaster Recovery

- State snapshots (periodic/event-driven)
- Checkpoint/offset alignment for resume
- Corruption detection (checksums on state)

## Version Compatibility

- Versioned state format; fail with clear upgrade message if incompatible
- Versioned JSON output schema; minimal deprecations

## Recommended Additions Priority

**High Priority** (Add immediately):
1. Multi-line log support — Implemented (MVP heuristics)
2. Error recovery strategy — Implemented (malformed JSON quarantine + partial processing)
3. CLI interface specifications — Implemented (single-command with focused views)
4. Basic pattern confidence — Implemented (presence/frequency-based score per pattern)

**Medium Priority** (Add for v1.0):
1. Log source attribution
2. Minimal querying (flags + aggregations)
3. Incremental processing
4. Testing strategy

**Low Priority** (Consider for v2.0):
1. Distributed processing
2. i18n support
3. Compliance features
4. Feedback loops

## Configuration Management

### Sample Configuration (TOML)

```toml
[general]
mode = "batch"  # or "streaming"
state_path = "/var/lib/logscope/state"

[formats]
detect_json = true
timestamp_formats = [
    "%Y-%m-%dT%H:%M:%S%.fZ",
    "%b %d %H:%M:%S",
    "unix_epoch"
]

[fields]
# JSON field mappings
timestamp = "time"
level = "level"
service = "service"
message = "message"

[masking]
enabled = true
maskers = ["uuid", "ip", "num", "float", "hex", "timestamp", "email", "path", "b64", "url"]
drop_keys = ["request_id", "trace_id", "span_id"]  # High cardinality, low value

[drain]
similarity_threshold = 0.4
max_depth = 4
max_children = 100
max_clusters = 10000

[analysis]
anomaly_threshold = 0.001  # <0.1% frequency
burst_multiplier = 3.0      # 3x median rate
gap_multiplier = 10.0       # 10x median gap
correlation_window = 10     # seconds

[output]
format = "json"  # or "markdown", "html"
max_examples = 3
max_patterns = 100
```

## Security & Privacy Considerations

1. **PII Handling**: Masking must redact sensitive data
2. **Data Retention**: Configurable cleanup of old state
3. **Encrypted Storage**: Option for encrypted state persistence
4. **Local Security**: Respect filesystem permissions for state/logs

## Extensibility Points (MVP)

1. **Custom Maskers**: Pluggable masking rules for domain-specific patterns

Future (defer): analysis plugins, alternate storage backends, alert integrations

## Dependencies & Technology Stack

## Development Progress Log

### 2025-09-05
- Core parsing + masking + schema fingerprinting implemented (JSON/plaintext).
- Temporal analytics: bursts, gaps, spikes; query interface (pattern/time/context).
- Pattern clustering (simple + drain-rs adapter) and pattern anomalies (new/rare).
- AI output enrichment: patterns with severity/temporal/examples, schema_changes, anomalies, suggestions, query interface.
- Correlations: integrated top related patterns per template (10s window, Jaccard-like strength).
- Masking extended: added `<UUID>`, `<PATH>`, `<URL>`, `<HEX>`, `<B64>` with tests.
- Anomaly output enriched: includes field anomalies (median/MAD outliers, cardinality explosions) and temporal anomalies (bursts/gaps).
- CLI: Added `logoscope analyze` supporting multiple input files and stdin, producing enriched JSON summary. Multi-file analysis aggregates timestamps across files for proper time correlation.
- Timestamp detection hints:
  - Parser now supports prioritized timestamp field hints for JSON (e.g., `ts`, `timestamp`, `time`), falling back to auto-detection across all fields if none match.
  - CLI flag `--time-key` accepts multiple hints (in priority order).

### Core Technologies
- **Language**: Rust (performance-focused CLI)
- **Pattern Mining**: drain-rs or drainrs (Rust implementations)
- **JSON Parsing**: serde_json with streaming support
- **Regex Engine**: regex crate with pre-compilation
- **Statistical**: Statistical crate for robust statistics
- **Persistence**: CBOR or MessagePack for efficient serialization

### Future Considerations (Defer)
- Explore bindings or alternative runtimes only if needed post-MVP

## Risk Mitigation

### Technical Risks
1. **Pattern Explosion**: LRU eviction and configurable limits
2. **Memory Overflow**: Streaming processing with bounded buffers
3. **Performance Degradation**: Lazy evaluation and index structures
4. **Format Changes**: Extensible parser architecture

### Operational Risks
1. **State Corruption**: Checksums and backup strategies
2. **Version Migration**: Versioned state format with migration tools
3. **Resource Exhaustion**: Circuit breakers and rate limiting

## Conclusion

This system bridges the gap between massive log volumes and AI agent context limitations, enabling efficient automated debugging. By combining Drain's proven pattern extraction with temporal analysis, schema tracking, and intelligent compression, we create a tool that makes logs truly AI-accessible while preserving the ability to drill into specific issues when needed.

The hybrid approach—maintaining both structure awareness (for JSON) and pattern mining (via Drain)—ensures we catch both content anomalies and structural changes that often correlate with production issues. The result is a system that transforms overwhelming log data into actionable intelligence that fits comfortably within AI context windows.

## Development Progress Log

### 2025-09-05
- Completed: Project scaffold (Rust library `logoscope`).
  - Added Cargo configuration with dependencies: `serde`, `serde_json`, `regex`, `chrono`, `once_cell`, `thiserror`, `itertools`.
- Completed: Base parsing (Milestone M1 scope)
  - Implemented `parser::parse_line` with format detection (JSON vs plaintext).
  - JSON handling: flatten nested objects to dot-paths using stable ordering (BTreeMap).
  - Timestamp extraction: parse ISO-8601/RFC3339 from `time` field; fallback support for epoch seconds.
  - Synthetic message generation for JSON: stable `key=value` concatenation sorted by key.
  - Tests: Added `tests/parser_tests.rs` covering plaintext detection and JSON flattening + timestamp + synthetic message ordering. All tests passing (`cargo test`).
- Completed: Masking engine (Milestone M3, prioritized ahead of M2 per updated priorities)
  - Implemented `masking::mask_text` with ordered replacements: `<TIMESTAMP>`, `<IP>` (IPv6+IPv4), `<EMAIL>`, `<NUM>` (float+int).
  - Tests: Added `tests/masking_tests.rs` validating plaintext masking and masking of synthetic JSON messages. All tests passing.
- Next: Schema fingerprinting for JSON (Milestone M2)
  - Plan: TDD for computing `(field_path -> type)` fingerprints and diffing (added/removed/type-changed).

### 2025-09-05 (cont.)
- Completed: Schema fingerprinting for JSON (Milestone M2)
  - Implemented `schema::{fingerprint_line, diff_fingerprints}` with field typing (`string`, `int`, `float`, `bool`, `null`, arrays indexed by position).
  - Tests: `tests/schema_tests.rs` cover basic types and detection of added/type-changed fields.
- Completed: Temporal extraction for plaintext (Milestone M5 - timestamp extraction)
  - Parser now extracts timestamps from plaintext lines (ISO-8601, syslog `%b %d %H:%M:%S`, and 10-digit Unix epoch).
  - Tests: Added to `tests/parser_tests.rs` validating syslog timestamp extraction using current year context.
- Next: Pattern extraction (Milestone M4)
  - Plan: TDD to cluster masked messages via Drain; integrate `drain-rs` behind an adapter.

### 2025-09-05 (cont.)
- Completed: Simple pattern clustering and Drain integration (Milestone M4)
  - Implemented `patterns::cluster_masked` to group masked lines by generic templates (`<*>`).
  - Integrated `drain-rs` via `drain_adapter::DrainAdapter` using `DrainTree::add_log_line`/`log_groups` and mapping to generic templates.
  - Tests: `tests/patterns_tests.rs` (adapter-free) and `tests/drain_adapter_tests.rs` (drain-backed) both green.
- Next: Basic anomaly detection (Milestone M6)
  - Plan: TDD for pattern anomalies (new patterns vs baseline, rare patterns by frequency threshold). Frequency spikes to follow with temporal buckets.

### 2025-09-05 (cont.)
- Completed: Pattern anomaly detection (Milestone M6 - partial)
  - Implemented `anomaly::detect_pattern_anomalies` to flag `NewPattern` and `RarePattern` anomalies based on baseline and frequency threshold.
  - Tests: `tests/anomaly_tests.rs` ensures correct detection.
- Completed: Temporal burst detection (Milestone M8 - bursts only)
  - Implemented `temporal::compute_bursts` using per-minute bucketing and median-based thresholding.
  - Tests: `tests/temporal_tests.rs` verifies detection of a burst period and severity.
- Next: AI output summary (Milestone M11 - skeleton)
  - Plan: Produce compact JSON-able summary (total_lines, unique_patterns, compression_ratio, time_span) as a first integration step.

### 2025-09-05 (cont.)
- Completed: AI-optimized output summary (Milestone M11 - skeleton)
  - Implemented `ai::summarize_lines` to compute `summary` with `total_lines`, `unique_patterns`, `compression_ratio`, and `time_span` from earliest to latest timestamp.
  - Integrates parser + masking + drain clustering to derive patterns count.
  - Tests: `tests/ai_summary_tests.rs` confirms summary fields.

Pending next steps (planned):
- Completed: Temporal gap detection and frequency spikes (Milestone M8 - gaps/spikes)
  - Implemented `temporal::compute_gaps` using inter-arrival median and multiplier.
  - Implemented `temporal::compute_spikes` using per-bucket counts and z-score threshold.
  - Tests: Added to `tests/temporal_tests.rs` for gaps and spikes.
- Field anomalies: robust stats per-pattern (median/MAD) and categorical cardinality explosions.
- Query interface: basic commands (`GET_LINES_BY_PATTERN`, `GET_LINES_BY_TIME`, `GET_CONTEXT`).
- Persistence: serialize drain state and schema baselines.

### 2025-09-05 (cont.)
- Completed: Query interface (Milestone M10 - basic)
  - Implemented in-memory `query::QueryIndex` with:
    - `get_lines_by_pattern(template)` — equality on generic `<*>` templates.
    - `get_lines_by_time(start, end, template)` — half-open [start, end) with optional pattern filter.
    - `get_context(id, before, after)` — returns neighboring lines by insertion order.
  - Heuristic for plaintext: strip syslog/app prefix up to last `": "` before masking, to focus on content.
  - Tests: `tests/query_tests.rs` validates all three commands.

Next up (planned):
- Persistence (M13): serialize Drain tree + schema baselines; periodic checkpoints.
- Field anomalies (M7): per-pattern numeric baseline (median/MAD), categorical cardinality explosions.
- AI output enrichment (M11/M12): include patterns, schema_changes, anomalies, suggestions.

### 2025-09-05 (cont.)
- Change: Persistence removed (Updated priorities)
  - Removed persistence features and tests. No on-disk state is kept.
  - Deleted `persistence` module and CBOR dependencies; all analysis remains in-memory.

### 2025-09-05 (cont.)
- Completed: Field-level statistical analysis (Milestone M7 - partial)
  - Implemented `field_anomaly` module:
    - Numeric outliers using robust statistics (median + MAD with 0.6745 scale) per pattern and field.
    - Categorical cardinality explosions using unique/total ratio threshold with min sample size.
  - Tests: `tests/field_anomaly_tests.rs` validates numeric outlier and cardinality explosion detection.
- Next: AI output enrichment (Milestones M11/M12)
  - Plan: extend `ai::summarize_lines` to include patterns block, schema_changes, and anomalies (pattern/field/temporal) with compact summaries and suggestions.

### 2025-09-05 (cont.)
- Completed: AI output enrichment (Milestones M11/M12 - initial)
  - `ai::summarize_lines` now returns enriched `AiOutput` including:
    - `summary`: total_lines, unique_patterns, compression_ratio, time_span.
    - `patterns`: [{ template, frequency, total_count, severity (mode of levels), temporal { bursts count, largest_burst }, examples }].
    - `schema_changes`: derived by diffing first vs last JSON fingerprints (type changes, field add/remove; timestamp set to last JSON ts when available).
    - `anomalies.pattern_anomalies`: integrates New/Rare pattern detection (threshold 0.1 by default).
  - Tests: `tests/ai_enriched_tests.rs` verifies patterns content and schema changes detection.
- Next: Suggestions generation (M12)
  - Plan: derive suggested investigations from bursts/spikes, schema changes near bursts, and rare/new patterns.

### 2025-09-05 (cont.)
- Completed: Suggestions generation (Milestone M12)
  - `AiOutput` now includes `query_interface` with:
    - `available_commands`: [GET_LINES_BY_PATTERN, GET_LINES_BY_TIME, GET_CONTEXT]
    - `suggested_investigations`: derived from:
      - Largest burst per pattern → HIGH priority GET_LINES_BY_TIME with start/end and pattern.
      - Schema changes (field added/removed/type changed) → MEDIUM priority GET_LINES_BY_TIME ±5 minutes around change.
      - Pattern anomalies (NewPattern → HIGH, RarePattern → LOW) → GET_LINES_BY_PATTERN.
  - Tests: `tests/ai_suggestions_tests.rs` asserts presence of time-range suggestion for burst case.

Remaining roadmap highlights:
- Persist rolling statistics and learned baselines across runs (extend M13).
- Completed: Pattern correlation analysis (Milestone M9 - basic)
  - Implemented `correlation::compute_correlations` to compute pairwise co-occurrence counts within a time window and Jaccard-like strength.
  - Tests: `tests/correlation_tests.rs` validates strong A↔B correlation and weak A↔C.
- Integrated: Correlations in AI output
  - `patterns[].correlations`: top related templates with `{template, count, strength}` using a 10s window.
  - Test: `tests/ai_correlation_integration_tests.rs` checks that correlated patterns are present for co-occurring templates.
- CLI/API adapters (M18) for batch and streaming modes.
