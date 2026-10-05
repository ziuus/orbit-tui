#!/usr/bin/env node
// Thin shim: exec the native binary that install.js downloaded next to us.
const path = require('path');
const fs = require('fs');
const { spawnSync } = require('child_process');

const isWin = process.platform === 'win32';
const bin = path.join(__dirname, isWin ? 'orbit-bin.exe' : 'orbit-bin');

if (!fs.existsSync(bin)) {
  const installer = path.join(__dirname, '..', 'install.js');
  if (fs.existsSync(installer)) {
    process.stderr.write('orbit: downloading release binary...\n');
    const res = spawnSync(process.execPath, [installer], { stdio: 'inherit' });
    if (res.status !== 0 || !fs.existsSync(bin)) {
      process.exit(1);
    }
  } else {
    console.error('orbit: binary missing — reinstall with `npm install -g @ziuus/orbit-tui`');
    console.error('orbit: or build from source:  cargo install --git https://github.com/ziuus/orbit-tui');
    process.exit(1);
  }
}

const { status, signal } = spawnSync(bin, process.argv.slice(2), { stdio: 'inherit' });
process.exit(signal ? 1 : (status ?? 1));
