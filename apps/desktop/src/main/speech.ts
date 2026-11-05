import { execFileSync, spawn, type ChildProcessWithoutNullStreams } from 'node:child_process'
import { existsSync } from 'node:fs'
import { join } from 'node:path'
import { app, ipcMain, webContents, type WebContents } from 'electron'
import { CallAudioOwner } from '../shared/call-audio-owner'
import type { SpeechEvent } from '../shared/ipc'

// On-device speech recognition for calls: a small Swift helper (native/speech-macos) runs
// SFSpeechRecognizer on the microphone and prints JSON lines. macOS only.

function helperPath() {
  const candidates = [
    app.isPackaged ? join(process.resourcesPath, 'codync-speech') : null,
    join(__dirname, '../../native/speech-macos/.build/release/codync-speech'),
    join(__dirname, '../../native/speech-macos/.build/debug/codync-speech'),
  ]
  return candidates.find((c): c is string => !!c && existsSync(c)) ?? null
}

/** Whether this build can transcribe on device. */
export const speechAvailable = () => process.platform === 'darwin' && helperPath() !== null

let child: ChildProcessWithoutNullStreams | null = null
let listener: WebContents | null = null
/** Bumped per start: a stopping process may still deliver its final result, an older one may not. */
let generation = 0
const observed = new WeakSet<WebContents>()
const callOwner = new CallAudioOwner((active) => {
  for (const contents of webContents.getAllWebContents()) {
    if (!contents.isDestroyed()) contents.send('speech:callActive', active)
  }
})
export const callAudioActive = () => callOwner.active

/** Cancels capture without delivering late text or a stopped event to its next owner. */
function cancelCapture() {
  generation++
  stop()
  listener = null
}

function released(contents: WebContents, id: number) {
  if (listener === contents) cancelCapture()
  callOwner.setCalling(id, false)
}

function observe(contents: WebContents) {
  if (observed.has(contents)) return
  observed.add(contents)
  const id = contents.id
  contents.once('destroyed', () => released(contents, id))
}

function emit(event: SpeechEvent) {
  if (listener && !listener.isDestroyed()) listener.send('speech:event', event)
}

function start(sender: WebContents, locale: string | null) {
  observe(sender)
  if (!callOwner.permitsSpeech(sender.id)) {
    sender.send('speech:event', { type: 'error', message: 'A voice call is using the microphone.' } satisfies SpeechEvent)
    return
  }
  stop()
  // Another window's dictation or call loses the helper, and its exit (an older generation) won't say so.
  if (listener && listener !== sender) emit({ type: 'stopped' })
  listener = sender
  const path = helperPath()
  if (!path) return emit({ type: 'error', message: "On-device speech isn't available in this build." })
  const proc = spawn(path, locale ? ['--locale', locale] : [], { stdio: 'pipe' })
  const mine = ++generation
  child = proc
  let buffer = ''
  proc.stdout.on('data', (chunk: Buffer) => {
    buffer += chunk.toString('utf8')
    let nl: number
    while ((nl = buffer.indexOf('\n')) >= 0) {
      const line = buffer.slice(0, nl)
      buffer = buffer.slice(nl + 1)
      try {
        if (generation === mine) emit(JSON.parse(line) as SpeechEvent)
      } catch {}
    }
  })
  proc.on('exit', () => {
    if (child === proc) child = null
    if (generation === mine) emit({ type: 'stopped' })
  })
  proc.on('error', (error) => { if (generation === mine) emit({ type: 'error', message: error.message }) })
}

function stop() {
  const proc = child
  child = null
  if (proc && !proc.killed) proc.kill('SIGTERM')
}

let supported: string[] | null = null

/** The languages the recognizer can transcribe (identifiers like `zh-TW`), asked once. */
function locales() {
  if (supported) return supported
  const path = helperPath()
  try {
    supported = path ? (JSON.parse(execFileSync(path, ['--locales'], { encoding: 'utf8', timeout: 5000 })) as string[]) : []
  } catch {
    supported = []
  }
  return supported
}

export function registerSpeech() {
  ipcMain.on('speech:callActive', (e) => (e.returnValue = callOwner.active))
  ipcMain.on('voice:calling', (e, on: boolean) => {
    if (typeof on !== 'boolean') return
    observe(e.sender)
    if (callOwner.setCalling(e.sender.id, on) && on && listener && listener !== e.sender) cancelCapture()
  })
  app.on('render-process-gone', (_e, contents) => released(contents, contents.id))
  ipcMain.on('speech:available', (e) => (e.returnValue = speechAvailable()))
  ipcMain.on('speech:locales', (e) => (e.returnValue = locales()))
  ipcMain.on('speech:start', (e, locale: string | null) => start(e.sender, locale))
  // Only the window using the helper stops it: one that lost it to another window has nothing to stop.
  ipcMain.on('speech:stop', (e) => {
    if (e.sender === listener) stop()
  })
  app.on('before-quit', stop)
}
