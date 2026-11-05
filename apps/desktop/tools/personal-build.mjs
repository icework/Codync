// Production identity and cloud, native architecture, no upstream auto-updater.
// CODYNC_SIGNING_IDENTITY must name a valid local Apple signing identity.
import { execFileSync } from 'node:child_process'
import { copyFileSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

if (process.platform !== 'darwin') throw new Error('Personal packaging is macOS only')
const identity = process.env.CODYNC_SIGNING_IDENTITY
if (!identity) throw new Error('Set CODYNC_SIGNING_IDENTITY to a local Apple signing identity')
const desktop = join(dirname(fileURLToPath(import.meta.url)), '..')
const repo = join(desktop, '../..')
const host = join(repo, 'host')
const derived = join(repo, 'build/dd')
const native = join(desktop, 'build/native')
const run = (bin, args, cwd = desktop, extraEnv = {}) => execFileSync(bin, args, {
  cwd, stdio: 'inherit', env: { ...process.env, ...extraEnv },
})
const tool = (name) => execFileSync('rustup', ['which', name], { cwd: host, encoding: 'utf8' }).trim()
const target = process.arch === 'arm64' ? 'aarch64-apple-darwin' : 'x86_64-apple-darwin'
mkdirSync(native, { recursive: true })
run(tool('cargo'), ['build', '--locked', '--target-dir', join(host, 'target'), '--profile', 'dev-app', '--target', target], host, {
  RUSTC: tool('rustc'), CODYNC_ENV: 'main', CODYNC_UPDATE_CHANNEL: 'personal',
})
copyFileSync(join(host, 'target', target, 'dev-app/codync-host'), join(native, 'codync-host'))
run(process.execPath, ['tools/native-screen.mjs', 'main'])
run('swift', ['build', '-c', 'release', '--package-path', 'native/speech-macos', '--scratch-path', join(derived, 'speech-macos')])
copyFileSync(join(derived, 'speech-macos/release/codync-speech'), join(native, 'codync-speech'))
run(process.execPath, ['tools/account-config.mjs', 'main'])
const configPath = join(desktop, 'resources/account-config.json')
const config = JSON.parse(readFileSync(configPath, 'utf8'))
writeFileSync(configPath, JSON.stringify({ ...config, updateChannel: 'personal' }, null, 2) + '\n')
run('npm', ['run', 'build'])
run('npm', ['exec', 'electron-builder', '--', '--mac', '--config', 'tools/personal-builder.cjs', `--config.mac.identity=${identity}`])
