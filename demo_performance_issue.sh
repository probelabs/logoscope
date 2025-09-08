#!/bin/bash

# Demo: Performance Issue Investigation
# Shows how to identify and analyze performance problems in logs

echo "========================================="
echo "   PERFORMANCE INVESTIGATION DEMO       "
echo "========================================="
echo

# Create logs with performance degradation
cat > performance_logs.json << 'EOF'
{"level":"info","time":"2024-01-15T10:00:00Z","msg":"Request processed","service":"api","duration_ms":50,"endpoint":"/users"}
{"level":"info","time":"2024-01-15T10:00:01Z","msg":"Request processed","service":"api","duration_ms":45,"endpoint":"/users"}
{"level":"info","time":"2024-01-15T10:00:02Z","msg":"Request processed","service":"api","duration_ms":52,"endpoint":"/users"}
{"level":"info","time":"2024-01-15T10:01:00Z","msg":"Request processed","service":"api","duration_ms":150,"endpoint":"/users"}
{"level":"info","time":"2024-01-15T10:01:01Z","msg":"Request processed","service":"api","duration_ms":200,"endpoint":"/users"}
{"level":"warn","time":"2024-01-15T10:01:02Z","msg":"Slow request","service":"api","duration_ms":500,"endpoint":"/users"}
{"level":"warn","time":"2024-01-15T10:01:03Z","msg":"Slow request","service":"api","duration_ms":750,"endpoint":"/users"}
{"level":"warn","time":"2024-01-15T10:01:04Z","msg":"Slow request","service":"api","duration_ms":1200,"endpoint":"/users"}
{"level":"error","time":"2024-01-15T10:01:05Z","msg":"Request timeout","service":"api","duration_ms":5000,"endpoint":"/users"}
{"level":"info","time":"2024-01-15T10:00:00Z","msg":"Cache hit ratio","service":"cache","ratio":0.95}
{"level":"info","time":"2024-01-15T10:01:00Z","msg":"Cache hit ratio","service":"cache","ratio":0.80}
{"level":"warn","time":"2024-01-15T10:01:30Z","msg":"Cache hit ratio","service":"cache","ratio":0.45}
{"level":"warn","time":"2024-01-15T10:01:00Z","msg":"Memory usage high","service":"api","memory_mb":1800}
{"level":"warn","time":"2024-01-15T10:01:30Z","msg":"Memory usage high","service":"api","memory_mb":2100}
{"level":"error","time":"2024-01-15T10:02:00Z","msg":"Out of memory","service":"api","memory_mb":2500}
{"level":"info","time":"2024-01-15T10:00:00Z","msg":"Database query","service":"db","query_time_ms":10}
{"level":"info","time":"2024-01-15T10:01:00Z","msg":"Database query","service":"db","query_time_ms":100}
{"level":"warn","time":"2024-01-15T10:01:30Z","msg":"Database query","service":"db","query_time_ms":500}
{"level":"error","time":"2024-01-15T10:02:00Z","msg":"Database query timeout","service":"db","query_time_ms":5000}
EOF

echo "📊 ANALYZING PERFORMANCE DEGRADATION"
echo "===================================="
echo

echo "1️⃣ Quick Overview (Normal Mode):"
echo "--------------------------------"
./logoscope/target/debug/logoscope performance_logs.json 2>/dev/null | \
  jq '{
    total_lines: .summary.total_lines,
    unique_patterns: .summary.unique_patterns,
    time_range: (.summary.start_date + " to " + .summary.end_date)
  }'
echo

echo "2️⃣ Performance Anomalies (Field Anomalies):"
echo "-------------------------------------------"
./logoscope/target/debug/logoscope performance_logs.json 2>/dev/null | \
  jq '.anomalies.field_anomalies[] | select(contains("duration_ms") or contains("memory_mb") or contains("query_time_ms"))'
echo

echo "3️⃣ Warning/Error Patterns (Verbose Mode):"
echo "-----------------------------------------"
./logoscope/target/debug/logoscope performance_logs.json --verbose 2>/dev/null | \
  jq '.patterns[] | select(.severity == "error" or .severity == "warn") | 
    {
      severity,
      pattern: .template | split(" ") | map(select(. != "time" and . != "=")) | join(" ") | .[0:80],
      count: .total_count
    }'
echo

echo "4️⃣ Performance Metrics Analysis (Deep Mode):"
echo "--------------------------------------------"
echo "Duration statistics:"
./logoscope/target/debug/logoscope performance_logs.json --deep 2>/dev/null | \
  jq '.patterns[] | select(.template | contains("duration_ms")) | 
    {
      pattern: .template | split(" ") | map(select(startswith("msg") or startswith("duration"))) | join(" "),
      param_stats: .param_stats.DURATION_MS // .param_stats.duration_ms // {}
    }' | head -20
echo

echo "5️⃣ PERFORMANCE TIMELINE:"
echo "========================"
echo "10:00:00-10:00:02 - ✅ Normal (50ms avg response)"
echo "10:01:00-10:01:02 - ⚠️  Degradation begins (150-200ms)"
echo "10:01:02-10:01:04 - 🔶 Slow requests (500-1200ms)"
echo "10:01:05         - 🚨 Timeout (5000ms)"
echo "10:01:00-10:01:30 - 📉 Cache hit ratio drops (95% → 45%)"
echo "10:01:00-10:02:00 - 💾 Memory exhaustion (1800MB → 2500MB)"
echo "10:01:00-10:02:00 - 🗄️  Database slowdown (10ms → 5000ms)"
echo

echo "🎯 ROOT CAUSE INDICATORS:"
echo "========================="
echo "• Cache performance degraded → Increased DB load"
echo "• Database queries slowed → API response times increased"
echo "• Memory usage spiked → System instability"
echo "• Cascading failure: Cache → Database → API → OOM"
echo

echo "🔧 RECOMMENDED ACTIONS:"
echo "======================="
echo "1. Investigate cache eviction (hit ratio: 95% → 45%)"
echo "2. Check database indexes and query plans"
echo "3. Review memory allocation and garbage collection"
echo "4. Consider implementing circuit breakers"
echo

echo "========================================="
echo "Demo complete! Key takeaways:"
echo "• Use field anomalies to detect metric outliers"
echo "• Verbose mode helps prioritize performance issues"
echo "• Deep mode provides detailed parameter statistics"
echo "• Correlation of multiple signals reveals root cause"
echo "========================================="