import assert from 'node:assert/strict'
import { test } from 'node:test'
import { CallAudioOwner } from './call-audio-owner.ts'

test('a call excludes other renderers from speech and only its owner releases it', () => {
  const changes: boolean[] = []
  const owner = new CallAudioOwner((active) => changes.push(active))
  assert.equal(owner.permitsSpeech(2), true)
  assert.equal(owner.setCalling(1, true), true)
  assert.equal(owner.permitsSpeech(1), true)
  assert.equal(owner.permitsSpeech(2), false)
  assert.equal(owner.setCalling(2, true), false)
  assert.equal(owner.setCalling(2, false), false)
  assert.equal(owner.active, true)
  assert.equal(owner.setCalling(1, true), false, 'duplicate claim emits no new state')
  assert.deepEqual(changes, [true])
  assert.equal(owner.setCalling(1, false), true)
  assert.equal(owner.permitsSpeech(2), true)
  assert.deepEqual(changes, [true, false])
})

test('renderer cleanup is idempotent and cannot clear a replacement owner', () => {
  const changes: boolean[] = []
  const owner = new CallAudioOwner((active) => changes.push(active))
  owner.setCalling(1, true)
  owner.setCalling(1, false) // render-process-gone
  owner.setCalling(2, true)
  owner.setCalling(1, false) // destroyed after its replacement took over
  assert.equal(owner.active, true)
  assert.equal(owner.permitsSpeech(1), false)
  assert.equal(owner.permitsSpeech(2), true)
  assert.deepEqual(changes, [true, false, true])
})
