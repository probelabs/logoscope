#!/bin/bash

# Demo: Real-time Log Monitoring with Streaming Mode
# Shows how to use logoscope for continuous log analysis

echo "========================================="
echo "     REAL-TIME MONITORING DEMO          "
echo "========================================="
echo

# Function to generate continuous logs
generate_logs() {
    local count=0
    while true; do
        count=$((count + 1))
        timestamp=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
        
        # Normal traffic (80% of logs)
        if [ $((count % 10)) -lt 8 ]; then
            echo "{\"level\":\"info\",\"time\":\"$timestamp\",\"msg\":\"Request processed\",\"service\":\"api\",\"status\":200,\"duration_ms\":$((30 + RANDOM % 70))}"
        # Warnings (15% of logs)
        elif [ $((count % 10)) -eq 8 ]; then
            echo "{\"level\":\"warn\",\"time\":\"$timestamp\",\"msg\":\"High latency detected\",\"service\":\"api\",\"duration_ms\":$((500 + RANDOM % 500))}"
        # Errors (5% of logs)
        else
            echo "{\"level\":\"error\",\"time\":\"$timestamp\",\"msg\":\"Request failed\",\"service\":\"api\",\"status\":500,\"error\":\"internal_error\"}"
        fi
        
        # Occasional bursts
        if [ $((count % 50)) -eq 0 ]; then
            for i in {1..5}; do
                echo "{\"level\":\"error\",\"time\":\"$timestamp\",\"msg\":\"Database connection error\",\"service\":\"db\",\"error\":\"timeout\"}"
            done
        fi
        
        sleep 0.5
    done
}

echo "📡 Starting real-time log generation and monitoring..."
echo "This demo will:"
echo "1. Generate continuous log entries"
echo "2. Show periodic summaries every 5 seconds"
echo "3. Highlight pattern changes and anomalies"
echo
echo "Press Ctrl+C to stop the demo"
echo
echo "========================================="
echo

# Create a named pipe for log streaming
mkfifo /tmp/logoscope_demo_pipe 2>/dev/null || true

# Start log generator in background
generate_logs > /tmp/logoscope_demo_pipe &
GENERATOR_PID=$!

# Cleanup function
cleanup() {
    echo
    echo "Stopping demo..."
    kill $GENERATOR_PID 2>/dev/null
    rm -f /tmp/logoscope_demo_pipe
    exit 0
}
trap cleanup INT TERM

echo "🚀 Monitoring started. Streaming analysis output:"
echo

# Run logoscope in streaming mode
cat /tmp/logoscope_demo_pipe | ./logoscope/target/debug/logoscope \
    --follow \
    --interval 5 \
    --window 30 \
    - 2>&1 | while IFS= read -r line; do
    
    # Format output for better readability
    if echo "$line" | grep -q "^\[stream\]"; then
        # Status updates (from stderr)
        echo "📊 $line"
    elif echo "$line" | grep -q "template.*delta"; then
        # Pattern deltas (JSONL from stdout)
        echo "$line" | jq -r '"📈 Pattern change: " + .template[:60] + " (Δ" + (.delta | tostring) + ")"' 2>/dev/null || echo "$line"
    elif echo "$line" | grep -q "summary"; then
        # Periodic summaries
        echo "📋 Full Summary:"
        echo "$line" | jq '.' 2>/dev/null || echo "$line"
    else
        echo "$line"
    fi
done

cleanup