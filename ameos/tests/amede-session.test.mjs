// SPDX-License-Identifier: BSD-2-Clause
// Smoke-test the AmeDE session launcher without Wayfire or a GPU.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const source = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const tmp = fs.mkdtempSync(path.join(fs.realpathSync(os.tmpdir()), 'ameos-session-test-'));
const shell = process.env.AMEOS_SH || (process.platform === 'win32'
  ? path.join(process.env.ProgramFiles || 'C:\\Program Files', 'Git/bin/sh.exe')
  : 'sh');
const posix = value => process.platform === 'win32'
  ? value.replaceAll('\\', '/').replace(/^([A-Za-z]):/, (_, drive) => `/${drive.toLowerCase()}`)
  : value;

function run(args, options = {}) {
  const env = { ...process.env, ...options.env };
  if (process.platform === 'win32') env.PATH = `/mingw64/bin:/usr/bin:/bin:${env.PATH}`;
  const result = spawnSync(shell, args, { encoding: 'utf8', timeout: 15000, ...options, env });
  if (result.error) throw result.error;
  assert.equal(result.signal, null, `shell interrupted: ${result.stderr}`);
  return result;
}

try {
  const checker = posix(path.join(source, 'scripts/check-amede-session.sh'));
  const launcher = path.join(tmp, 'amede-session');
  const fakeWayfire = path.join(tmp, 'wayfire');
  const config = path.join(tmp, 'explicit config.ini');
  const runtime = path.join(tmp, 'runtime');
  const log = path.join(tmp, 'wayfire-args.log');

  // Static checks catch bad overlay edits before any runtime test.
  const check = run([checker], { cwd: source });
  assert.equal(check.status, 0, check.stderr);

  fs.copyFileSync(path.join(source, 'overlay/usr/local/bin/amede-session'), launcher);
  fs.chmodSync(launcher, 0o755);
  fs.writeFileSync(config, '[core]\nplugins = autostart\n');
  fs.writeFileSync(fakeWayfire, `#!/bin/sh
printf '%s\\n' "$@" > "${posix(log)}"
exit 0
`);
  fs.chmodSync(fakeWayfire, 0o755);

  const env = {
    HOME: tmp,
    PATH: `${posix(tmp)}:/usr/bin:/bin`,
    XDG_RUNTIME_DIR: posix(runtime),
    WAYFIRE_CONFIG_FILE: posix(config),
  };
  const userConfigDir = path.join(tmp, 'config', 'wayfire');
  fs.mkdirSync(userConfigDir, { recursive: true });
  const userConfig = path.join(userConfigDir, 'wayfire.ini');
  const legacyConfig = path.join(userConfigDir, 'wayfire-amede.ini');
  fs.writeFileSync(userConfig, '[core]\nplugins = autostart\n');
  fs.writeFileSync(legacyConfig, '[core]\nplugins = autostart\n');
  env.XDG_CONFIG_HOME = posix(path.dirname(userConfigDir));

  function expectConfig(overrides, expected) {
    const result = run([posix(launcher)], { cwd: tmp, env: { ...env, ...overrides } });
    assert.equal(result.status, 0, result.stderr);
    assert.equal(fs.readFileSync(log, 'utf8'), `-c\n${posix(expected)}\n`);
  }
  expectConfig({}, config); // explicit override wins, including paths with spaces
  assert.equal(fs.statSync(runtime).mode & 0o777, 0o700);
  expectConfig({ WAYFIRE_CONFIG_FILE: '' }, userConfig);
  fs.unlinkSync(userConfig);
  expectConfig({ WAYFIRE_CONFIG_FILE: '' }, legacyConfig);

  fs.unlinkSync(log);
  const missingConfig = path.join(tmp, 'missing config.ini');
  const invalid = run([posix(launcher)], {
    cwd: tmp, env: { ...env, WAYFIRE_CONFIG_FILE: posix(missingConfig) },
  });
  assert.equal(invalid.status, 1, invalid.stderr);
  assert.match(invalid.stderr, /wayfire config not found/);
  assert.ok(invalid.stderr.includes(posix(missingConfig)));
  assert.equal(fs.existsSync(log), false, 'Wayfire must not run with an invalid override');

  // Start the shell with its normal PATH, then restrict only the launcher's
  // PATH inside that shell. Clearing spawnSync's PATH would prevent Node from
  // finding `sh` itself (ENOENT/status=null), not exercise the launcher.
  // Keep runtime setup tools available, but deliberately provide no wayfire.
  const tools = path.join(tmp, 'runtime-tools');
  fs.mkdirSync(tools);
  for (const tool of ['mkdir', 'chmod']) {
    fs.writeFileSync(path.join(tools, tool), `#!/bin/sh\nexec /bin/${tool} "$@"\n`);
    fs.chmodSync(path.join(tools, tool), 0o755);
  }
  const missing = run([
    '-c', 'PATH="$1"; export PATH; . "$2"',
    'session-error-test', posix(tools), posix(launcher),
  ], { cwd: tmp, env });
  assert.equal(missing.status, 127, missing.stderr);
  assert.match(missing.stderr, /wayfire not found/);

  console.log('check-amede-session: runtime smoke passed');
} finally {
  fs.rmSync(tmp, { recursive: true, force: true });
}
