#!/bin/bash

# Demo: Comparing All Logoscope Modes
# Shows the differences between normal, verbose, triage, and deep modes

echo "========================================="
echo "      MODE COMPARISON DEMO              "
echo "========================================="
echo

# Create sample logs with various severities and patterns
cat > comparison_logs.json << 'EOF'
{"level":"error","time":"2024-01-15T10:00:00Z","msg":"Payment failed","service":"payment","user":"user123","amount":99.99,"error":"card_declined"}
{"level":"error","time":"2024-01-15T10:00:01Z","msg":"Payment failed","service":"payment","user":"user456","amount":150.00,"error":"card_declined"}
{"level":"error","time":"2024-01-15T10:00:02Z","msg":"Payment failed","service":"payment","user":"user789","amount":75.50,"error":"insufficient_funds"}
{"level":"error","time":"2024-01-15T10:00:10Z","msg":"Database connection lost","service":"api","host":"db-primary","error":"connection_timeout"}
{"level":"error","time":"2024-01-15T10:00:11Z","msg":"Database connection lost","service":"api","host":"db-replica","error":"connection_timeout"}
{"level":"warn","time":"2024-01-15T10:01:00Z","msg":"High memory usage","service":"api","memory_mb":1850,"threshold_mb":1500}
{"level":"warn","time":"2024-01-15T10:01:30Z","msg":"High memory usage","service":"api","memory_mb":1920,"threshold_mb":1500}
{"level":"warn","time":"2024-01-15T10:02:00Z","msg":"Cache miss rate high","service":"cache","miss_rate":0.35,"threshold":0.20}
{"level":"info","time":"2024-01-15T10:00:00Z","msg":"User logged in","service":"auth","user":"alice","ip":"192.168.1.100"}
{"level":"info","time":"2024-01-15T10:00:30Z","msg":"User logged in","service":"auth","user":"bob","ip":"192.168.1.101"}
{"level":"info","time":"2024-01-15T10:01:00Z","msg":"User logged in","service":"auth","user":"charlie","ip":"192.168.1.102"}
{"level":"info","time":"2024-01-15T10:02:00Z","msg":"Order created","service":"orders","order_id":"ORD-123","total":199.99}
{"level":"info","time":"2024-01-15T10:02:30Z","msg":"Order created","service":"orders","order_id":"ORD-124","total":89.99}
{"level":"debug","time":"2024-01-15T10:00:00Z","msg":"Cache lookup","service":"cache","key":"product:123","hit":true}
{"level":"debug","time":"2024-01-15T10:00:01Z","msg":"Cache lookup","service":"cache","key":"product:456","hit":false}
{"level":"debug","time":"2024-01-15T10:00:02Z","msg":"SQL query executed","service":"db","query":"SELECT * FROM users","duration_ms":15}
{"level":"trace","time":"2024-01-15T10:00:00Z","msg":"Method entry","service":"api","method":"processRequest","thread":"worker-1"}
{"level":"trace","time":"2024-01-15T10:00:01Z","msg":"Method exit","service":"api","method":"processRequest","thread":"worker-1","duration_ms":25}
EOF

echo "📊 1. NORMAL MODE (Default)"
echo "==========================="
echo "Shows balanced analysis with all key information"
echo
OUTPUT_SIZE=$(./logoscope/target/debug/logoscope comparison_logs.json 2>/dev/null | wc -c)
echo "Output size: $OUTPUT_SIZE bytes"
echo "Pattern count: $(./logoscope/target/debug/logoscope comparison_logs.json 2>/dev/null | jq '.patterns | length')"
echo "Pattern order: $(./logoscope/target/debug/logoscope comparison_logs.json 2>/dev/null | jq -r '.patterns[:3] | .[] | .severity' | tr '\n' ' ')"
echo
echo "Sample output structure:"
./logoscope/target/debug/logoscope comparison_logs.json 2>/dev/null | jq 'keys'
echo

echo "Press Enter to see VERBOSE mode..."
read

echo "📊 2. VERBOSE MODE (--verbose)"
echo "=============================="
echo "Reorders patterns by importance (errors first)"
echo
echo "Pattern order: $(./logoscope/target/debug/logoscope comparison_logs.json --verbose 2>/dev/null | jq -r '.patterns[:5] | .[] | .severity' | tr '\n' ' ')"
echo
echo "Top 5 patterns by importance:"
./logoscope/target/debug/logoscope comparison_logs.json --verbose 2>/dev/null | \
  jq '.patterns[:5] | .[] | {severity, count: .total_count, template: (.template[:60] + "...")}'
echo

echo "Press Enter to see TRIAGE mode..."
read

echo "🚨 3. TRIAGE MODE (--triage)"
echo "============================"
echo "Ultra-compact output for rapid incident response"
echo
OUTPUT_SIZE=$(./logoscope/target/debug/logoscope comparison_logs.json --triage 2>/dev/null | wc -c)
echo "Output size: $OUTPUT_SIZE bytes (much smaller!)"
echo
./logoscope/target/debug/logoscope comparison_logs.json --triage 2>/dev/null | jq '.'
echo

echo "Press Enter to see DEEP mode..."
read

echo "🔬 4. DEEP MODE (--deep)"
echo "======================="
echo "Maximum detail for thorough investigation"
echo
OUTPUT_SIZE=$(./logoscope/target/debug/logoscope comparison_logs.json --deep 2>/dev/null | wc -c)
echo "Output size: $OUTPUT_SIZE bytes (largest)"
echo "All patterns shown: $(./logoscope/target/debug/logoscope comparison_logs.json --deep 2>/dev/null | jq '.patterns | length')"
echo "Examples per pattern: $(./logoscope/target/debug/logoscope comparison_logs.json --deep 2>/dev/null | jq '.patterns[0].examples | length')"
echo
echo "Sample of deep analysis features:"
./logoscope/target/debug/logoscope comparison_logs.json --deep 2>/dev/null | \
  jq '{
    total_patterns: .patterns | length,
    has_temporal: (.temporal_analysis != null),
    has_correlations: (.correlations != null),
    first_pattern_details: .patterns[0] | {
      template: (.template[:60] + "..."),
      examples_count: .examples | length,
      has_param_stats: (.param_stats != null)
    }
  }'
echo

echo "========================================="
echo "📋 MODE COMPARISON SUMMARY"
echo "========================================="
echo
echo "| Mode    | Use Case                  | Output Size | Key Feature            |"
echo "|---------|---------------------------|-------------|------------------------|"
echo "| Normal  | General analysis          | Medium      | Balanced detail        |"
echo "| Verbose | Issue prioritization      | Medium      | Severity ordering      |"
echo "| Triage  | Incident response         | Small       | Critical only          |"
echo "| Deep    | Root cause investigation  | Large       | Maximum detail         |"
echo
echo "========================================="
echo "💡 RECOMMENDATIONS:"
echo "========================================="
echo "• Start with --triage during incidents"
echo "• Use --verbose to prioritize multiple issues"
echo "• Switch to --deep for complex investigations"
echo "• Use normal mode for routine monitoring"
echo "========================================="