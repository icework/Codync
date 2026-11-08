import type { VoiceFailureDetail } from '@shared/analytics'
import type { CallLink } from './call-link'

export const isLiveModel = (model: string) => model.startsWith('gpt-live-') && !model.includes('transcribe')
export type LiveUpdateKind = 'reply' | 'notice' | 'progress'

/** Live sends a transcript delta about every word: adjacent ones of one speaker reach the host as one event. */
const TRANSCRIPT_BATCH_MS = 250
const TRANSCRIPT_BATCH_CHARS = 4_000
type LiveEvent = Record<string, unknown>
const isTranscript = (event: LiveEvent) =>
  event.type === 'session.input_transcript.delta' || event.type === 'session.output_transcript.delta'

/** The host owns Live's transcripts and Responses tool routing. This adapter only carries its events. */
export class OpenAILiveSession {
  private sessionId: string | null = null
  private stopped = false
  private closing = false
  private queue: Promise<void> = Promise.resolve()
  private closed: (() => void) | null = null
  private queued = 0
  private pending: LiveEvent | null = null
  private pendingTimer: ReturnType<typeof setTimeout> | undefined
  private link: Pick<CallLink, 'startLive' | 'liveEvent' | 'liveUpdate'>
  private send: (event: unknown) => void
  private failed: (message: string, detail: VoiceFailureDetail) => void

  constructor(
    link: Pick<CallLink, 'startLive' | 'liveEvent' | 'liveUpdate'>,
    send: (event: unknown) => void,
    failed: (message: string, detail: VoiceFailureDetail) => void,
  ) { this.link = link; this.send = send; this.failed = failed }

  async start(model: string, voice: string, sdp: string): Promise<string> {
    const answer = await this.link.startLive(model, voice, sdp)
    if (this.stopped) throw new Error('The voice connection was replaced.')
    this.sessionId = answer.sessionId
    return answer.sdp
  }

  receive(event: LiveEvent) {
    if (event.type === 'session.closed') {
      this.flush()
      this.closed?.()
      if (!this.closing && !this.stopped) this.failed('OpenAI closed the Live session.', 'session_closed')
      return
    }
    const transcript = isTranscript(event)
    if (this.stopped || (this.closing && !transcript)) return
    if (event.type === 'error' || event.type === 'session.error') {
      this.failed((event.error as { message?: string } | undefined)?.message ?? 'OpenAI Live reported an error.', 'provider_error')
      return
    }
    if (!['response.event', 'session.input_transcript.delta', 'session.output_transcript.delta', 'session.delegation.created'].includes(String(event.type))) return
    if (transcript) this.hold(event)
    else {
      this.flush()
      this.enqueue((id) => this.link.liveEvent(id, event))
    }
  }

  update(kind: LiveUpdateKind, text: string) {
    this.flush()
    this.enqueue((id) => this.link.liveUpdate(id, kind, text))
  }

  /** Invalidate every outstanding host result before a replacement connection is created. */
  discard() {
    this.stopped = true
    clearTimeout(this.pendingTimer)
    this.pending = null
  }

  async shutdown() {
    if (this.closing) return
    this.closing = true
    let timer: ReturnType<typeof setTimeout> | undefined
    const confirmed = await new Promise<boolean>((resolve) => {
      this.closed = () => resolve(true)
      timer = setTimeout(() => resolve(false), 1_000)
      this.send({ type: 'session.close' })
    })
    clearTimeout(timer)
    this.flush()
    const drained = await new Promise<boolean>((resolve) => {
      timer = setTimeout(() => resolve(false), 2_000)
      void this.queue.then(() => resolve(true))
    })
    clearTimeout(timer)
    if (!drained) console.warn('GPT-Live transcript delivery was not confirmed before hanging up.')
    this.stopped = true
    this.closed = null
    if (!confirmed) console.warn('OpenAI Live closed without a final session.closed acknowledgement.')
  }

  /** Joins a transcript delta onto the held one when it continues it; it waits at most [TRANSCRIPT_BATCH_MS]. */
  private hold(event: LiveEvent) {
    const held = this.pending
    if (held && held.type === event.type && typeof held.delta === 'string' && typeof event.delta === 'string'
      && held.delta.length + event.delta.length <= TRANSCRIPT_BATCH_CHARS) {
      this.pending = { ...held, delta: held.delta + event.delta, end_ms: event.end_ms }
      return
    }
    this.flush()
    this.pending = event
    this.pendingTimer = setTimeout(() => this.flush(), TRANSCRIPT_BATCH_MS)
  }

  private flush() {
    clearTimeout(this.pendingTimer)
    this.pendingTimer = undefined
    const event = this.pending
    if (!event) return
    this.pending = null
    this.enqueue((id) => this.link.liveEvent(id, event), true)
  }

  private enqueue(run: (id: string) => Promise<{ events: unknown[] }>, transcript = false) {
    if (this.stopped || (this.closing && !transcript)) return
    if (this.queued >= 256) {
      this.stopped = true
      this.failed('The computer cannot keep up with GPT-Live events.', 'queue_full')
      return
    }
    this.queued++
    this.queue = this.queue.then(async () => {
      if (this.stopped || (this.closing && !transcript) || !this.sessionId) return
      const result = await run(this.sessionId)
      if (this.stopped || this.closing) return
      for (const event of result.events) this.send(event)
    }).catch((error: unknown) => {
      if (this.closing && transcript) console.warn('GPT-Live transcript delivery failed while hanging up.')
      if (!this.stopped && !this.closing) {
        this.stopped = true
        this.failed(error instanceof Error ? error.message : String(error), 'host_rpc')
      }
    }).finally(() => { this.queued-- })
  }
}

/** Live uses a complete SDP offer; stop waiting if the connection closes or ICE cannot gather. */
export async function gatheredOffer(pc: RTCPeerConnection): Promise<string> {
  if (pc.iceGatheringState !== 'complete') {
    await new Promise<void>((resolve, reject) => {
      const finish = (error?: Error) => {
        clearTimeout(timer)
        pc.removeEventListener('icegatheringstatechange', changed)
        pc.removeEventListener('connectionstatechange', changed)
        if (error) reject(error); else resolve()
      }
      const changed = () => {
        if (pc.connectionState === 'closed') finish(new Error('The voice connection closed.'))
        else if (pc.iceGatheringState === 'complete') finish()
      }
      const timer = setTimeout(() => finish(new Error('Could not gather the voice connection addresses.')), 10_000)
      pc.addEventListener('icegatheringstatechange', changed)
      pc.addEventListener('connectionstatechange', changed)
      changed()
    })
  }
  const sdp = pc.localDescription?.sdp
  if (!sdp) throw new Error('The voice connection produced no SDP offer.')
  return sdp
}
