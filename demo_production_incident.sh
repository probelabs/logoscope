#!/bin/bash

# Demo: Production Incident Analysis Workflow
# This demonstrates how to use logoscope during a production incident

echo "========================================="
echo "   PRODUCTION INCIDENT ANALYSIS DEMO    "
echo "========================================="
echo

# Create realistic production logs with an incident
cat > incident_logs.json << 'EOF'
{"level":"info","time":"2024-01-15T14:00:00Z","msg":"Service started","service":"api"}
{"level":"info","time":"2024-01-15T14:05:00Z","msg":"Health check passed","service":"api"}
{"level":"info","time":"2024-01-15T14:10:00Z","msg":"User login","service":"auth","user":"alice"}
{"level":"info","time":"2024-01-15T14:15:00Z","msg":"User login","service":"auth","user":"bob"}
{"level":"warn","time":"2024-01-15T14:19:00Z","msg":"Database slow query","service":"api","duration_ms":1500}
{"level":"warn","time":"2024-01-15T14:19:30Z","msg":"Database slow query","service":"api","duration_ms":2000}
{"level":"error","time":"2024-01-15T14:20:00Z","msg":"Database connection failed","service":"api","error":"timeout"}
{"level":"error","time":"2024-01-15T14:20:01Z","msg":"Database connection failed","service":"api","error":"timeout"}
{"level":"error","time":"2024-01-15T14:20:02Z","msg":"Database connection failed","service":"api","error":"timeout"}
{"level":"error","time":"2024-01-15T14:20:03Z","msg":"Database connection failed","service":"api","error":"timeout"}
{"level":"error","time":"2024-01-15T14:20:04Z","msg":"Database connection failed","service":"api","error":"timeout"}
{"level":"error","time":"2024-01-15T14:20:05Z","msg":"Database connection failed","service":"api","error":"timeout"}
{"level":"error","time":"2024-01-15T14:20:10Z","msg":"API request failed","service":"api","status":500,"path":"/users"}
{"level":"error","time":"2024-01-15T14:20:11Z","msg":"API request failed","service":"api","status":500,"path":"/orders"}
{"level":"error","time":"2024-01-15T14:20:12Z","msg":"API request failed","service":"api","status":500,"path":"/products"}
{"level":"warn","time":"2024-01-15T14:20:30Z","msg":"Circuit breaker opened","service":"api","component":"database"}
{"level":"info","time":"2024-01-15T14:21:00Z","msg":"Attempting database reconnection","service":"api"}
{"level":"info","time":"2024-01-15T14:21:30Z","msg":"Database connection restored","service":"api"}
{"level":"info","time":"2024-01-15T14:22:00Z","msg":"Service recovered","service":"api"}
{"level":"info","time":"2024-01-15T14:25:00Z","msg":"Health check passed","service":"api"}
EOF

echo "📊 STEP 1: INITIAL TRIAGE"
echo "========================="
echo "Command: logoscope incident_logs.json --triage"
echo
./logoscope/target/debug/logoscope incident_logs.json --triage 2>/dev/null | jq '.'
echo

read -p "Press Enter to continue to verbose analysis..."
echo

echo "🔍 STEP 2: VERBOSE ANALYSIS (Prioritized by Severity)"
echo "====================================================="
echo "Command: logoscope incident_logs.json --verbose"
echo
echo "Top 3 patterns by importance:"
./logoscope/target/debug/logoscope incident_logs.json --verbose 2>/dev/null | \
  jq '.patterns[:3] | .[] | {severity, count: .total_count, template: .template[:80]}'
echo

read -p "Press Enter to continue to deep investigation..."
echo

echo "🔬 STEP 3: DEEP INVESTIGATION"
echo "=============================="
echo "Command: logoscope incident_logs.json --deep"
echo
echo "Database error pattern with full details:"
./logoscope/target/debug/logoscope incident_logs.json --deep 2>/dev/null | \
  jq '.patterns[] | select(.template | contains("Database connection failed")) | 
    {
      template: .template[:80],
      count: .total_count,
      examples: .examples | length,
      start_time,
      end_time,
      severity
    }'
echo

echo "Temporal anomalies detected:"
./logoscope/target/debug/logoscope incident_logs.json 2>/dev/null | \
  jq '.anomalies.temporal_anomalies[]' 2>/dev/null || echo "No temporal anomalies"
echo

read -p "Press Enter to see the incident timeline..."
echo

echo "📈 INCIDENT TIMELINE"
echo "==================="
echo
echo "14:00-14:15 - Normal operation (info logs)"
echo "14:19      - ⚠️  Database slowdown begins (warnings)"
echo "14:20      - 🚨 Database failure cascade (6 timeouts + 3 API failures)"
echo "14:20:30   - Circuit breaker triggered"
echo "14:21:30   - ✅ Database connection restored"
echo "14:22      - Service recovered"
echo

echo "🎯 KEY INSIGHTS"
echo "==============="
./logoscope/target/debug/logoscope incident_logs.json --triage 2>/dev/null | \
  jq -r '.insights[]'
echo

echo "========================================="
echo "Demo complete! This workflow showed how to:"
echo "1. Use --triage for immediate incident assessment"
echo "2. Use --verbose to prioritize issues by severity"
echo "3. Use --deep for root cause investigation"
echo "========================================="