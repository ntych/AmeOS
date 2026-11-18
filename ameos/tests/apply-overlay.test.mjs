// SPDX-License-Identifier: BSD-2-Clause
// Tests for ameos/scripts/apply-overlay.sh.
// Runs on any POSIX host (or Windows with Git Bash); no FreeBSD required.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const source = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const tmpRoot = fs.realpathSync(os.tmpdir());
const temp = fs.mkdtempSync(path.join(tmpRoot, 'ameos-overlay-test-'));
const repo = path.join(temp, 'repo');
const shell = process.env.AMEOS_SH || (process.platform === 'win32'
  ? path.join(process.env.ProgramFiles || 'C:\\Program Files', 'Git/bin/sh.exe')
  : 'sh');
const posix = v => process.platform === 'win32'
  ? v.replaceAll('\\', '/').replace(/^([A-Za-z]):/, (_, d) => `/${d.toLowerCase()}`)
  : v;
let passed = 0;

function run(args, options = {}) {
  const env = { ...process.env, ...options.env };
  if (process.platform === 'win32') {
    env.PATH = `/mingw64/bin:/usr/bin:/bin:${env.PATH}`;
  }
  const result = spawnSync(shell, args, {
    cwd: repo, encoding: 'utf8', timeout: 15000, ...options, env,
  });
  if (result.error) throw result.error;
  assert.equal(result.signal, null, `process interrupted: ${result.stderr}`);
  return result;
}

function git(...args) {
  const result = spawnSync('git', [
    '-c', 'core.autocrlf=false', '-c', 'core.hooksPath=/dev/null',
    '-c', 'commit.gpgsign=false', '-c', 'user.name=AmeOS Tests',
    '-c', 'user.email=tests@example.invalid', ...args,
  ], { cwd: repo, encoding: 'utf8' });
  assert.equal(result.status, 0, result.stderr);
  return result.stdout.trim();
}

function write(rel, content) {
  const target = path.join(repo, rel);
  fs.mkdirSync(path.dirname(target), { recursive: true });
  fs.writeFileSync(target, content);
}

function check(name, fn) {
  fn();
  passed += 1;
  console.log(`ok ${passed} - ${name}`);
}

let script = posix(path.join(source, 'scripts/apply-overlay.sh'));

try {
  // ── Set up a minimal fixture repository ───────────────────────────────────
  fs.mkdirSync(repo);
  git('init', '-q');
  write('sys/conf/newvers.sh', 'TYPE="FreeBSD"\nREVISION="16.0"\nBRANCH="CURRENT"\n');
  write('ameos/profiles/v0.2-b-amd64.conf',
    'AMEOS_VERSION="0.2-b"\nTARGET="amd64"\nTARGET_ARCH="amd64"\nKERNCONF="GENERIC"\n');
  // Minimal overlay with one token-bearing file and the meta README
  write('ameos/overlay/README.md', '# meta — not installed\n');
  write('ameos/overlay/etc/os-release',
    'NAME="AmeOS"\nVERSION="@AMEOS_VERSION@"\nBUILD_ID="@SOURCE_COMMIT@"\n' +
    'FREEBSD_REVISION="@FREEBSD_REVISION@"\nFREEBSD_BRANCH="@FREEBSD_BRANCH@"\n');
  write('ameos/overlay/boot/loader.conf.local',
    'loader_menu_title="AmeOS @AMEOS_VERSION@"\n');
  git('add', '.');
  git('commit', '-qm', 'fixture');
  const head = git('rev-parse', 'HEAD');

  // Run the script from the fixture repository, not from the source checkout.
  // The script resolves its root relative to its own path and uses HEAD as
  // BUILD_ID, so this makes the expected commit and the script's commit agree.
  const fixtureScript = path.join(repo, 'ameos/scripts/apply-overlay.sh');
  fs.copyFileSync(path.join(source, 'scripts/apply-overlay.sh'), fixtureScript);
  fs.chmodSync(fixtureScript, 0o755);
  script = posix(fixtureScript);

  const destdir = path.join(temp, 'dest');

  function invoke(extraArgs = []) {
    const d = path.join(temp, `dest-${passed + 1}`);
    fs.mkdirSync(d, { recursive: true });
    return {
      result: run([script,
        '--profile', 'v0.2-b-amd64',
        '--destdir', posix(d),
        ...extraArgs]),
      destdir: d,
    };
  }

  // ── Syntax check ──────────────────────────────────────────────────────────
  check('POSIX shell syntax is valid', () => {
    const r = run(['-n', script]);
    assert.equal(r.status, 0, r.stderr);
  });

  // ── Help / dry-run ────────────────────────────────────────────────────────
  check('--help prints Usage and exits 0', () => {
    const r = run([script, '--help'], { cwd: repo });
    assert.equal(r.status, 0, r.stderr);
    assert.match(r.stdout, /Usage:/);
  });

  check('--dry-run lists files and writes nothing', () => {
    const { result, destdir: d } = invoke(['--dry-run']);
    assert.equal(result.status, 0, result.stderr);
    assert.match(result.stdout, /would install.*os-release/);
    assert.match(result.stdout, /would install.*loader\.conf\.local/);
    // README.md is excluded
    assert.doesNotMatch(result.stdout, /README/);
    // No files should have been written
    assert.equal(fs.readdirSync(d).length, 0);
  });

  // ── Token substitution ────────────────────────────────────────────────────
  check('tokens are substituted in installed files', () => {
    const { result, destdir: d } = invoke();
    assert.equal(result.status, 0, result.stderr);
    const osRelease = fs.readFileSync(path.join(d, 'etc/os-release'), 'utf8');
    assert.match(osRelease, /VERSION="0\.2-b"/);
    assert.match(osRelease, new RegExp(`BUILD_ID="${head}"`));
    assert.match(osRelease, /FREEBSD_REVISION="16\.0"/);
    assert.match(osRelease, /FREEBSD_BRANCH="CURRENT"/);
    // No raw @TOKEN@ placeholders survive
    assert.doesNotMatch(osRelease, /@\w+@/);
    const loader = fs.readFileSync(path.join(d, 'boot/loader.conf.local'), 'utf8');
    assert.match(loader, /AmeOS 0\.2-b/);
    assert.doesNotMatch(loader, /@\w+@/);
  });

  check('README.md at overlay root is never installed', () => {
    const { result, destdir: d } = invoke();
    assert.equal(result.status, 0, result.stderr);
    assert.equal(fs.existsSync(path.join(d, 'README.md')), false);
  });

  check('output is written atomically (no .tmp leftovers)', () => {
    const { result, destdir: d } = invoke();
    assert.equal(result.status, 0, result.stderr);
    // Recursively collect every file name
    function walk(dir) {
      return fs.readdirSync(dir, { withFileTypes: true }).flatMap(e =>
        e.isDirectory() ? walk(path.join(dir, e.name)) : [e.name]);
    }
    const names = walk(d);
    assert.ok(names.every(n => !n.endsWith('.apply-overlay.tmp')));
  });

  // ── Error paths ───────────────────────────────────────────────────────────
  check('missing --profile is rejected', () => {
    const d = path.join(temp, 'dest-no-profile');
    fs.mkdirSync(d);
    const r = run([script, '--destdir', posix(d)], { cwd: repo });
    assert.notEqual(r.status, 0);
    assert.match(r.stderr, /--profile is required/);
  });

  check('missing --destdir is rejected', () => {
    const r = run([script, '--profile', 'v0.2-b-amd64'], { cwd: repo });
    assert.notEqual(r.status, 0);
    assert.match(r.stderr, /--destdir is required/);
  });

  check('non-existent destdir is rejected', () => {
    const r = run([script,
      '--profile', 'v0.2-b-amd64',
      '--destdir', posix(path.join(temp, 'does-not-exist'))]);
    assert.notEqual(r.status, 0);
    assert.match(r.stderr, /does not exist/);
  });

  check('unknown profile is rejected', () => {
    const d = path.join(temp, 'dest-bad-profile');
    fs.mkdirSync(d);
    const r = run([script, '--profile', 'nonexistent', '--destdir', posix(d)]);
    assert.notEqual(r.status, 0);
    assert.match(r.stderr, /profile not found/);
  });

  check('unknown argument is rejected', () => {
    const d = path.join(temp, 'dest-bad-arg');
    fs.mkdirSync(d);
    const r = run([script, '--profile', 'v0.2-b-amd64',
      '--destdir', posix(d), '--unknown-flag']);
    assert.notEqual(r.status, 0);
    assert.match(r.stderr, /unknown argument/);
  });

  console.log(`\n${passed} checks passed.`);
} finally {
  assert.equal(path.dirname(fs.realpathSync(temp)), tmpRoot);
  assert.ok(path.basename(temp).startsWith('ameos-overlay-test-'));
  fs.rmSync(temp, { recursive: true, force: true });
}
