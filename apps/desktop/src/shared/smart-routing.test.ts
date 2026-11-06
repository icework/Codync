import { test } from 'node:test'
import assert from 'node:assert/strict'
import { normalizeBot, draftOf } from './models.ts'
import { withBackend } from './fallbacks.ts'
import { fastModeShown } from './fast-mode.ts'

test('routing survives editor saves and ordinary edits without changing manual defaults', () => {
  const bot = normalizeBot({ id: 'b', backend: 'codex', smartRouting: true, model: 'manual', fastMode: true })
  const draft = draftOf(bot)
  assert.equal(draft.smartRouting, true)
  assert.equal(draft.model, 'manual')
  assert.equal(draft.fastMode, true)
  assert.equal(fastModeShown(bot), false)
  assert.equal(withBackend(draft, 'claude').smartRouting, false)
  assert.equal(withBackend(draft, 'codex').smartRouting, true)
})

test('an older host leaves routing unknown and omitted from an editor save', () => {
  const bot = normalizeBot({ id: 'old' })
  assert.equal(bot.smartRouting, null)
  assert.equal(draftOf(bot).smartRouting, undefined)
})
