import { test, expect } from '@playwright/test'
import { api, attach, sandbox, typeMessage } from './sandbox.ts'
import { DatabaseSync } from 'node:sqlite'
import { join, resolve } from 'node:path'
import { homedir } from 'node:os'
import { SANDBOX_PREFIX } from '../src/shared/dev-sandbox.ts'

test('Codex smart routing persists and keeps the manual model as fallback', async () => {
  const { browser, page } = await attach()
  const name = `Smart routing e2e ${Date.now()}`
  let id: string | undefined
  try {
    const voice = await api<{ providers: { provider: string; configured: boolean }[] }>(page, 'voiceStatus', {})
    expect(voice.providers.some((p) => p.provider === 'openai' && p.configured), 'This fixture requires a sandbox without real API keys').toBe(false)
    const fixture = resolve(__dirname, '../../../host/tests/fixtures/routing_agent.py')
    const result = await api<{ bot: { id: string } }>(page, 'createBot', {
      name, backend: 'codex', command: `python3 -u '${fixture}'`, model: 'gpt-6.1-sol', modelOptions: { reasoning_effort: 'low' },
      smartRouting: false, fastMode: true, connectors: [], skills: [], notify: false,
    })
    id = result.bot.id
    await page.locator('.roster-row', { hasText: name }).click()
    await page.getByRole('button', { name: `${name}, view details` }).click()
    const details = page.locator('.details-panel')
    const picker = details.locator('.agent-picker')
    await picker.locator('.agent-picker-summary').click()
    const toggle = details.getByRole('switch', { name: /^Smart routing\b/ })
    await expect(toggle).not.toBeChecked()
    await toggle.click()
    await expect(toggle).toBeChecked()
    await expect(picker.locator('.agent-picker-summary [title="Fast mode"]')).toHaveCount(0)
    await expect.poll(() => {
      const db = new DatabaseSync(join(homedir(), SANDBOX_PREFIX + sandbox().name, 'codync.db'), { readOnly: true })
      try {
        const row = db.prepare('SELECT config FROM bots WHERE id = ?').get(id!)
        return JSON.parse(String(row?.config))
      } finally { db.close() }
    }).toMatchObject({ smartRouting: true, model: 'gpt-6.1-sol', fastMode: true })
    await page.reload()
    await page.locator('.roster-row', { hasText: name }).click()
    await page.getByRole('button', { name: `${name}, view details` }).click()
    await page.locator('.details-panel .agent-picker-summary').click()
    await expect(page.locator('.details-panel').getByRole('switch', { name: /^Smart routing\b/ })).toBeChecked()
    // The ordinary editor clears custom launch commands for a named agent. Reinstall the
    // deterministic adapter after verifying the actual UI save, before sending any task.
    await api(page, 'updateBot', { id, command: `python3 -u '${fixture}'` })
    await typeMessage(page, 'Routing smoke test')
    await expect(page.getByRole('log', { name: 'Messages' }).getByText('gpt-6.1-sol low off', { exact: true })).toBeVisible({ timeout: 20_000 })
    await expect(page.getByRole('log', { name: 'Messages' }).getByText(/Smart routing unavailable.*missing_api_key/)).toBeVisible()
    await page.screenshot({ path: test.info().outputPath('smart-routing.png') })
  } finally {
    if (id) await api(page, 'deleteBot', { botId: id })
    await browser.close()
  }
})
