import { spawn } from 'child_process';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

export function getBinaryPath() {
  const isWindows = process.platform === 'win32';
  const binaryName = isWindows ? 'logoscope.exe' : 'logoscope';
  return join(__dirname, '..', 'bin', binaryName);
}

export function run(args = [], options = {}) {
  const bin = getBinaryPath();
  return spawn(bin, args, { stdio: 'inherit', ...options });
}
