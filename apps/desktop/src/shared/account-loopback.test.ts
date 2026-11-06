import assert from 'node:assert/strict'
import { EventEmitter } from 'node:events'
import path from 'node:path'
import { test } from 'node:test'
import { AuthenticationEnded, BrowserAuthentication } from '../main/browser-authentication.ts'
import { loadMainModule } from './test-support/main-module.ts'

const until = async (condition: () => boolean) => {
  for (let i = 0; i < 100 && !condition(); i++) await new Promise<void>(resolve => setImmediate(resolve))
  assert.ok(condition())
}

function harness(fail: 'create' | 'prepare' | 'browser' | null = null, timeout = 1000) {
  const returns: { url: string; closed: number; receive: (url: string) => void }[] = []
  const redirects: string[] = []
  let opened = 0
  let attempts = 0
  const browser = new BrowserAuthentication(async () => { opened++; if (fail === 'browser') throw new Error('browser unavailable') }, timeout)
  const reply = (body: unknown, status = 200) => new Response(JSON.stringify(body), { status })
  const active = { last_active_session_id: 's', sessions: [{ id: 's', status: 'active', user: { id: 'fixture-user' } }] }
  const { AccountService } = loadMainModule<typeof import('../main/account.ts')>('account', {
    'node:events': { EventEmitter }, 'node:fs': { promises: {} }, 'node:path': path,
    electron: { app: { isPackaged: false, getVersion: () => 'test' }, ipcMain: {}, safeStorage: {} },
    './auth': { authenticate: browser.authenticate.bind(browser), AuthenticationEnded, handleURL: browser.handleURL.bind(browser) },
    './account-return': { loopbackAccountReturn: async (receive: (url: string) => void) => {
      const entry = { url: `http://127.0.0.1:4444/fixture-${returns.length}`, closed: 0, receive }
      returns.push(entry)
      return { url: entry.url, scheme: 'http', close: () => { entry.closed++ } }
    } },
    './cloud': {},
    './environment': { identity: { appId: 'test.codync', environment: 'dev' }, buildConfig: {
      clerkPublishableKey: `pk_test_${Buffer.from('clerk.example.test$').toString('base64')}`, cloudURL: 'https://cloud.example.test',
    } },
  }, {
    URL, URLSearchParams, Buffer, process: { env: {}, platform: 'darwin' },
    fetch: async (input: string, init: RequestInit) => {
      const url = new URL(input)
      if (init.method === 'POST') redirects.push(new URLSearchParams(String(init.body)).get('redirect_url')!)
      if (url.pathname.endsWith('/sign_ins')) {
        if (fail === 'create') throw new Error('create unavailable')
        return reply({ response: { id: `login-${++attempts}` } })
      }
      if (url.pathname.endsWith('/prepare_first_factor')) {
        if (fail === 'prepare') throw new Error('prepare unavailable')
        return reply({ response: { first_factor_verification: { external_verification_redirect_url: 'https://provider.example.test/login' } } })
      }
      if (url.pathname.includes('/sign_ins/')) {
        if (url.searchParams.get('rotating_token_nonce') !== 'current') return reply({ errors: [{ code: 'signed_out' }] }, 401)
        return reply({ response: { id: `login-${attempts}`, status: 'complete' } })
      }
      if (url.pathname === '/v1/client') return reply({ response: active })
      throw new Error('unexpected request')
    },
  })
  return { account: new AccountService(), returns, redirects, opened: () => opened }
}

test('unpackaged macOS uses one return for both requests, validates the nonce and closes on success', async () => {
  const h = harness()
  const user = () => h.account.state.user
  const login = h.account.signIn('google')
  await until(() => h.opened() === 1)
  assert.deepEqual(h.redirects, [h.returns[0]!.url, h.returns[0]!.url])
  h.returns[0]!.receive(h.returns[0]!.url + '?rotating_token_nonce=stale')
  await new Promise<void>(resolve => setImmediate(resolve))
  assert.equal(user(), null)
  assert.equal(h.returns[0]!.closed, 0)
  h.returns[0]!.receive(h.returns[0]!.url + '?rotating_token_nonce=current')
  assert.equal(await login, null)
  assert.equal(user()?.userId, 'fixture-user')
  assert.equal(h.returns[0]!.closed, 1)
})

test('unpackaged macOS closes listeners after API/browser failure and timeout', async () => {
  for (const failure of ['create', 'prepare', 'browser'] as const) {
    const h = harness(failure)
    assert.equal(await h.account.signIn('google'), 'error')
    assert.equal(h.returns[0]!.closed, 1)
    assert.equal(h.account.state.busy, false)
  }
  const h = harness(null, 20)
  assert.equal(await h.account.signIn('google'), 'timeout')
  assert.equal(h.returns[0]!.closed, 1)
})

test('replacing a macOS login closes only the old listener', async () => {
  const h = harness()
  const first = h.account.signIn('google')
  await until(() => h.opened() === 1)
  const second = h.account.signIn('google')
  await until(() => h.opened() === 2)
  assert.equal(await first, 'cancelled')
  assert.deepEqual(h.returns.map(r => r.closed), [1, 0])
  h.returns[1]!.receive(h.returns[1]!.url + '?rotating_token_nonce=current')
  assert.equal(await second, null)
  assert.deepEqual(h.returns.map(r => r.closed), [1, 1])
})
