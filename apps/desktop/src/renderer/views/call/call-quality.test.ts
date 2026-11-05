import assert from 'node:assert/strict'
import { test } from 'node:test'
import { CallQuality } from './call-quality.ts'

test('call quality reports ready time, reconnects, stalls and failure once per engine', () => {
  const events: unknown[] = []
  const quality = new CallQuality((e) => events.push(e))
  quality.begin('gemini')
  quality.phase('listening', null)
  quality.phase('reconnecting', null)
  quality.phase('listening', null)
  quality.stalled()
  quality.phase('failed', 'connection')
  quality.begin('on_device')
  const first = events[0] as Record<string, unknown>
  assert.equal(first.event, 'voice_call_quality')
  assert.equal(first.engine, 'gemini')
  assert.equal(typeof first.ready_ms, 'number')
  assert.deepEqual([first.reconnects, first.stuck, first.failure], [1, 'speaking', 'connection'])
  quality.report()
  assert.equal(events.length, 2)

  let now = 0
  const stuck: unknown[] = []
  const slow = new CallQuality((e) => stuck.push(e), () => now)
  slow.begin('openai')
  now += CallQuality.stuckMs
  slow.report()
  assert.deepEqual(stuck, [{ event: 'voice_call_quality', engine: 'openai', reconnects: 0, stuck: 'connecting', failure: 'none' }])
})

test('call diagnostics correlate engine replacement and view teardown without error text', () => {
  const events: Record<string, unknown>[] = []
  let now = 0
  const quality = new CallQuality((event) => events.push(event), () => now, 'call-for-local-diagnostics')
  quality.begin('openai_live')
  now = 120
  quality.phase('listening', null)
  quality.phase('failed', 'connection', 'host_rpc')
  quality.begin('on_device')
  quality.report('view_unmounted')
  quality.report('user')
  assert.equal(events.length, 2)
  assert.equal(events[0]!.call_id, 'call-for-local-diagnostics')
  assert.equal(events[0]!.failure_detail, 'host_rpc')
  assert.equal(events[0]!.end_reason, 'engine_replaced')
  assert.equal(events[0]!.ready_ms, 120)
  assert.equal(events[1]!.end_reason, 'view_unmounted')
})
