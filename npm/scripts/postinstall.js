#!/usr/bin/env node
import fs from 'fs-extra';
import path from 'path';
import { fileURLToPath } from 'url';
import { downloadBinary } from '../src/downloader.js';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const binDir = path.resolve(__dirname, '..', 'bin');

async function main() {
  try {
    await fs.ensureDir(binDir);
    const readmePath = path.join(binDir, 'README.md');
    await fs.writeFile(readmePath, '# Logoscope Binary Directory\n');
    const gitignorePath = path.join(binDir, '.gitignore');
    await fs.writeFile(gitignorePath, '*\n!.gitignore\n!.gitkeep\n!README.md\n!logoscope\n');

    // Determine package version
    let pkgVersion = '0.0.0';
    const pkgPaths = [
      path.resolve(__dirname, '..', 'package.json'),
      path.resolve(__dirname, '..', '..', 'package.json')
    ];
    for (const p of pkgPaths) {
      try { if (fs.existsSync(p)) { const j = JSON.parse(fs.readFileSync(p, 'utf-8')); if (j.version) { pkgVersion = j.version; break; } } } catch {}
    }

    // Download
    const dest = await downloadBinary(pkgVersion);
    const isWindows = process.platform === 'win32';
    const target = path.join(binDir, isWindows ? 'logoscope.exe' : 'logoscope');
    if (dest !== target) {
      await fs.copyFile(dest, target);
      await fs.chmod(target, 0o755);
    }
    console.log('Logoscope binary installed to', target);
  } catch (err) {
    console.warn('[logoscope] postinstall warning:', err.message);
  }
}

main().catch(e => console.error(e));

