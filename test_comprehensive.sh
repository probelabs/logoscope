#!/bin/bash

echo "=== Testing Logoscope Modes ==="
echo

echo "1. NORMAL MODE:"
echo "Pattern count: $(./target/debug/logoscope ../test_logs.json 2>/dev/null | jq '.patterns | length')"
echo "Compression ratio: $(./target/debug/logoscope ../test_logs.json 2>/dev/null | jq '.summary.compression_ratio')"
echo

echo "2. VERBOSE MODE (--verbose):"
echo "First pattern severity: $(./target/debug/logoscope ../test_logs.json --verbose 2>/dev/null | jq -r '.patterns[0].severity')"
echo "Second pattern severity: $(./target/debug/logoscope ../test_logs.json --verbose 2>/dev/null | jq -r '.patterns[1].severity')"
echo "Third pattern severity: $(./target/debug/logoscope ../test_logs.json --verbose 2>/dev/null | jq -r '.patterns[2].severity')"
echo

echo "3. TRIAGE MODE (--triage):"
echo "Status: $(./target/debug/logoscope ../test_logs.json --triage 2>/dev/null | jq -r '.summary.status')"
echo "Error lines: $(./target/debug/logoscope ../test_logs.json --triage 2>/dev/null | jq '.summary.error_lines')"
echo "Critical patterns: $(./target/debug/logoscope ../test_logs.json --triage 2>/dev/null | jq '.critical_patterns | length')"
echo

echo "4. DEEP MODE (--deep):"
echo "All patterns shown: $(./target/debug/logoscope ../test_logs.json --deep 2>/dev/null | jq '.patterns | length')"
echo "Examples in first pattern: $(./target/debug/logoscope ../test_logs.json --deep 2>/dev/null | jq '.patterns[0].examples | length')"
echo "Has temporal analysis: $(./target/debug/logoscope ../test_logs.json --deep 2>/dev/null | jq 'has("temporal_analysis")')"
echo "Has correlations: $(./target/debug/logoscope ../test_logs.json --deep 2>/dev/null | jq 'has("correlations")')"
echo

echo "=== All tests completed ==="