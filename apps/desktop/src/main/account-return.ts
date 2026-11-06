import { randomBytes } from 'node:crypto'
import { createServer } from 'node:http'
import type { AddressInfo } from 'node:net'

/** A native OAuth return for one unpackaged app. Generic Electron bundles cannot own macOS deep links. */
export async function loopbackAccountReturn(receive: (url: string) => void): Promise<{ url: string; scheme: string; close: () => void }> {
  const path = `/account-return/${randomBytes(24).toString('hex')}`
  let origin = ''
  const server = createServer((request, response) => {
    response.setHeader('Cache-Control', 'no-store')
    response.setHeader('Referrer-Policy', 'no-referrer')
    response.setHeader('Content-Security-Policy', "default-src 'none'; frame-ancestors 'none'")
    response.setHeader('Content-Type', 'text/plain; charset=utf-8')
    let url: URL
    try { url = new URL(request.url ?? '/', origin) } catch { response.writeHead(404).end('Not found'); return }
    if (request.method !== 'GET' || request.headers.host !== new URL(origin).host || url.origin !== origin || url.pathname !== path || (request.url?.length ?? 0) > 8192) {
      response.writeHead(404).end('Not found')
      return
    }
    // Receipt is not authentication: AccountService still checks the nonce against its current Clerk attempt.
    response.end('Return to Codync to finish signing in. You can close this tab.', () => receive(url.toString()))
  })
  server.requestTimeout = 10_000
  server.headersTimeout = 10_000
  server.unref()
  await new Promise<void>((resolve, reject) => {
    server.once('error', reject)
    server.listen(0, '127.0.0.1', () => { server.removeListener('error', reject); resolve() })
  })
  origin = `http://127.0.0.1:${(server.address() as AddressInfo).port}`
  return {
    url: origin + path,
    scheme: 'http',
    close: () => { server.close(); server.closeAllConnections() },
  }
}
