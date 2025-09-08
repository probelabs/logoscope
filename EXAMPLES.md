# Logoscope Examples & Demos

## Quick Examples

### Basic Usage
```bash
# Analyze a single log file
logoscope app.log > analysis.json

# Analyze multiple files
logoscope app1.log app2.log app3.log > combined.json

# Analyze from stdin
cat app.log | logoscope -

# Analyze compressed logs
zcat app.log.gz | logoscope -
```

### Mode Selection

#### 🚨 Triage Mode - Rapid Incident Response
```bash
# Quick assessment during an incident
logoscope /var/log/prod/*.log --triage

# Example output (compact, critical only):
{
  "summary": {
    "status": "CRITICAL",
    "error_lines": 847,
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
    "CRITICAL: Database errors spiking at 14:15 UTC"
  ]
}
```

#### 📊 Verbose Mode - Priority-Based Investigation
```bash
# See errors first, then warnings, then info
logoscope app.log --verbose

# Errors appear at top, even if less frequent
# Pattern ordering: ERROR > WARN > INFO > DEBUG > TRACE
```

#### 🔬 Deep Mode - Root Cause Analysis
```bash
# Maximum detail for thorough investigation
logoscope app.log --deep

# Includes:
# - ALL patterns (even single occurrences)
# - Up to 10 examples per pattern
# - Full parameter statistics
# - Temporal analysis
# - Cross-pattern correlations
```

### Real-World Scenarios

#### Scenario 1: Database Outage Investigation
```bash
# Step 1: Quick triage
logoscope prod-*.log --triage
# Output: "CRITICAL - 847 database timeouts"

# Step 2: Focus on database errors
logoscope prod-*.log --verbose --match "database|DB" --level error

# Step 3: Deep dive with context
logoscope prod-*.log --deep \
  --start 2024-01-15T14:19:00Z \
  --end 2024-01-15T14:21:00Z \
  --pattern "Database connection failed: <*>"
```

#### Scenario 2: Performance Degradation
```bash
# Find slow queries and high latency patterns
logoscope app.log --verbose | \
  jq '.patterns[] | select(.template | contains("duration_ms")) | 
    select(.param_stats.DURATION_MS.values[0].value | tonumber > 1000)'

# Check for memory issues
logoscope app.log --match "memory|heap" --format table

# Analyze specific time window when slowdown occurred
logoscope app.log \
  --start 2024-01-15T10:00:00Z \
  --end 2024-01-15T11:00:00Z \
  --only patterns
```

#### Scenario 3: Security Incident
```bash
# Find authentication failures
logoscope auth.log --match "fail|denied|unauthorized" --level error

# Check for unusual IP patterns
logoscope access.log --deep | \
  jq '.patterns[] | select(.param_stats.IP) | 
    {
      pattern: .template,
      unique_ips: .param_stats.IP.cardinality,
      top_ip: .param_stats.IP.values[0]
    }'

# Investigate specific user activity
logoscope app.log --pattern "*user = alice*" --before 10 --after 10
```

### Filtering and Formatting

#### Pattern Filtering
```bash
# Only errors with more than 10 occurrences
logoscope app.log --level error --min-count 10

# Exclude health checks and ping endpoints
logoscope app.log --exclude "health|ping|metrics"

# Focus on specific service
logoscope app.log --service payment --level error
```

#### Output Formatting
```bash
# Table format grouped by severity
logoscope app.log --format table --group-by level

# Top 10 patterns sorted by burst activity
logoscope app.log --format table --sort bursts --top 10

# JSON with only patterns
logoscope app.log --only patterns | jq '.patterns[:5]'
```

### Time-Based Analysis

```bash
# Last hour of logs
logoscope app.log --start "$(date -u -d '1 hour ago' '+%Y-%m-%dT%H:%M:%SZ')"

# Specific time window
logoscope app.log \
  --start 2024-01-15T14:00:00Z \
  --end 2024-01-15T15:00:00Z

# Get context around specific time
logoscope app.log \
  --start 2024-01-15T14:19:50Z \
  --end 2024-01-15T14:20:10Z \
  --before 5 --after 5
```

### Streaming & Real-time Monitoring

```bash
# Follow logs with periodic summaries
tail -F /var/log/app.log | \
  logoscope --follow --interval 10 --window 300

# Monitor Kubernetes pod logs
kubectl logs -f deployment/api | \
  logoscope --follow --interval 5

# Docker container monitoring
docker logs -f mycontainer 2>&1 | \
  logoscope --follow --triage --interval 10
```

### Integration Examples

#### CI/CD Pipeline Check
```bash
#!/bin/bash
STATUS=$(logoscope app.log --triage | jq -r '.summary.status')
if [ "$STATUS" = "CRITICAL" ]; then
  echo "Deployment blocked: Critical errors detected"
  exit 1
fi
```

#### Monitoring Alert Script
```bash
#!/bin/bash
while true; do
  RESULT=$(logoscope /var/log/app.log --triage --last-5min)
  STATUS=$(echo "$RESULT" | jq -r '.summary.status')
  
  if [ "$STATUS" = "CRITICAL" ]; then
    # Send alert
    echo "$RESULT" | mail -s "CRITICAL: Log Alert" oncall@example.com
  fi
  
  sleep 60
done
```

#### Slack/Discord Notification
```bash
# Create alert message
ALERT=$(logoscope app.log --triage | jq -r '
  if .summary.status == "CRITICAL" then
    "🚨 *CRITICAL ALERT*\n" +
    "Errors: " + (.summary.error_lines | tostring) + "\n" +
    "Top Issue: " + .critical_patterns[0].template
  else
    "✅ System Normal"
  end
')

# Send to Slack
curl -X POST -H 'Content-type: application/json' \
  --data "{\"text\":\"$ALERT\"}" \
  $SLACK_WEBHOOK_URL
```

### Advanced Analysis

#### Find Patterns with Anomalies
```bash
# Patterns with parameter anomalies
logoscope app.log --deep | \
  jq '.patterns[] | select(.parameter_anomalies) | 
    {
      pattern: .template[:60],
      anomalies: .parameter_anomalies
    }'

# Patterns with temporal bursts
logoscope app.log | \
  jq '.patterns[] | select(.temporal.bursts > 0) | 
    {
      pattern: .template[:60],
      bursts: .temporal.bursts,
      largest_burst: .temporal.largest_burst
    }'
```

#### Cross-Service Correlation
```bash
# Analyze multiple services together
logoscope api.log auth.log payment.log db.log --deep | \
  jq '.correlations[] | select(.strength > 0.7)'

# Find cascading failures
logoscope *.log --verbose | \
  jq '.patterns[] | select(.severity == "error") | 
    {
      service: (.sources.by_service[0].name // "unknown"),
      pattern: .template[:60],
      count: .total_count,
      time: .start_time
    }' | \
  jq -s 'sort_by(.time)'
```

#### Pattern Evolution
```bash
# Find new patterns in recent deployment
BEFORE=$(logoscope old-logs/*.log --only patterns | jq '.patterns[].template')
AFTER=$(logoscope new-logs/*.log --only patterns | jq '.patterns[].template')
diff <(echo "$BEFORE" | sort) <(echo "$AFTER" | sort) | grep "^>"
```

### Performance Tips

```bash
# For huge files, limit pattern count
logoscope huge.log --max-patterns 100

# Process in parallel
parallel -j 4 'logoscope {} --triage > {.}_analysis.json' ::: logs/*.log

# Sample large files
shuf -n 10000 huge.log | logoscope -

# Use time windows for large datasets
for hour in {00..23}; do
  logoscope app.log \
    --start "2024-01-15T${hour}:00:00Z" \
    --end "2024-01-15T${hour}:59:59Z" \
    --triage > "hour_${hour}_analysis.json"
done
```

## Demo Scripts

Several demo scripts are provided in the repository:

1. **demo_production_incident.sh** - Shows incident response workflow
2. **demo_performance_issue.sh** - Demonstrates performance investigation
3. **demo_mode_comparison.sh** - Compares all modes side-by-side
4. **demo_realtime_monitoring.sh** - Real-time streaming analysis

Run any demo:
```bash
chmod +x demo_*.sh
./demo_production_incident.sh
```

## Common Use Cases

### Daily Operations
```bash
# Morning system check
logoscope /var/log/app-$(date +%Y%m%d).log --triage

# Error summary for standup
logoscope yesterday.log --level error --format table --top 10
```

### Incident Response
```bash
# Immediate triage
logoscope *.log --triage

# Find error spike time
logoscope *.log --verbose | jq '.patterns[] | select(.severity == "error") | .temporal.largest_burst'

# Deep dive into spike window
logoscope *.log --deep --start <spike_time> --end <spike_end>
```

### Capacity Planning
```bash
# Analyze growth patterns
logoscope app-*.log --deep | \
  jq '.temporal_analysis.hourly_distribution'

# Find resource bottlenecks
logoscope app.log --match "memory|cpu|disk" --verbose
```

### Debugging
```bash
# Find rare errors
logoscope app.log | \
  jq '.patterns[] | select(.frequency < 0.001 and .severity == "error")'

# Trace specific request
REQUEST_ID="req-12345"
logoscope app.log --match "$REQUEST_ID" --before 10 --after 10
```

## Tips & Tricks

1. **Start with triage** during incidents for quick assessment
2. **Use verbose mode** when you need to prioritize multiple issues
3. **Switch to deep mode** only for complex investigations
4. **Combine with jq** for custom filtering and formatting
5. **Use --time-key** for custom timestamp fields in JSON logs
6. **Stream mode** for continuous monitoring
7. **Save analyses** to JSON for historical comparison
8. **Use --examples** to control output verbosity

## Need Help?

```bash
# Show all options
logoscope --help

# Report issues
https://github.com/your-org/logoscope/issues
```