# Logoscope Project - AI Assistant Guidelines

This file provides comprehensive guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Context
This is a Rust-based log analysis tool (logoscope) that implements Drain algorithm for log pattern clustering with performance-critical requirements and security analysis capabilities.

## Development Commands

### Rust (Core Library)
The main Rust codebase is located in `logoscope/`:

```bash
# Build the project
cd logoscope && cargo build --release

# Run tests (comprehensive test suite)
cd logoscope && cargo test

# Run specific test file
cd logoscope && cargo test multiline_tests

# Run benchmarks
cd logoscope && cargo run --bin benchmark_smart_masking

# Build and run the main binary
cd logoscope && cargo run -- [args]
```

### Node.js Wrapper
The npm package wrapper is in `npm/`:

```bash
# Install dependencies
cd npm && npm install

# Test the wrapper (runs postinstall to download binary)
cd npm && npm run postinstall
```

## Performance & Architecture Principles

### Critical Performance Requirements
- **Preserve 3s benchmark**: The system must process 100k records in ~3 seconds. Never sacrifice proven performance optimizations for architectural purity
- **Use release builds for testing**: Always use `cargo build --release` when testing struct changes to ensure latest field definitions are included
- **Add timing instrumentation first**: When performance issues are reported, immediately add detailed timing across all stages before investigating specific bottlenecks

### Parallel Processing Patterns
- Use thread-local caches with global fallback to avoid lock contention in expensive operations
- Implement static `Lazy<Regex>` patterns with prewarm functions to prevent compilation contention
- Test both debug and release builds - release optimizations can cause unexpected behavior

### Architecture Decisions
- Multi-pass approaches work better than complex cross-references for data transformations
- Structure-first canonicalization (JSON → consistent key=placeholder format) dramatically improves clustering
- Dual tracking: mask for pattern clustering while preserving original values for security analysis

### Performance Optimization History
- Successfully achieved 86% improvement (15s → 2.1s) through systematic multi-stage optimization
- Thread-local LRU caching eliminated lock contention bottlenecks
- Structure-first canonicalization provided 12x pattern reduction (300→24 patterns)

## Architecture Overview

### Core Components

**logoscope/src/lib.rs** - Main module declarations for the Rust library

**Key modules:**
- **parser.rs** - Log parsing with multi-format support (JSON + plaintext)
- **multiline.rs** - Handles stack traces and multi-line log consolidation
- **masking.rs & smart_masking.rs** - PII masking and high-cardinality data normalization
- **drain_adapter.rs** - Integration with Drain algorithm for pattern extraction
- **patterns.rs** - Pattern extraction, stability scoring, template optimization
- **schema.rs** - Schema fingerprinting and change detection
- **anomaly.rs & field_anomaly.rs** - Anomaly detection (new patterns, numeric outliers)
- **temporal.rs** - Temporal analysis (bursts, gaps, spikes)
- **correlation.rs** - Cross-pattern and cross-file correlation analysis
- **ai.rs** - AI-powered insights and suggestions
- **query.rs** - Query interface for log slicing and filtering
- **param_extractor.rs** - Parameter extraction and statistics

### Binary Structure
- **src/bin/logoscope.rs** - Main CLI with clap argument parsing
- **src/bin/benchmark_smart_masking.rs** - Performance benchmarking tool

### Data Flow
1. **Input Processing**: Multi-line aggregation → JSON/plaintext parsing → timestamp detection
2. **Pattern Extraction**: Masking → Drain clustering → Template optimization → Parameter analysis
3. **Analysis**: Schema tracking → Anomaly detection → Temporal analysis → Correlation analysis
4. **Output**: JSON summary with patterns, anomalies, temporal insights, and query suggestions

## Key Features

### Pattern Analysis
- Uses Drain algorithm for stable log template extraction
- Smart masking for PII and high-cardinality values (<NUM>, <IP>, <EMAIL>, etc.)
- Pattern stability scoring combines temporal consistency and frequency
- Parameter statistics with cardinality analysis and value distributions

### Anomaly Detection
- New/rare pattern detection
- Numeric outliers using robust z-score (median/MAD)
- Cardinality explosions in parameter values
- Temporal anomalies (bursts, gaps, spikes)

### Multi-format Support
- Auto-detection between JSON and plaintext per line
- Multi-line consolidation for stack traces and bracket-balanced JSON
- Timestamp auto-detection: ISO8601, syslog, epoch formats

### Streaming & Performance
- Streaming mode with rolling windows and periodic summaries
- Parallel processing with rayon
- Fixed-depth Drain tree for bounded memory usage
- Optimized for analyzing gigabytes of logs efficiently

## Testing Strategy

The codebase has comprehensive test coverage in `logoscope/tests/`:
- Integration tests for each major component
- AI-powered analysis tests
- Multi-format parsing tests
- Anomaly detection validation
- Performance regression tests

## Rust-Specific Patterns

### Build & Test
- Run `cargo check` and `cargo build --release` frequently to catch issues early
- Always create comprehensive test coverage for new modules before marking complete
- Use debug builds cautiously - they may contain outdated struct definitions

### Code Quality
- Manual parsing algorithms work better than regex for variable-length values with spaces
- Always verify template replacements create actual changes to prevent infinite loops
- Use descriptive parameter names like 'NESTED_PATTERN' for extracted nested values

### Common Debug Patterns
- MSG parameter splitting issues typically involve canonicalization key selection, not parsing logic
- Template truncation often caused by overly broad pattern matching catching placeholders
- Hanging issues may be infinite loops in template humanization, not regex backtracking

## Output Formats
- **Default**: Comprehensive JSON summary with patterns, anomalies, schema changes
- **--only patterns**: Pattern table or JSON focused view
- **--only logs**: Time-sliced log extraction with context
- **--format table**: Human-readable table output for patterns

## Security & Analysis Requirements

### Anomaly Detection
- Show both concentration anomalies (dominant values) AND rare outliers in same analysis
- Remove features that complicate analysis without clear debugging value
- Use descriptive field names ('pattern_stability') instead of generic terms ('confidence')

### Log Processing
- For structured data requested as single unit (USER_AGENT), don't parse components
- Provide aggregate temporal metrics (start/end times) rather than individual timestamps
- Ensure parameter tracking corresponds logically to template placeholders
- Test with both `--chunked` and `--no-chunked` modes explicitly when validating consistency
- Replace single-value placeholders with actual values in templates for readability

## User Communication Preferences

### Response Style
- **Concise and direct**: Provide clear problem identification with specific solutions
- **Real-world focused**: Use exact user-provided test data rather than simplified examples
- **Performance-oriented**: Always include concrete metrics and timing breakdowns to validate improvements

### Tool Usage Patterns
- Use @agent-architect for complex multi-phase implementations when explicitly requested
- Use TodoWrite proactively for multi-step tasks to show progress and organization
- Use Big Brain agent with complete code context for complex optimization recommendations

## Critical Boundaries & Constraints

### Never Do
- Hard-code application-specific field names (ORG_ID, API_ID) in generic systems
- Use `rm -rf` commands in testing
- Create documentation files unless explicitly requested
- Filter out useful anomaly information even if it seems noisy
- Sacrifice proven performance for architectural changes without validation

### Always Do
- Preserve existing fast implementations when adding abstraction layers
- Use exact JSON examples from user for debugging parameter extraction issues
- Verify canonicalization uses raw JSON messages, not flattened key=value representations
- Test with both `--chunked` and `--no-chunked` modes explicitly when validating consistency
- Replace single-value placeholders with actual values in templates for readability

## Project-Specific Context

### Logoscope Components
- Drain algorithm integration with smart masking for log pattern clustering
- Chunked vs non-chunked modes with different analytical capabilities
- Parameter extraction with anomaly detection for security analysis
- JSON log canonicalization for consistent clustering

## Environment Notes
- macOS (darwin) development environment
- Git hooks with vow-check requiring consent codes for commits
- npm test timeouts set to 10 minutes when needed
- Uses ripgrep (rg), cargo, jq, and standard Rust toolchain

## Deployment
- Rust binary for performance-critical log processing
- Node.js wrapper (`@logoscope/cli`) for npm ecosystem integration
- MCP server integration for AI assistant tools