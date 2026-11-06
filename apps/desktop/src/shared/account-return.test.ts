import assert from 'node:assert/strict'
import { test } from 'node:test'
import { request } from 'node:http'
import { loopbackAccountReturn } from '../main/account-return.ts'

test('each unpackaged login gets an isolated loopback return without reflecting its nonce', async () => {
  const seen: string[] = []
  const first = await loopbackAccountReturn(url => seen.push(url))
  const second = await loopbackAccountReturn(url => seen.push(url))
  try {
    assert.notEqual(first.url, second.url)
    assert.equal(new URL(first.url).hostname, '127.0.0.1')
    const wrong = await fetch(new URL('/wrong', first.url))
    assert.equal(wrong.status, 404)
    assert.equal((await fetch(first.url, { method: 'POST' })).status, 404)
    assert.equal(await new Promise<number | undefined>((resolve, reject) => {
      const req = request(first.url, { headers: { Host: 'attacker.invalid' } }, res => { res.resume(); resolve(res.statusCode) })
      req.on('error', reject); req.end()
    }), 404)
    assert.deepEqual(seen, [])
    const callback = first.url + '?rotating_token_nonce=fixture-only'
    const response = await fetch(callback)
    assert.equal(response.status, 200)
    assert.equal(response.headers.get('cache-control'), 'no-store')
    assert.equal(response.headers.get('referrer-policy'), 'no-referrer')
    assert.equal((await response.text()).includes('fixture-only'), false)
    assert.deepEqual(seen, [callback])
  } finally { first.close(); second.close() }
  await assert.rejects(fetch(first.url))
})
