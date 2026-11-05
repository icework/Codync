import { test, expect } from '@playwright/test'
import { randomUUID } from 'node:crypto'
import { readFileSync } from 'node:fs'
import { homedir, tmpdir } from 'node:os'
import { join } from 'node:path'
import { api, attach, sandbox, setPref } from './sandbox.ts'

// Exercises the actual desktop view/host path; no provider key or bot turn is used.
test('a call survives navigation and stops on hangup or destination deletion', async () => {
  const { browser, page } = await attach()
  const names = [`Voice diagnostic ${randomUUID().slice(0, 8)}`, `Other diagnostic ${randomUUID().slice(0, 8)}`]
  const bots: string[] = []
  const log = () => readFileSync(join(homedir(), `.codync-dev-${sandbox().name}`, 'host.log'), 'utf8').replace(/\x1b\[[0-9;]*m/g, '')
  const before = log().length
  const previous = await page.evaluate(() => localStorage.getItem('callEngine'))
  try {
    await setPref(page, 'callEngine', 'device')
    for (const name of names) {
      const { bot } = await api<{ bot: { id: string } }>(page, 'createBot', {
        name, backend: 'claude', cwd: tmpdir(), connectors: [], skills: [],
      })
      bots.push(bot.id)
    }
    await page.getByText(names[0]!, { exact: true }).first().click()
    await page.getByRole('button', { name: 'Call', exact: true }).click()
    await expect(page.locator('.call-bar')).toBeVisible()
    await expect.poll(() => page.evaluate(() => window.codync.speech.callActive())).toBe(true)
    await page.getByRole('button', { name: 'End call', exact: true }).click()
    await expect(page.locator('.call-bar')).toHaveCount(0)
    await expect.poll(() => page.evaluate(() => window.codync.speech.callActive())).toBe(false)
    await expect.poll(() => log().slice(before)).toMatch(/operation="end_call"[^\n]*end_reason="user"/)
    await page.getByRole('button', { name: 'Call', exact: true }).click()
    await expect(page.locator('.call-bar')).toBeVisible()
    const endsBeforeNavigation = log().slice(before).split('operation="end_call"').length - 1
    const bar = await page.locator('.call-bar').elementHandle()
    await page.getByText(names[1]!, { exact: true }).first().click()
    await expect(page.locator('.call-bar')).toBeVisible()
    expect(await bar!.evaluate((element) => element.isConnected)).toBe(true)
    await expect(page.locator('.call-destination')).toContainText(names[0]!)
    await expect(page.getByRole('button', { name: 'Dictate', exact: true })).toBeDisabled()
    await expect(page.getByRole('button', { name: 'Call', exact: true })).toHaveCount(0)
    expect(log().slice(before).split('operation="end_call"').length - 1).toBe(endsBeforeNavigation)
    await page.getByRole('button', { name: 'Call settings', exact: true }).click()
    await expect(page.locator('.settings-voice-sheet')).toBeVisible()
    await page.keyboard.press('Escape')
    await expect(page.locator('.call-bar')).toBeVisible()
    await page.locator('.call-destination').click()
    await expect(page.locator('.call-destination')).toContainText(names[0]!)
    await page.getByRole('button', { name: 'End call', exact: true }).click()
    await expect(page.locator('.call-bar')).toHaveCount(0)
    await expect.poll(() => log().slice(before).split('operation="end_call"').length - 1).toBe(endsBeforeNavigation + 1)
    expect(log().slice(before)).not.toContain('end_reason="view_unmounted"')
    // Losing the actual destination is different from merely navigating elsewhere.
    await page.getByRole('button', { name: 'Call', exact: true }).click()
    await expect(page.locator('.call-bar')).toBeVisible()
    await api(page, 'deleteBot', { botId: bots[0] })
    await expect(page.locator('.call-bar')).toHaveCount(0)
    await expect.poll(() => log().slice(before)).toContain('end_reason="view_unmounted"')
    await page.getByRole('button', { name: 'OK', exact: true }).click()
    expect(log().slice(before).split('\n').filter((line) => line.includes('codync_host::voice_diagnostics')).join('\n')).not.toContain(names[0])
  } finally {
    for (const botId of bots) await api(page, 'deleteBot', { botId }).catch(() => {})
    await page.evaluate((value) => {
      if (value === null) localStorage.removeItem('callEngine')
      else localStorage.setItem('callEngine', value)
      window.dispatchEvent(new StorageEvent('storage', { key: 'callEngine', newValue: value }))
    }, previous)
    await browser.close()
  }
})
