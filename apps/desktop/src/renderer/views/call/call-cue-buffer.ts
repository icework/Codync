import type { CallCue } from '@shared/calls'

/** Call-level delivery survives voice-engine setup/replacement; it never replays an accepted cue. */
export class CallCueBuffer {
  private pending: CallCue[] = []
  private receiver: ((cue: CallCue) => void) | null = null
  private stopped = false
  private overflowed = false
  private overflow: () => void
  private readonly limit: number

  constructor(overflow: () => void, limit = 100) { this.overflow = overflow; this.limit = limit }

  receive = (cue: CallCue) => {
    if (this.stopped) return
    if (cue.kind === 'progress') this.pending = this.pending.filter((p) => p.kind !== 'progress')
    if (this.pending.length >= this.limit) {
      this.pending = this.pending.filter((p) => p.kind !== 'progress')
      if (this.pending.length >= this.limit) {
        if (!this.overflowed) { this.overflowed = true; this.overflow() }
        return
      }
    }
    this.pending.push(cue)
    this.flush()
  }

  activate(receiver: (cue: CallCue) => void) { this.receiver = receiver; this.flush() }
  pause(receiver: (cue: CallCue) => void) { if (this.receiver === receiver) this.receiver = null }
  stop() { this.stopped = true; this.receiver = null; this.pending = [] }

  private flush() {
    const receiver = this.receiver
    while (receiver && this.receiver === receiver && this.pending.length) receiver(this.pending.shift()!)
  }
}
