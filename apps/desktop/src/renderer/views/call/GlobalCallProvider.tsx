import { createContext, useContext, useEffect, useRef, useState, type ReactNode } from 'react'
import type { BotStore } from '../../store/bot-store'
import { StoreContext, useApp } from '../../store/context'
import { useAccount } from '../../store/account'
import { useModel } from '../../lib/observable'
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
  useModel(active?.store)

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
    if (!active) return
    const identityChanged = active.accountId !== (account.user?.userId ?? null)
    const storeReplaced = app.store(active.store.computer.id) !== active.store
    const hostChanged = active.hostId !== null && active.store.hostIdentity !== null && active.hostId !== active.store.hostIdentity
    const deleted = active.store.connection.kind === 'online' && !active.store.bots.has(active.botId)
    if (identityChanged || storeReplaced || hostChanged || deleted || active.store.mismatch) {
      end()
      app.setError(trNow('The call ended because its bot or computer connection changed. Start a new call when it is available.'))
    }
  })
  useEffect(() => () => { current.current?.releaseMicrophone(); current.current = null }, [])

  return (
    <CallContext.Provider value={{ active, speaking, start, interrupt: () => interrupt.current?.() }}>
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
