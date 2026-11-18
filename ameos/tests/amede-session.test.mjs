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
  return spawnSync(shell, args, { encoding: 'utf8', timeout: 15000, ...options, env });
}

try {
  const checker = posix(path.join(source, 'scripts/check-amede-session.sh'));
  const launcher = path.join(tmp, 'amede-session');
  const fakeWayfire = path.join(tmp, 'wayfire');
  const config = path.join(tmp, 'wayfire.ini');
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
  const result = run([posix(launcher)], { cwd: tmp, env });
  assert.equal(result.status, 0, result.stderr);
  assert.equal(fs.readFileSync(log, 'utf8'), `-c\n${posix(config)}\n`);
  assert.equal(fs.statSync(runtime).mode & 0o777, 0o700);

  // Note: amede-session's error handling (missing wayfire check) is validated
  // by check-amede-session.sh on all platforms. We do not test error exit codes
  // here because shell behavior for missing commands varies across CI runners.

  console.log('check-amede-session: runtime smoke passed');
} finally {
  fs.rmSync(tmp, { recursive: true, force: true });
}
