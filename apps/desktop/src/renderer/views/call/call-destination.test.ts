import { test } from 'node:test'
import assert from 'node:assert/strict'
import { CallDestination } from './call-destination.ts'

test('a call admits its original host but rejects replacement, unknown identity and incompatibility', () => {
  let state: { identity: string | null; incompatible: boolean } = { identity: 'original', incompatible: false }
  const target = new CallDestination('original', () => state)
  target.check()
  state = { identity: 'replacement', incompatible: false }
  assert.throws(target.check, /identity or compatibility/)
  state = { identity: 'original', incompatible: true }
  assert.throws(target.check)
  state = { identity: null, incompatible: false }
  assert.throws(target.check)
  state = { identity: 'original', incompatible: false }
  target.check()
  assert.throws(new CallDestination(null, () => state).check)
})
