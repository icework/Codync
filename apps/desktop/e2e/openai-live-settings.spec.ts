import { test, expect } from '@playwright/test'
import { attach, openGeneralSettings, setPref } from './sandbox.ts'

// Provider metadata is a fixture: this checks the real settings UI without a key or a paid call.
test('GPT-Live settings verify model access and preserve the Realtime default', async () => {
  const { browser, page } = await attach()
  try {
    await page.evaluate(async () => {
      const modulePath = '/client/host-client.ts'
      const { HostClient } = await import(modulePath)
      const original = HostClient.prototype.call
      const state = { requests: [] as { method: string; body: Record<string, unknown> }[], restore: () => { HostClient.prototype.call = original } }
      ;(window as unknown as { liveSettingsTest: typeof state }).liveSettingsTest = state
      HostClient.prototype.call = function (method: string, body: Record<string, unknown> = {}, timeout?: number) {
        if (method === 'voiceStatus') return Promise.resolve({ supportsLive: true, providers: [{ provider: 'openai', configured: true }] })
        if (method === 'voiceModels') {
          state.requests.push({ method, body })
          return Promise.resolve({ models: ['gpt-live-1', 'gpt-realtime-2.1'], transcribe: [], speech: [], defaults: { realtime: 'gpt-realtime-2.1', transcribe: 'gpt-transcribe', speech: 'gpt-4o-mini-tts' } })
        }
        if (method === 'voiceSession') {
          state.requests.push({ method, body })
          return Promise.resolve({ validated: true })
        }
        return original.call(this, method, body, timeout)
      }
    })
    await setPref(page, 'callEngine', 'openai')
    await setPref(page, 'callMode.openai', 'realtime')
    await setPref(page, 'callModel.openai', null)
    await openGeneralSettings(page)
    await page.getByRole('button', { name: 'Voice chat', exact: true }).click()
    await page.getByRole('button', { name: 'gpt-realtime-2.1', exact: true }).click()
    await page.getByText('gpt-live-1', { exact: true }).click()
    await expect(page.getByText('GPT-Live voice and the Responses tool router are billed separately by OpenAI.')).toBeVisible()
    await page.getByRole('button', { name: 'Test key', exact: true }).click()
    await expect(page.getByText('Model access verified', { exact: true })).toBeVisible()
    const requests = await page.evaluate(() => (window as unknown as { liveSettingsTest: { requests: unknown[] } }).liveSettingsTest.requests)
    expect(requests).toContainEqual({ method: 'voiceModels', body: { provider: 'openai', supportsLive: true } })
    expect(requests).toContainEqual({ method: 'voiceSession', body: { provider: 'openai', model: 'gpt-live-1', voice: 'marin' } })
  } finally {
    await page.evaluate(() => (window as unknown as { liveSettingsTest?: { restore(): void } }).liveSettingsTest?.restore())
    await setPref(page, 'callEngine', 'device')
    await setPref(page, 'callModel.openai', null)
    await page.keyboard.press('Escape')
    await browser.close()
  }
})

test('the host call ends only after received Live transcripts finish forwarding', async () => {
  const { browser, page } = await attach()
  try {
    const result = await page.evaluate(async () => {
      const linkPath = '/views/call/call-link.ts'
      const sessionPath = '/views/call/openai-live-session.ts'
      const { CallLink } = await import(linkPath)
      const { OpenAILiveSession } = await import(sessionPath)
      const order: string[] = []
      let release: (() => void) | undefined
      const link = new CallLink({ endCall: async () => { order.push('endCall') } }, 'fixture-bot', 'fixture-call')
      const session = new OpenAILiveSession({
        startLive: async () => ({ sessionId: 'fixture-session', sdp: 'v=0' }),
        liveEvent: async () => {
          await new Promise<void>((resolve) => { release = resolve })
          order.push('transcript')
          return { events: [] }
        },
        liveUpdate: async () => ({ events: [] }),
      }, () => {}, () => {})
      await session.start('gpt-live-1', 'marin', 'v=0')
      session.receive({ type: 'session.input_transcript.delta', delta: 'last words' })
      await Promise.resolve()
      const closing = session.shutdown()
      link.end(1, closing)
      session.receive({ type: 'session.closed' })
      await new Promise((resolve) => setTimeout(resolve, 0))
      const beforeRelease = [...order]
      release?.()
      await closing
      await new Promise((resolve) => setTimeout(resolve, 0))
      return { beforeRelease, order }
    })
    expect(result).toEqual({ beforeRelease: [], order: ['transcript', 'endCall'] })
  } finally {
    await browser.close()
  }
})
