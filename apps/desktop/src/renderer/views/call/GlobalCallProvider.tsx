import { createContext, useContext, useEffect, useMemo, useRef, useState, useSyncExternalStore, type ReactNode } from 'react'
import type { BotStore } from '../../store/bot-store'
import { StoreContext, useApp } from '../../store/context'
import { useAccount } from '../../store/account'
import { stopDictation } from '../thread/dictation'
import { CallView } from './CallView'
import { acquireCallMicrophone } from './call-microphone'
import { trNow } from '../../lib/l10n'

interface ActiveCall {
  id: string
  store: BotStore
  botId: string
  accountId: string | null
  hostId: string
  releaseMicrophone: () => void
}
interface Calls {
  active: ActiveCall | null
  speaking: boolean
  start(store: BotStore, botId: string): void
  interrupt(): void
}
const idle: Calls = { active: null, speaking: false, start: () => {}, interrupt: () => {} }
const CallContext = createContext<Calls>(idle)
export const useGlobalCall = () => useContext(CallContext)

/** Above navigation/onboarding/recovery branches: the destination never follows the selected chat. */
export function GlobalCallProvider({ children }: { children: ReactNode }) {
  const app = useApp()
  const account = useAccount()
  const [active, setActive] = useState<ActiveCall | null>(null)
  const current = useRef<ActiveCall | null>(null)
  const [speaking, setSpeaking] = useState(false)
  const interrupt = useRef<(() => void) | null>(null)
  // Only whether the call lost its bot or computer, not every change of the store (a streaming reply), renders here.
  const userId = account.user?.userId ?? null
  const lost = useSyncExternalStore(active ? active.store.subscribe : noSubscription,
    () => active !== null && callLost(active, app.store(active.store.computer.id), userId))

  const end = () => {
    current.current?.releaseMicrophone()
    current.current = null
    setActive(null)
    setSpeaking(false)
    interrupt.current = null
  }
  const start = (store: BotStore, botId: string) => {
    // Synchronous guard: two clicks in one render cannot create two microphone owners.
    const hostId = store.hostIdentity
    if (current.current || !hostId || store.mismatch || !store.bots.has(botId)) return
    const call = { id: crypto.randomUUID(), store, botId, accountId: account.user?.userId ?? null,
      hostId, releaseMicrophone: acquireCallMicrophone() }
    stopDictation()
    current.current = call
    setActive(call)
  }

  useEffect(() => {
    if (!lost) return
    end()
    app.setError(trNow('The call ended because its bot or computer connection changed. Start a new call when it is available.'))
  }, [lost])
  useEffect(() => () => { current.current?.releaseMicrophone(); current.current = null }, [])

  const calls = useMemo<Calls>(() => ({ active, speaking, start, interrupt: () => interrupt.current?.() }), [active, speaking])

  return (
    <CallContext.Provider value={calls}>
      {children}
      {active ? (
        <div className="global-call-overlay">
          <StoreContext.Provider value={active.store}>
            <CallView key={active.id} botId={active.botId} expectedHostIdentity={active.hostId} onEnd={end} interrupt={interrupt}
              onSpeaking={(value) => { if (current.current?.id === active.id) setSpeaking(value) }}
              onReturn={() => app.select({ computerId: active.store.computer.id, botId: active.botId })} />
          </StoreContext.Provider>
        </div>
      ) : null}
    </CallContext.Provider>
  )
}

const noSubscription = () => () => {}

/** The call's account, computer, host or bot is gone or changed under it. */
function callLost(call: ActiveCall, current: BotStore | null, userId: string | null) {
  const { store } = call
  const hostChanged = store.hostIdentity !== null && call.hostId !== store.hostIdentity
  const deleted = store.connection.kind === 'online' && !store.bots.has(call.botId)
  return call.accountId !== userId || current !== store || hostChanged || deleted || store.mismatch
}
