# Logoscope

**Gigabytes of logs → kilobytes of AI-ready insights.**  
Pattern and anomaly log extraction for AI and humans.

Turn millions of log lines into structured intelligence that fits in AI context windows. Every spike, drift, and anomaly — instantly queryable.

## Quick Start

### AI Integration (Claude Desktop)

```bash
# Add to Claude Desktop
claude mcp add logoscope -- npx -y @probelabs/logoscope@latest mcp

# Then ask Claude:
"Analyze these logs: /var/log/app.log"
```

Claude will automatically use fast triage analysis first, then deep analysis if needed.

### CLI

```bash
# Step 1: Quick triage (fast anomaly detection)
npx -y @probelabs/logoscope@latest --triage logs/app-*.log

# Step 2: Full analysis (if anomalies found)
npx -y @probelabs/logoscope@latest logs/app-*.log --out insights.json

# Streaming logs
kubectl logs -n prod deploy/web --since=30m | npx -y @probelabs/logoscope@latest --triage
journalctl -u api -o json | npx -y @probelabs/logoscope@latest --out insights.json
```

## The Problem

Production is down. Your logs are **gigabytes of noise**: thousands of identical lines, buried anomalies, and patterns too complex for grep. Uploading to AI fails — context windows overflow, tokens waste on repetition, and temporal patterns vanish in copy-paste.

## How It Works

Logoscope uses the **Drain algorithm** to extract log templates, replacing variables with wildcards. We identify **anomalies** (new patterns, frequency spikes, numeric outliers), track **temporal dynamics** (bursts, gaps, trends), and compress millions of lines into **kilobytes of structured JSON**.

## What You Get

- **Pattern extraction** with severity levels and trend analysis
- **Temporal anomalies** with exact timestamps and burst detection
- **Schema tracking** for field additions, removals, and type changes
- **Parameter statistics** with cardinality analysis and value distributions
- **AI-ready JSON** that fits in context windows
- **Queryable access** to raw logs with context

## Installation

### NPM (Recommended)

```bash
npm i -g @probelabs/logoscope
```

The npm package auto-downloads the platform-specific binary during installation.

### Direct Binary

Download from [GitHub Releases](https://github.com/probelabs/logoscope/releases) and add to PATH.

### Build from Source

```bash
cargo build --release
# Binary at target/release/logoscope
```

## Core Usage

### Two-Step Workflow

Always start with `--triage` for rapid anomaly detection, then run full analysis if needed:

```bash
# Step 1: Quick triage (seconds)
logoscope --triage logs/*.log

# Step 2: Full analysis (if anomalies found)
logoscope logs/*.log --out analysis.json
```

### Time Window Analysis

Extract logs from specific time periods when investigating incidents:

```bash
# Analyze last hour's logs
logoscope --triage \
  --start 2024-01-15T14:00:00Z \
  --end 2024-01-15T15:00:00Z \
  logs/*.log

# Get logs around an incident with context
logoscope --only logs \
  --start 2024-01-15T14:19:00Z \
  --end 2024-01-15T14:25:00Z \
  --before 5 --after 5 \
  logs/*.log
```

### Streaming Analysis

Monitor logs in real-time:

```bash
# Stream with triage mode
tail -F /var/log/app.log | logoscope --follow --triage

# Kubernetes logs
kubectl logs -f deployment/api | logoscope --follow --triage
```

## Output Format

Logoscope produces structured JSON optimized for both AI consumption and human analysis:

```json
{
  "summary": {
    "total_lines": 523412,
    "unique_patterns": 142,
    "compression_ratio": 3686.7,
    "start_date": "2024-01-15T00:00:00Z",
    "end_date": "2024-01-15T23:59:59Z"
  },
  "patterns": [{
    "template": "ERROR Database connection failed: <*>",
    "total_count": 11700,
    "frequency": 0.0234,
    "severity": "ERROR",
    "pattern_stability": 0.82,
    "temporal": {
      "bursts": 2,
      "largest_burst": "2024-01-15T14:20:00Z",
      "trend": "increasing"
    },
    "param_stats": {
      "ERROR_CODE": {
        "cardinality": 3,
        "values": [
          { "value": "timeout", "count": 8000 },
          { "value": "refused", "count": 3200 }
        ]
      }
    }
  }],
  "anomalies": {
    "temporal_anomalies": [{
      "type": "FrequencySpike",
      "template": "ERROR Database connection failed: <*>",
      "at": "2024-01-15T14:20:00Z",
      "value": 127,
      "baseline": 18
    }],
    "pattern_anomalies": [{
      "type": "NewPattern",
      "template": "WARN retry=<*> exceeded for op=<*>",
      "first_seen": "2024-01-15T14:19:45Z"
    }]
  }
}
```

## Real Questions, Real Answers

Ask questions that matter:

- **"What spiked at 14:20?"** — Get exact patterns with frequency analysis
- **"Show new error patterns from today"** — Identify newly emerged issues
- **"Which fields changed before the outage?"** — Track schema evolution
- **"Extract payment failures with context"** — Query specific patterns with surrounding lines

## Advanced Usage

### Multi-Format Support

- **Auto-detection**: JSON and plaintext per line
- **Multi-line**: Stack traces and bracket-balanced JSON
- **Timestamps**: ISO8601, syslog, epoch (auto-detected)

### Performance

- **Fast**: ~3 seconds for 100k log lines
- **Scalable**: Fixed-depth Drain tree, bounded memory
- **Parallel**: Multi-threaded pattern extraction
- **Streaming**: Rolling windows for continuous analysis

### Security & Masking

Built-in PII protection with smart masking:
- `<NUM>`, `<IP>`, `<EMAIL>`, `<UUID>`, `<PATH>`, `<URL>`, `<HEX>`, `<B64>`
- Preserves structure while protecting sensitive data

## MCP Server Configuration

For AI assistants and editors, configure the MCP server:

```json
{
  "mcpServers": {
    "logoscope": {
      "type": "stdio",
      "command": "npx",
      "args": ["-y", "@probelabs/logoscope@latest", "mcp"]
    }
  }
}
```

Available MCP tools:
- `log_anomalies`: Quick triage analysis
- `full_log_analysis`: Comprehensive analysis
- `patterns_table`: Pattern-focused view
- `logs_slice`: Time-windowed log extraction

## Examples

### Database Outage Diagnosis

```bash
# 1. Quick triage to find issues
logoscope --triage prod-*.log

# 2. Focus on database errors
logoscope --only patterns --match "database|timeout" --level error prod-*.log

# 3. Extract specific time window with context
logoscope --only logs \
  --start 2024-01-15T14:15:00Z \
  --end 2024-01-15T14:30:00Z \
  --pattern "ERROR Database connection failed: <*>" \
  --before 2 --after 2 \
  prod-*.log
```

### Performance Regression Detection

```bash
# Find latency outliers
logoscope api-*.log | jq '.anomalies.field_anomalies[] | select(contains("latency"))'

# Extract slow requests
logoscope --only logs --pattern "*latency_ms=<*>*" --min-value 5000 api-*.log
```

### Continuous Monitoring

```bash
# Stream with anomaly detection
tail -F /var/log/app.log | logoscope --follow --triage --interval 10

# Watch for specific patterns
tail -F app.log | logoscope --follow --match "ERROR|CRITICAL" --alert-threshold 10
```

## Contributing

We welcome contributions! Check out our [contributing guide](CONTRIBUTING.md) for details.

## License

MIT

---

Built by [Probelabs](https://probelabs.com) — for debugging at scale.