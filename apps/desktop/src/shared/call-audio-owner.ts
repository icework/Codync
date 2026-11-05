/** Main-process microphone ownership. Renderer IDs never leave this coordinator. */
export class CallAudioOwner {
  private owner: number | null = null
  private changed: (active: boolean) => void

  constructor(changed: (active: boolean) => void) { this.changed = changed }

  get active() { return this.owner !== null }
  permitsSpeech(renderer: number) { return this.owner === null || this.owner === renderer }

  /** A competing claim or a different renderer's release cannot take ownership away. */
  setCalling(renderer: number, on: boolean): boolean {
    if (on) {
      if (this.owner !== null) return false
      this.owner = renderer
    } else {
      if (this.owner !== renderer) return false
      this.owner = null
    }
    this.changed(this.active)
    return true
  }
}
