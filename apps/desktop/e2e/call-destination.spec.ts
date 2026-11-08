import { test, expect } from '@playwright/test'
import { attach } from './sandbox.ts'

test('waiting call setup never dispatches to a replacement host identity', async () => {
  const { browser, page } = await attach()
  try {
    const result = await page.evaluate(async () => {
      const path = '/views/call/call-link.ts'
      const { CallLink } = await import(path)
      let identity = 'original'
      let release: (() => void) | undefined
      let calls = 0
      const client = { call: async () => { calls++; return {} } }
      const store = { get hostIdentity() { return identity }, mismatch: null,
        ready: () => new Promise((resolve) => { release = () => resolve(client) }) }
      const link = new CallLink(store, 'fixture-bot', 'fixture-call')
      const pending = link.start('device').then(() => false, () => true)
      identity = 'replacement'
      release!()
      return { rejected: await pending, calls }
    })
    expect(result).toEqual({ rejected: true, calls: 0 })
  } finally { await browser.close() }
})

test('a host change during Live capability discovery cannot create the session', async () => {
  const { browser, page } = await attach()
  try {
    const result = await page.evaluate(async () => {
      const path = '/views/call/call-link.ts'
      const { CallLink } = await import(path)
      let identity = 'original'
      const methods: string[] = []
      const store = { get hostIdentity() { return identity }, mismatch: null,
        ready: async () => ({ call: async (method: string) => {
          methods.push(method)
          identity = 'replacement'
          return { supportsLive: true }
        } }) }
      const link = new CallLink(store, 'fixture-bot', 'fixture-call')
      const rejected = await link.startLive('gpt-live-1', 'marin', 'v=0').then(() => false, () => true)
      return { rejected, methods }
    })
    expect(result).toEqual({ rejected: true, methods: ['voiceStatus'] })
  } finally { await browser.close() }
})

test('a queued voice send rechecks its host identity before retrying', async () => {
  const { browser, page } = await attach()
  try {
    const result = await page.evaluate(async () => {
      const storePath = '/store/bot-store.ts'
      const clientPath = '/client/host-client.ts'
      const guardPath = '/views/call/call-destination.ts'
      const { BotStore } = await import(storePath)
      const { HostError } = await import(clientPath)
      const { CallDestination } = await import(guardPath)
      const scope = `call-destination-${crypto.randomUUID()}`
      const store = new BotStore({ id: scope, name: 'Fixture host', signKey: '', urls: [] }, true,
        () => { throw new Error('No network transport is allowed in this fixture') }, '2.39.1', scope)
      let sends = 0
      let ready = 0
      store.hello = { hostId: 'original' }
      store.ready = async () => {
        ready++
        return { send: async () => {
          sends++
          store.hello = { hostId: 'replacement' }
          throw HostError.unreachable()
        } }
      }
      const guard = new CallDestination('original', () => ({ identity: store.hostIdentity, incompatible: false }))
      try {
        store.send('Synthetic voice request', 'fixture-bot', { call: 'fixture-call', validateDestination: guard.check })
        for (let i = 0; i < 40 && (store.entries.get('fixture-bot')?.[0]?.data.status !== 'failed'); i++) {
          await new Promise((resolve) => setTimeout(resolve, 50))
        }
        return { sends, ready, status: store.entries.get('fixture-bot')?.[0]?.data.status }
      } finally { store.retire() }
    })
    expect(result).toEqual({ sends: 1, ready: 2, status: 'failed' })
  } finally { await browser.close() }
})

test('cached online state cannot release call RPCs before the new transport verifies its hello', async () => {
  const { browser, page } = await attach()
  try {
    const result = await page.evaluate(async () => {
      const storePath = '/store/bot-store.ts'
      const linkPath = '/views/call/call-link.ts'
      const { BotStore } = await import(storePath)
      const { CallLink } = await import(linkPath)
      const local = (await window.codync.host.snapshot()).local!
      const hello = await window.codync.host.call(local.baseURL, local.token, 'hello', {}, 10_000) as Record<string, unknown>
      let release: ((value: unknown) => void) | undefined
      let dispatched = 0
      const scope = `verified-call-${crypto.randomUUID()}`
      const transport = {
        call: (method: string) => method === 'hello'
          ? new Promise((resolve) => { release = resolve })
          : (dispatched++, Promise.resolve({})),
        stream: (_path: string, onData: (data: string) => void) => {
          onData(JSON.stringify({ type: 'hello', hostId: 'replacement', rev: 0, usage: [], screen: null }))
          return () => {}
        },
      }
      const store = new BotStore({ id: scope, name: 'Fixture', signKey: '', urls: [] }, false,
        () => transport, '2.39.1', scope)
      store.connection = { kind: 'online' }
      store.hello = { ...hello, hostId: 'original' }
      store.hostId = 'original'
      const link = new CallLink(store, 'fixture-bot', 'fixture-call', 'original')
      store.start()
      let settled = false
      const pending = link.start('device').then(() => false, () => true).finally(() => { settled = true })
      try {
        await new Promise((resolve) => setTimeout(resolve, 30))
        const beforeHello = { settled, dispatched }
        release!({ ...hello, hostId: 'replacement' })
        return { beforeHello, rejected: await pending, dispatched }
      } finally { store.retire() }
    })
    expect(result).toEqual({ beforeHello: { settled: false, dispatched: 0 }, rejected: true, dispatched: 0 })
  } finally { await browser.close() }
})

test('retrying a failed voice message keeps its row and refuses a new unguarded send', async () => {
  const { browser, page } = await attach()
  try {
    const result = await page.evaluate(async () => {
      const path = '/store/bot-store.ts'
      const { BotStore } = await import(path)
      const scope = `voice-retry-${crypto.randomUUID()}`
      const store = new BotStore({ id: scope, name: 'Fixture host', signKey: '', urls: [] }, true,
        () => { throw new Error('No network transport is allowed in this fixture') }, '2.42.1', scope)
      const entry = { id: 'local-voice', seq: 1, botId: 'fixture-bot', rev: 0, kind: 'user', turn: 0,
        data: { text: 'Synthetic voice request', status: 'failed', clientNonce: 'voice', call: 'original-call' },
        createdAt: 1, updatedAt: 1 }
      let sends = 0
      store.send = () => { sends++ }
      store.upsert(entry)
      try {
        store.retry(entry)
        const rows = store.entries.get(entry.botId) ?? []
        return { sends, rows: rows.map((row: typeof entry) => ({ id: row.id, status: row.data.status, call: row.data.call })),
          error: store.lastError }
      } finally { store.retire() }
    })
    expect(result.sends).toBe(0)
    expect(result.rows).toEqual([{ id: 'local-voice', status: 'failed', call: 'original-call' }])
    expect(result.error).toBe('This voice message cannot be retried. Return to the original conversation and send it again.')
  } finally { await browser.close() }
})
