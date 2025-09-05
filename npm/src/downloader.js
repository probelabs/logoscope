import axios from 'axios';
import fs from 'fs-extra';
import path from 'path';
import tar from 'tar';
import { detectOsArch, ensureBinDirectory } from './utils.js';
import os from 'os';

const DEFAULT_OWNER = process.env.LOGOSCOPE_REPO_OWNER || 'probelabs';
const DEFAULT_REPO = process.env.LOGOSCOPE_REPO_NAME || 'logoscope';
const BINARY_NAME = 'logoscope';

const LOCAL_DIR = path.resolve(path.dirname(new URL(import.meta.url).pathname), '..', 'bin');

export async function downloadBinary(version) {
  await ensureBinDirectory(LOCAL_DIR);
  const { os: osInfo, arch: archInfo } = detectOsArch();
  const owner = DEFAULT_OWNER;
  const repo = DEFAULT_REPO;
  const isWindows = os.platform() === 'win32';
  const targetName = isWindows ? `${BINARY_NAME}.exe` : BINARY_NAME;
  const targetPath = path.join(LOCAL_DIR, targetName);
  try {
    const tag = `v${version}`;
    const api = `https://api.github.com/repos/${owner}/${repo}/releases/tags/${tag}`;
    const { data } = await axios.get(api, { headers: { 'User-Agent': 'logoscope-npm' } });
    const asset = findAsset(data.assets, osInfo, archInfo);
    if (!asset) throw new Error('No matching asset found');
    const tmp = path.join(os.tmpdir(), asset.name);
    await downloadTo(asset.browser_download_url || asset.url, tmp);
    if (asset.name.endsWith('.tar.gz') || asset.name.endsWith('.tgz')) {
      await tar.x({ file: tmp, cwd: LOCAL_DIR });
    } else {
      await fs.copyFile(tmp, targetPath);
      await fs.chmod(targetPath, 0o755);
    }
    return targetPath;
  } catch (e) {
    console.warn('[logoscope] download error:', e.message, '— using placeholder path');
    return targetPath;
  }
}

function findAsset(assets, osInfo, archInfo) {
  if (!assets) return null;
  let best = null, bestScore = -1;
  for (const a of assets) {
    const n = a.name || '';
    if (n.endsWith('.sha256') || n.endsWith('.md5') || n.endsWith('.asc')) continue;
    let s = 0;
    if (n.includes(osInfo.type)) s += 10; else if (osInfo.keywords.some(k => n.includes(k))) s += 8;
    if (archInfo.keywords.some(k => n.includes(k))) s += 5;
    if (n.startsWith(`${BINARY_NAME}-`)) s += 3;
    if (s > bestScore) { bestScore = s; best = a; }
  }
  return best;
}

async function downloadTo(url, dest) {
  const resp = await axios.get(url, { responseType: 'stream', headers: { 'User-Agent': 'logoscope-npm' } });
  await fs.ensureDir(path.dirname(dest));
  await new Promise((resolve, reject) => {
    const w = fs.createWriteStream(dest);
    resp.data.pipe(w);
    w.on('finish', resolve);
    w.on('error', reject);
  });
}
