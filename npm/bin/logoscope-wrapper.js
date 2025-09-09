#!/usr/bin/env node
import { spawn } from 'child_process';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

// Check if the first argument is 'mcp'
if (process.argv[2] === 'mcp') {
  // Launch the MCP server
  const mcpServerPath = join(__dirname, '..', 'src', 'mcp-server.js');
  const child = spawn('node', [mcpServerPath, ...process.argv.slice(3)], {
    stdio: 'inherit'
  });
  
  child.on('exit', (code) => {
    process.exit(code || 0);
  });
} else {
  // Launch the actual logoscope binary
  const isWindows = process.platform === 'win32';
  const binaryName = isWindows ? 'logoscope.exe' : 'logoscope';
  const binaryPath = join(__dirname, binaryName);
  
  const child = spawn(binaryPath, process.argv.slice(2), {
    stdio: 'inherit'
  });
  
  child.on('exit', (code) => {
    process.exit(code || 0);
  });
}