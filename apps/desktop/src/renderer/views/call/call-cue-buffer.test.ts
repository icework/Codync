import { test } from 'node:test'
import assert from 'node:assert/strict'
import { CallCueBuffer } from './call-cue-buffer.ts'
import type { CallCue } from '../../../shared/calls.ts'

const cue = (kind: CallCue['kind'], text: string): CallCue => ({ call: 'call', seq: 1, kind, text })

test('replies and approvals survive engine handoff; progress is coalesced', () => {
  const seen: string[] = []
  const first = (c: CallCue) => seen.push(`first:${c.text}`)
  const second = (c: CallCue) => seen.push(`second:${c.text}`)
  const buffer = new CallCueBuffer(() => assert.fail('overflow'))
  buffer.activate(first)
  buffer.receive(cue('reply', 'before'))
  buffer.pause(first)
  buffer.receive(cue('progress', 'old progress'))
  buffer.receive(cue('notice', 'approval'))
  buffer.receive(cue('progress', 'new progress'))
  buffer.receive(cue('reply', 'result'))
  buffer.activate(second)
  buffer.pause(first) // A late callback from the retired engine cannot pause its replacement.
  buffer.receive(cue('reply', 'after'))
  assert.deepEqual(seen, ['first:before', 'second:approval', 'second:new progress', 'second:result', 'second:after'])
  buffer.stop()
  buffer.receive(cue('reply', 'late'))
  buffer.activate(first)
  assert.equal(seen.length, 5)
})

test('an engine that fails while draining leaves later cues for its replacement', () => {
  const seen: string[] = []
  const buffer = new CallCueBuffer(() => assert.fail('overflow'))
  buffer.receive(cue('reply', 'one'))
  buffer.receive(cue('notice', 'two'))
  const first = (c: CallCue) => { seen.push(c.text); buffer.pause(first) }
  buffer.activate(first)
  assert.deepEqual(seen, ['one'])
  buffer.activate((c) => seen.push(c.text))
  assert.deepEqual(seen, ['one', 'two'])
})

test('bounded overflow is reported instead of silently losing an unlimited queue', () => {
  let overflow = 0
  const buffer = new CallCueBuffer(() => overflow++, 2)
  buffer.receive(cue('reply', 'one'))
  buffer.receive(cue('reply', 'two'))
  buffer.receive(cue('reply', 'three'))
  buffer.receive(cue('reply', 'four'))
  assert.equal(overflow, 1)
})
