#!/usr/bin/env node
// Minimal MCP server for Logoscope exposed as `logoscope mcp`
import { Server } from '@modelcontextprotocol/sdk/server/index.js';
import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js';
import { CallToolRequestSchema, ListToolsRequestSchema, McpError } from '@modelcontextprotocol/sdk/types.js';
import { spawn } from 'child_process';
import { fileURLToPath } from 'url';
import { dirname, resolve } from 'path';
import fs from 'fs-extra';

import { getBinaryPath } from './index.js';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

function getPackageVersion() {
  const candidates = [
    resolve(__dirname, '..', 'package.json'),
    resolve(__dirname, '..', '..', 'package.json'),
  ];
  for (const p of candidates) {
    try {
      if (fs.existsSync(p)) {
        const j = JSON.parse(fs.readFileSync(p, 'utf-8'));
        if (j.version) return j.version;
      }
    } catch {}
  }
  return '0.0.0';
}

async function runLogoscope(args = [], stdinText = '', timeoutMs) {
  const bin = getBinaryPath();
  return new Promise((resolvePromise, reject) => {
    const child = spawn(bin, args, { stdio: ['pipe', 'pipe', 'pipe'] });
    let out = '';
    let err = '';
    if (stdinText && typeof stdinText === 'string') {
      child.stdin.write(stdinText);
    }
    child.stdin.end();
    child.stdout.on('data', (d) => (out += d.toString()));
    child.stderr.on('data', (d) => (err += d.toString()));

    let killed = false;
    let timer;
    if (timeoutMs && timeoutMs > 0) {
      timer = setTimeout(() => {
        killed = true;
        try { child.kill('SIGKILL'); } catch {}
        reject(new McpError('TimeoutError', `Timed out after ${timeoutMs}ms`));
      }, timeoutMs);
    }

    child.on('error', (e) => {
      if (timer) clearTimeout(timer);
      reject(new McpError('InternalError', `Spawn error: ${e.message}`));
    });
    child.on('close', (code) => {
      if (timer) clearTimeout(timer);
      if (killed) return; // already rejected
      if (code === 0) resolvePromise({ out, err });
      else reject(new McpError('InternalError', `logoscope exited with code ${code}\n${err}`));
    });
  });
}

function buildTools() {
  return [
    {
      name: 'log_anomalies',
      description:
        'Quick log analysis focused on anomalies and patterns. Uses triage mode for fast processing. IMPORTANT: This method should ALWAYS be used first when you want to analyze logs - it provides a quick overview before deciding if full analysis is needed. Accepts file paths or glob patterns. NOTE: To analyze program output, first save the output to a temporary file, then pass that file path to this method.',
      inputSchema: {
        type: 'object',
        properties: {
          paths: {
            type: 'array',
            items: { type: 'string' },
            description: 'File paths or glob patterns to analyze (e.g., "/var/log/*.log", "/tmp/program_output.log")',
          },
          timeout: { type: 'number', description: 'Timeout in seconds (default 30)' },
        },
        required: ['paths'],
      },
    },
    {
      name: 'full_log_analysis',
      description:
        'Comprehensive log analysis with full patterns, temporal insights, schema changes, anomalies, and AI suggestions. Use this method only after running log_anomalies first to get the initial overview. This provides detailed analysis when you need complete insights. Accepts file paths or glob patterns. NOTE: To analyze program output, first save the output to a temporary file, then pass that file path to this method.',
      inputSchema: {
        type: 'object',
        properties: {
          paths: {
            type: 'array',
            items: { type: 'string' },
            description: 'File paths or glob patterns to analyze (e.g., "/var/log/*.log", "/tmp/program_output.log")',
          },
          timeout: { type: 'number', description: 'Timeout in seconds (default 60)' },
        },
        required: ['paths'],
      },
    },
    {
      name: 'patterns_table',
      description:
        'Return a patterns-only view as a compact table with filtering, grouping, and sorting. NOTE: To analyze program output, first save the output to a temporary file, then pass that file path to this method.',
      inputSchema: {
        type: 'object',
        properties: {
          paths: { type: 'array', items: { type: 'string' }, description: 'File paths or glob patterns to analyze' },
          top: { type: 'number', description: 'Top N patterns to show' },
          minCount: { type: 'number' },
          minFrequency: { type: 'number' },
          match: { type: 'string' },
          exclude: { type: 'string' },
          level: { type: 'string' },
          examples: { type: 'number', description: 'Max examples per pattern' },
          groupBy: { type: 'string', enum: ['none', 'service', 'level'] },
          sortBy: { type: 'string', enum: ['count', 'freq', 'bursts', 'confidence'] },
          timeout: { type: 'number' },
        },
        required: ['paths'],
      },
    },
    {
      name: 'logs_slice',
      description:
        'Return a slice of raw logs filtered by time window and/or pattern with optional context lines. NOTE: To analyze program output, first save the output to a temporary file, then pass that file path to this method.',
      inputSchema: {
        type: 'object',
        properties: {
          paths: { type: 'array', items: { type: 'string' }, description: 'File paths or glob patterns to analyze' },
          start: { type: 'string', description: 'RFC3339 timestamp start' },
          end: { type: 'string', description: 'RFC3339 timestamp end' },
          pattern: { type: 'string', description: 'Template match to filter logs' },
          before: { type: 'number', description: 'Context lines before' },
          after: { type: 'number', description: 'Context lines after' },
          timeout: { type: 'number' },
        },
        required: ['paths'],
      },
    },
  ];
}

function sec(n) { return typeof n === 'number' && isFinite(n) && n > 0 ? Math.floor(n * 1000) : undefined; }

class LogoscopeMcpServer {
  constructor() {
    this.server = new Server(
      { name: '@logoscope/cli-mcp', version: getPackageVersion() },
      { capabilities: { tools: {} } }
    );
    this.server.onerror = (e) => console.error('[MCP error]', e);

    this.server.setRequestHandler(ListToolsRequestSchema, async () => ({ tools: buildTools() }));

    this.server.setRequestHandler(CallToolRequestSchema, async (req) => {
      const name = req.params.name;
      try {
        if (name === 'log_anomalies') {
          const args = req.params.arguments || {};
          const timeoutMs = sec(args.timeout) ?? 30000;
          const cli = ['--triage']; // Quick analysis with triage mode
          const paths = Array.isArray(args.paths) ? args.paths.map(String) : [];
          const { out } = await runLogoscope(cli.concat(paths), '', timeoutMs);
          return { content: [{ type: 'text', text: out }] };
        }

        if (name === 'full_log_analysis') {
          const args = req.params.arguments || {};
          const timeoutMs = sec(args.timeout) ?? 60000; // Longer timeout for full analysis
          const cli = []; // No --triage flag for comprehensive analysis
          const paths = Array.isArray(args.paths) ? args.paths.map(String) : [];
          const { out } = await runLogoscope(cli.concat(paths), '', timeoutMs);
          return { content: [{ type: 'text', text: out }] };
        }

        if (name === 'patterns_table') {
          const a = req.params.arguments || {};
          const timeoutMs = sec(a.timeout) ?? 30000;
          const cli = ['--only', 'patterns', '--format', 'table'];
          if (a.top != null) cli.push('--top', String(a.top));
          if (a.minCount != null) cli.push('--min-count', String(a.minCount));
          if (a.minFrequency != null) cli.push('--min-frequency', String(a.minFrequency));
          if (a.match) cli.push('--match', String(a.match));
          if (a.exclude) cli.push('--exclude', String(a.exclude));
          if (a.level) cli.push('--level', String(a.level));
          if (a.examples != null) cli.push('--examples', String(a.examples));
          if (a.groupBy) cli.push('--group-by', String(a.groupBy));
          if (a.sortBy) cli.push('--sort', String(a.sortBy));
          const paths = Array.isArray(a.paths) ? a.paths.map(String) : [];
          const { out } = await runLogoscope(cli.concat(paths), '', timeoutMs);
          return { content: [{ type: 'text', text: out }] };
        }

        if (name === 'logs_slice') {
          const a = req.params.arguments || {};
          const timeoutMs = sec(a.timeout) ?? 30000;
          const cli = ['--only', 'logs'];
          if (a.start) cli.push('--start', String(a.start));
          if (a.end) cli.push('--end', String(a.end));
          if (a.pattern) cli.push('--pattern', String(a.pattern));
          if (a.before != null) cli.push('--before', String(a.before));
          if (a.after != null) cli.push('--after', String(a.after));
          const paths = Array.isArray(a.paths) ? a.paths.map(String) : [];
          const { out } = await runLogoscope(cli.concat(paths), '', timeoutMs);
          return { content: [{ type: 'text', text: out }] };
        }

        throw new McpError('MethodNotFound', `Unknown tool: ${name}`);
      } catch (e) {
        const msg = e?.message || String(e);
        return { content: [{ type: 'text', text: msg }], isError: true };
      }
    });
  }

  async run() {
    const transport = new StdioServerTransport();
    await this.server.connect(transport);
    console.error('Logoscope MCP server running on stdio');
  }
}

const server = new LogoscopeMcpServer();
server.run().catch((e) => {
  console.error('[MCP fatal]', e);
  process.exit(1);
});

