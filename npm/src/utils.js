import fs from 'fs-extra';
import os from 'os';

export function detectOsArch() {
  const osType = os.platform();
  const archType = os.arch();
  let osInfo, archInfo;
  switch (osType) {
    case 'linux': osInfo = { type: 'linux', keywords: ['linux','Linux','gnu'] }; break;
    case 'darwin': osInfo = { type: 'darwin', keywords: ['darwin','Darwin','mac','Mac','apple','Apple','osx','OSX'] }; break;
    case 'win32': osInfo = { type: 'windows', keywords: ['windows','Windows','msvc','pc-windows'] }; break;
    default: throw new Error(`Unsupported OS: ${osType}`);
  }
  switch (archType) {
    case 'x64': archInfo = { type: 'x86_64', keywords: ['x86_64','amd64','x64'] }; break;
    case 'arm64': archInfo = { type: 'aarch64', keywords: ['arm64','aarch64'] }; break;
    default: throw new Error(`Unsupported arch: ${archType}`);
  }
  return { os: osInfo, arch: archInfo };
}

export async function ensureBinDirectory(binDir) {
  await fs.ensureDir(binDir);
  return binDir;
}

