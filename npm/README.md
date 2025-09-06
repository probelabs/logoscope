@logoscope/cli
================

Node.js wrapper for the Logoscope CLI. On install, downloads the platform-specific binary from GitHub releases and exposes a `logoscope` command.

Install
-------

```
npm i -g @logoscope/cli
```

Environment overrides
---------------------

- `LOGOSCOPE_REPO_OWNER` (default: `your-org`)
- `LOGOSCOPE_REPO_NAME` (default: `logoscope`)

Usage
-----

```
# Analyze files or stdin
logoscope app1.log app2.log > summary.json
cat app.log | logoscope --only patterns --format table
```

MCP Subcommand
--------------

This package exposes a Model Context Protocol (MCP) server as a subcommand to integrate with MCP‑compatible editors/agents.

```
# Start the MCP server over stdio
logoscope mcp
```

Available Tools
---------------

- analyze_logs — Full JSON summary (accepts `stdin` or `files[]`, optional `timeKey[]`, `timeout`).
- patterns_table — Patterns‑only table with filters (`top`, `minCount`, `minFrequency`, `match`, `exclude`, `level`, `examples`, `groupBy`, `sortBy`, `timeout`).
- logs_slice — Raw log slice filtered by `start`, `end`, `pattern`, with `before`/`after` context.

Example MCP Config (Claude Desktop)
-----------------------------------

Create or edit `~/.claude/mcp.json`:

```json
{
  "mcpServers": {
    "logoscope": {
      "type": "stdio",
      "command": "logoscope",
      "args": ["mcp"],
      "env": {}
    }
  }
}
```

Then ask your AI assistant to analyze logs using Logoscope tools.
