// Downloads the prebuilt orbit binary for this platform from the matching
// GitHub release. ponytail: linux-x64 only — orbit reads /proc and /sys and
// links libdbus, so there is nothing to ship for other platforms. Add targets
// here and in .github/workflows/release.yml when someone asks for them.
const fs = require('fs');
const os = require('os');
const path = require('path');
const https = require('https');
const { execFileSync } = require('child_process');

const { version } = require('./package.json');
const TARGET_MAP = {
  'linux': { 'x64': 'x86_64-unknown-linux-gnu' },
  'darwin': {
    'x64': 'x86_64-apple-darwin',
    'arm64': 'aarch64-apple-darwin'
  },
  'win32': { 'x64': 'x86_64-pc-windows-msvc' }
};

const TARGET = TARGET_MAP[process.platform]?.[process.arch];
const FROM_SOURCE = 'cargo install --git https://github.com/ziuus/orbit-tui';

if (!TARGET) {
  console.error(`orbit: no prebuilt binary for ${process.platform}-${process.arch}.`);
  console.error(`orbit: build it yourself with:  ${FROM_SOURCE}`);
  process.exit(1);
}

const isWin = process.platform === 'win32';
const ASSET = `orbit-${TARGET}.tar.gz`;
const URL = `https://github.com/ziuus/orbit-tui/releases/download/v${version}/${ASSET}`;

// GitHub redirects release downloads to a CDN host, so follow Location.
function download(url, hops = 0, retries = 3) {
  return new Promise((resolve, reject) => {
    if (hops > 5) return reject(new Error('too many redirects'));
    const req = https
      .get(url, { headers: { 'user-agent': `@ziuus/orbit-tui/${version}` } }, (res) => {
        const { statusCode, headers } = res;
        if (statusCode >= 300 && statusCode < 400 && headers.location) {
          res.resume();
          return download(headers.location, hops + 1, retries).then(resolve, reject);
        }
        if (statusCode !== 200) {
          res.resume();
          return reject(new Error(`HTTP ${statusCode} for ${url}`));
        }
        const total = parseInt(headers['content-length'] || '0', 10);
        let downloaded = 0;
        const chunks = [];
        res.on('data', (c) => {
          chunks.push(c);
          downloaded += c.length;
          if (total > 0 && process.stderr.isTTY) {
            const pct = Math.round((downloaded / total) * 100);
            const mb = (downloaded / (1024 * 1024)).toFixed(1);
            const totMb = (total / (1024 * 1024)).toFixed(1);
            process.stderr.write(`\rorbit: downloading... ${pct}% (${mb}/${totMb} MB)`);
          }
        });
        res.on('end', () => {
          if (process.stderr.isTTY) process.stderr.write('\n');
          resolve(Buffer.concat(chunks));
        });
        res.on('error', (err) => {
          if (retries > 0) {
            process.stderr.write(`\norbit: download error (${err.message}), retrying...\n`);
            resolve(download(url, hops, retries - 1));
          } else {
            reject(err);
          }
        });
      })
      .on('error', (err) => {
        if (retries > 0) {
          process.stderr.write(`\norbit: download error (${err.message}), retrying...\n`);
          resolve(download(url, hops, retries - 1));
        } else {
          reject(err);
        }
      });
  });
}

(async () => {
  const binDir = path.join(__dirname, 'bin');
  const dest = path.join(binDir, isWin ? 'orbit-bin.exe' : 'orbit-bin');
  const tgz = path.join(os.tmpdir(), `orbit-${version}-${process.pid}.tar.gz`);

  fs.mkdirSync(binDir, { recursive: true });
  process.stderr.write(`orbit: fetching ${ASSET}\n`);
  fs.writeFileSync(tgz, await download(URL));

  try {
    const binName = isWin ? 'orbit.exe' : 'orbit';
    execFileSync('tar', ['-xzf', tgz, '-C', binDir, binName]);
    fs.renameSync(path.join(binDir, binName), dest);
    fs.chmodSync(dest, 0o755);
  } finally {
    fs.rmSync(tgz, { force: true });
  }

  // Fail the install rather than leave a bin shim pointing at nothing.
  execFileSync(dest, ['--version'], { stdio: 'ignore' });
  process.stderr.write('orbit: installed — run `orbit`\n');
})().catch((err) => {
  console.error(`orbit: install failed — ${err.message}`);
  console.error(`orbit: build from source instead:  ${FROM_SOURCE}`);
  process.exit(1);
});
