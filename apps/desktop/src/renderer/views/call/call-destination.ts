/** A call's authority is fixed when it starts, even if a Store reconnects before a queued RPC runs. */
export class CallDestination {
  private expected: string | null
  private current: () => { identity: string | null; incompatible: boolean }

  constructor(expected: string | null, current: () => { identity: string | null; incompatible: boolean }) {
    this.expected = expected
    this.current = current
  }

  check = () => {
    const { identity, incompatible } = this.current()
    if (!this.expected || identity !== this.expected || incompatible) {
      throw new Error('The call’s computer identity or compatibility changed. Start a new call when it is available.')
    }
  }
}
