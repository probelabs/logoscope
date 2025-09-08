# Logoscope Usage Examples

## Quick Start - Basic Analysis

```bash
# Analyze a single log file
logoscope /var/log/app.log > analysis.json

# Analyze multiple log files
logoscope app1.log app2.log app3.log > combined_analysis.json

# Analyze with specific timestamp field for JSON logs
logoscope --time-key timestamp --time-key ts logs.ndjson
```

## Real-World Scenarios

### 🚨 Scenario 1: Production Incident - Quick Triage

**Situation**: Users reporting errors, you need immediate assessment.

```bash
# Quick triage to identify critical issues
logoscope /var/log/prod/*.log --triage

# Output shows only:
# - Status: CRITICAL/WARNING/NORMAL
# - Error count and critical patterns
# - Top 3 anomalies
# - Actionable insights
```

**Example Output**:
```json
{
  "summary": {
    "status": "CRITICAL",
    "error_lines": 1247,
    "burst_patterns": 3,
    "time_range": "2024-01-15T14:00:00Z to 2024-01-15T14:30:00Z"
  },
  "critical_patterns": [
    {
      "template": "Database connection timeout",
      "count": 847,
      "severity": "error"
    }
  ],
  "insights": [
    "CRITICAL: Database errors spiking at 14:15 UTC",
    "847 connection timeouts in 30 minutes"
  ]
}
```

### 🔍 Scenario 2: Performance Investigation - Verbose Mode

**Situation**: Application slow, need to prioritize issues by severity.

```bash
# Verbose mode - patterns ordered by importance
logoscope /var/log/app.log --verbose

# ERROR patterns appear first
# Then WARN patterns
# Then INFO patterns
# Finally DEBUG/TRACE patterns
```

**Pattern Priority Example**:
```bash
# See top 10 most important patterns with examples
logoscope logs/*.log --verbose --top 10 --examples 5 | jq '.patterns[:10]'
```

### 🔬 Scenario 3: Deep Root Cause Analysis

**Situation**: Complex issue requiring thorough investigation.

```bash
# Deep mode - maximum detail for investigation
logoscope /var/log/*.log --deep > deep_analysis.json

# Includes:
# - ALL patterns (even single occurrences)
# - Up to 10 examples per pattern
# - Full parameter statistics
# - Temporal distribution (hourly breakdown)
# - Cross-pattern correlations
```

**Drilling into specific issues**:
```bash
# Extract database error patterns with full context
logoscope logs/*.log --deep | jq '.patterns[] | select(.template | contains("database"))'
```

### 📊 Scenario 4: Pattern Analysis with Filters

```bash
# Focus on database-related ERROR patterns
logoscope logs/*.log --match "database|DB" --level error

# Exclude debug noise, show table format
logoscope logs/*.log --exclude "health|ping" --format table

# Group patterns by service
logoscope logs/*.log --group-by service --format table

# Sort by burst activity (find spikes)
logoscope logs/*.log --sort bursts --top 20
```

### ⏰ Scenario 5: Time-Window Analysis

```bash
# Analyze specific time window
logoscope logs/*.log \
  --start 2024-01-15T14:00:00Z \
  --end 2024-01-15T15:00:00Z

# Get context around critical time
logoscope logs/*.log \
  --start 2024-01-15T14:19:00Z \
  --end 2024-01-15T14:21:00Z \
  --before 5 --after 5
```

### 🔄 Scenario 6: Streaming Analysis

```bash
# Follow logs in real-time with periodic summaries
tail -F /var/log/app.log | logoscope --follow --interval 10 --window 300

# Outputs:
# - Status to stderr every 10 seconds
# - Pattern deltas to stdout (JSONL)
# - Full summary every 5 minutes
```

### 🎯 Scenario 7: Combined Workflows

#### Investigation Flow 1: From Triage to Deep Dive
```bash
# Step 1: Quick triage
logoscope logs/*.log --triage
# Result: "CRITICAL - Database errors"

# Step 2: Verbose analysis of errors
logoscope logs/*.log --verbose --level error --match database
# Result: Prioritized database error patterns

# Step 3: Deep dive into specific pattern
logoscope logs/*.log --deep --pattern "Database connection failed: <*>"
# Result: Full analysis with examples, parameters, correlations
```

#### Investigation Flow 2: Multi-Service Correlation
```bash
# Analyze multiple services together
logoscope api.log auth.log db.log payment.log --verbose

# See which services have errors
logoscope *.log --triage | jq '.critical_patterns[].template' | grep -E "service="

# Deep dive into service interactions
logoscope *.log --deep | jq '.correlations'
```

### 🛠️ Scenario 8: CI/CD Integration

```bash
#!/bin/bash
# pre-deploy-check.sh

# Run triage on recent logs
RESULT=$(logoscope /var/log/app.log --triage --last-hour)
STATUS=$(echo "$RESULT" | jq -r '.summary.status')

if [ "$STATUS" = "CRITICAL" ]; then
  echo "⛔ Deployment blocked: System in critical state"
  echo "$RESULT" | jq '.insights'
  exit 1
elif [ "$STATUS" = "WARNING" ]; then
  echo "⚠️ Warning: System has issues"
  echo "$RESULT" | jq '.insights'
  # Continue but notify
fi

echo "✅ System stable for deployment"
```

### 📈 Scenario 9: Monitoring Dashboard Feed

```bash
# Generate metrics for monitoring
while true; do
  # Get current status
  logoscope /var/log/app.log --triage --last-5min | \
    jq '{
      timestamp: now,
      status: .summary.status,
      error_rate: .summary.error_lines,
      anomalies: .summary.anomaly_count
    }' >> metrics.jsonl
  
  sleep 60
done
```

### 🔎 Scenario 10: Pattern Discovery

```bash
# Find new patterns introduced in latest deployment
logoscope logs/*.log --only patterns | jq '.patterns[] | select(.pattern_stability < 0.3)'

# Find rare but critical patterns
logoscope logs/*.log --verbose | \
  jq '.patterns[] | select(.frequency < 0.001 and .severity == "error")'

# Find patterns with parameter anomalies
logoscope logs/*.log --deep | \
  jq '.patterns[] | select(.parameter_anomalies != null)'
```

## Output Processing Examples

### Extract Specific Information

```bash
# Get just the error patterns
logoscope logs/*.log | jq '.patterns[] | select(.severity == "error")'

# Get patterns with bursts
logoscope logs/*.log | jq '.patterns[] | select(.temporal.bursts > 0)'

# Get top 5 patterns by count
logoscope logs/*.log | jq '.patterns[:5] | .[] | {template, count: .total_count}'

# Get anomalies only
logoscope logs/*.log | jq '.anomalies'

# Get schema changes
logoscope logs/*.log | jq '.schema_changes'
```

### Format for Reporting

```bash
# Create executive summary
logoscope logs/*.log --triage | jq -r '
  "Status: " + .summary.status + "\n" +
  "Errors: " + (.summary.error_lines | tostring) + "\n" +
  "Top Issue: " + .critical_patterns[0].template
'

# Create CSV of patterns
logoscope logs/*.log | jq -r '
  ["Template", "Count", "Severity"] as $headers |
  .patterns[] as $p |
  [$p.template, $p.total_count, $p.severity] |
  @csv
' > patterns.csv

# Create alert message
logoscope logs/*.log --triage | jq -r '
  if .summary.status == "CRITICAL" then
    "🚨 ALERT: " + .insights[0]
  else
    "✅ System Normal"
  end
'
```

## Mode Selection Guide

| Situation | Mode | Command | Purpose |
|-----------|------|---------|---------|
| Incident response | Triage | `--triage` | Quick assessment, critical issues only |
| Issue prioritization | Verbose | `--verbose` | Severity-ordered investigation |
| Root cause analysis | Deep | `--deep` | Maximum detail, all correlations |
| Routine monitoring | Normal | (default) | Balanced analysis |
| Pattern discovery | Patterns only | `--only patterns` | Focus on log patterns |
| Time investigation | Logs view | `--only logs --start X --end Y` | Raw logs in time window |

## Performance Tips

```bash
# For large files, use filters to reduce data
logoscope huge.log --level error --top 100

# Process compressed logs directly
zcat app.log.gz | logoscope -

# Parallel processing of multiple files
parallel -j 4 logoscope {} --triage ::: logs/*.log

# Stream processing for continuous logs
tail -F app.log | logoscope --follow --interval 5
```

## Integration Examples

### With GitHub Actions
```yaml
- name: Analyze logs
  run: |
    logoscope logs/*.log --triage > triage.json
    if [ $(jq -r '.summary.status' triage.json) = "CRITICAL" ]; then
      echo "::error::Critical issues detected in logs"
      exit 1
    fi
```

### With Kubernetes
```bash
# Analyze pod logs
kubectl logs deployment/api -n prod --since=1h | \
  logoscope - --triage

# Analyze multiple pods
for pod in $(kubectl get pods -n prod -o name); do
  echo "Analyzing $pod"
  kubectl logs $pod -n prod --since=1h | \
    logoscope - --triage
done
```

### With Docker
```bash
# Analyze container logs
docker logs myapp 2>&1 | logoscope - --verbose

# Analyze compose stack
docker-compose logs | logoscope - --triage
```

## Troubleshooting Common Issues

```bash
# Issue: Too many patterns
# Solution: Use filters
logoscope logs/*.log --min-count 10 --min-frequency 0.001

# Issue: Missing timestamps
# Solution: Specify time key
logoscope logs.json --time-key created_at --time-key timestamp

# Issue: Too much output
# Solution: Use triage mode or limit patterns
logoscope logs/*.log --triage
# OR
logoscope logs/*.log --top 20 --examples 2

# Issue: Need specific time range
# Solution: Use time filters
logoscope logs/*.log \
  --start 2024-01-15T14:00:00Z \
  --end 2024-01-15T15:00:00Z
```