import type { FallbackReason } from './bot.ts'
import type { ArtifactCard } from './artifacts'
import type { CallLine } from './calls'

// Wire types for codync-host (see host/src/api/). Field names match the
// host's camelCase JSON. Timestamps are epoch milliseconds.

export * from './bot.ts'

export interface FileDiff {
  path: string
  added: number
  removed: number
  isNew: boolean
  startLine: number
  patch: string
}

export interface PermissionOption {
  optionId: string
  name: string
  /** allow_once | allow_always | reject_once | reject_always */
  kind: string
}

export interface PlanItem {
  content: string
  status: string
  priority?: string
}

/** One bot's emoji on a message. */
export interface BotReaction {
  bot: string
  emoji: string
}

/** The bots' reactions on one message, by emoji in the order each first appeared. */
export function groupBotReactions(reactions: BotReaction[]): { emoji: string; bots: string[] }[] {
  const groups: { emoji: string; bots: string[] }[] = []
  for (const r of reactions) {
    const group = groups.find((g) => g.emoji === r.emoji)
    if (group) group.bots.push(r.bot)
    else groups.push({ emoji: r.emoji, bots: [r.bot] })
  }
  return groups
}

/** The message a user message replies to, as the host quoted it. */
export interface ReplyQuote {
  id: string
  /** `user`, or the bot that wrote it. */
  author: string
  /** Its text (or file names), at most 500 characters. */
  text: string
  /** The artifact, when it quotes an artifact card: the message is about that artifact. */
  artifact?: string
}

export interface Attachment {
  id: string
  name: string
  size: number
}

export interface SharedFile extends Attachment { sha256: string }

export const isImageName = (name: string) =>
  ['png', 'jpg', 'jpeg', 'heic', 'gif', 'webp', 'tiff', 'bmp'].includes(name.split('.').pop()?.toLowerCase() ?? '')

/** A shared image small enough to show in the chat (the whole file is fetched for it). */
export const showsAsImage = (file: SharedFile) => isImageName(file.name) && file.size <= 20 * 1024 * 1024

export interface ConnectionRequest {
  [key: string]: unknown
}

export interface BotMessage {
  sourceBotId: string
  targetBotId: string
  text: string
  reply?: string
  detail?: string
  /** The peer's name when it is on another of the user's computers (its id is `name@computer`). */
  peerName?: string
}

/** "Earlier context was summarized": the harness's summary, when it gave one. */
export interface Compaction {
  summary?: string | null
  /** The host started it ahead of time; otherwise the harness did when its context filled. */
  byHost?: boolean
}

/** What a new conversation carries when the bot's own chat starts over; editable until the next message sends it. */
export interface HandoffCard {
  /** draft · sent · kept */
  status: string
  /** cold · away · full */
  reason?: string
  text?: string
}

/** notice: another agent of the bot's chain carried on, or (`back`) the bot's own is back. */
export interface AgentSwitch {
  from: string
  to: string
  back: boolean
  reason: FallbackReason | null
  resetsAt: number | null
  /** What the new agent was told, unfolded under the line. */
  carried: string | null
}

export interface EntryData {
  /** Structured conversation data added to the existing team-tool notice. */
  botMessage?: BotMessage
  /** notice: the agent summarized its context; kept out of the chat. */
  compaction?: Compaction
  /** notice: the bot's chat starts over with this handoff (see `HandoffNotice`). */
  handoff?: HandoffCard
  agentSwitch?: AgentSwitch
  /** notice (error): the bot has no fallbacks and its agent ran out; offers to add one. */
  addFallback?: boolean
  heading?: string
  delegationId?: string
  sourceBotId?: string
  targetBotId?: string
  connectionRequest?: ConnectionRequest
  /** A memory line an older host put in the chat. Memory lives in the bot's memory settings, so it isn't shown. */
  memory?: unknown
  memoryFile?: unknown
  routineId?: string
  runId?: string
  /** notice: a failed routine run the user can run again (`retryRoutineRun`). */
  retry?: boolean
  text?: string
  final?: boolean
  /** user: queued | sent | cancelled | failed · tool: pending | in_progress | completed | failed · permission: pending | answered | cancelled | expired */
  status?: string
  /** user: taken back from the queue before it went out (`unqueue`); no view reads it (`withoutRemoved`). */
  removed?: boolean
  clientNonce?: string
  title?: string
  toolKind?: string
  /** tool: whether it's the bot's own bookkeeping (composing its message, keeping its memory), left out of the
   * working steps; a host older than 2.25.0 leaves it out, and the steps then leave out the call too. */
  bookkeeping?: boolean
  output?: string
  diffs?: FileDiff[]
  locations?: { path: string; line?: number }[]
  options?: PermissionOption[]
  selected?: string
  command?: string
  detail?: string
  cwd?: string
  entries?: PlanItem[]
  /** notice: info | error */
  style?: string
  author?: string
  /** user: the message this one replies to. */
  replyTo?: ReplyQuote
  reactions?: string[]
  /** The bots' reactions, one per bot, oldest first. */
  botReactions?: BotReaction[]
  attachments?: Attachment[]
  files?: SharedFile[]
  /** agent: an artifact the bot published (or updated) in this turn. */
  artifact?: ArtifactCard
  callSeconds?: number
  /** The voice call it was said in (user), answered during (agent), or that it ends (notice): the chat shows those
   * messages only in the call's conversation. */
  call?: string
  /** user: `voice` when a call's voice wrote it for the user (`send_to_bot`). */
  from?: string
  /** notice: what was said on a realtime call, oldest first. */
  transcript?: CallLine[]
}

export interface Entry {
  id: string
  seq: number
  botId: string
  rev: number
  /** user | agent | thought | tool | plan | permission | notice */
  kind: string
  turn: number
  data: EntryData
  createdAt: number
  updatedAt: number
}

const shownOf = new WeakMap<Entry[], Entry[]>()

/**
 * `list` as the views read it, without the messages taken back (`unqueue`): the store keeps those for its rev guard
 * against a late response. One array per list, because the chat's memos depend on its identity.
 */
export function withoutRemoved(list: Entry[]): Entry[] {
  let shown = shownOf.get(list)
  if (!shown) {
    shown = list.filter((e) => !e.data.removed)
    shownOf.set(list, shown)
  }
  return shown
}

/** Chat-visible (Grok Bot model): user messages, final replies, approval cards, notices. */
export function isChat(e: Entry): boolean {
  switch (e.kind) {
    case 'user':
    case 'permission':
      return true
    case 'notice':
      // Only a fresh conversation gets a line: memory lives in the bot's memory settings, a summary keeps the same
      // conversation going, and a kept handoff changed nothing.
      return !e.data.memory && !e.data.memoryFile && !e.data.compaction && e.data.handoff?.status !== 'kept'
    case 'agent':
      return e.data.final === true
    default:
      return false
  }
}

/** Messages a reply can quote: the user's and the bots' final replies, once the computer has them. */
export function isReplyable(e: Entry): boolean {
  return !e.id.startsWith('local-') && (e.kind === 'user' || (e.kind === 'agent' && e.data.final === true))
}

/** The quote a sending reply shows until the computer's own replaces it. */
export function quoteOf(e: Entry): ReplyQuote {
  const names = [...(e.data.attachments ?? []), ...(e.data.files ?? [])].map((f) => f.name).join(', ')
  const quote: ReplyQuote = { id: e.id, author: e.kind === 'user' ? 'user' : (e.data.author ?? e.botId), text: (e.data.text || names).slice(0, 500) }
  if (e.data.artifact) quote.artifact = e.data.artifact.id
  return quote
}

export interface UsageWindow {
  id: string
  label: string
  percent: number
  resetsAt?: number | null
  resetsText?: string | null
  /** A reading that isn't a share of a cap. Shown instead of the percent, and never the tightest limit. */
  text?: string | null
}

export interface UsageProvider {
  id: string
  name: string
  windows: UsageWindow[]
  source: string
  updatedAt: number
  /** The agent's ACP registry id, which its logo goes by. Absent from a host that doesn't name it. */
  registry?: string
}

/** An agent the Usage screen can show or hide. The host keeps the choice, so every app follows it. */
export interface UsageService {
  id: string
  name: string
  shown: boolean
  /** The agent's ACP registry id, which its logo goes by. */
  registry?: string
}

export interface Usage {
  providers: UsageProvider[]
  /** Absent from a host older than the chooser. */
  services?: UsageService[]
}

export interface Backend {
  id: string
  name: string
  available: boolean
  command: string
  installHint: string
  registry?: string
  description?: string
  installed?: boolean
  curated?: boolean
  signedIn?: boolean
  canInstall?: boolean
  /** Only for an agent that runs on Node.js, on a computer without its own: Codync's copy is missing or added. */
  node?: 'missing' | 'added' | null
}

export interface ScreenDisplay {
  id: number
  name: string
  width: number
  height: number
  main: boolean
}

export interface ScreenState {
  /** The helper's display name in operating-system permission settings. */
  permissionApp?: string
  enabled: boolean
  connected: boolean
  platform: string
  capture: boolean
  input: boolean
  displays: ScreenDisplay[]
  userControl: boolean
  agentBot?: string | null
  /** Bots with computer use can act here (Windows: always; elsewhere Remote screen is on). */
  computerUse: boolean
  viewers: number
}

export function normalizeScreen(raw: Partial<ScreenState> | undefined | null): ScreenState | null {
  if (!raw) return null
  return {
    enabled: raw.enabled ?? false,
    connected: raw.connected ?? false,
    platform: raw.platform ?? '',
    permissionApp: raw.permissionApp,
    capture: raw.capture ?? false,
    input: raw.input ?? false,
    displays: raw.displays ?? [],
    userControl: raw.userControl ?? false,
    agentBot: raw.agentBot ?? null,
    computerUse: raw.computerUse ?? false,
    viewers: raw.viewers ?? 0,
  }
}

export interface Hello {
  iconsRevision?: number
  icons?: import('./custom-icons').CustomIcons
  hostId: string
  name: string
  version: string
  os: string
  device?: string
  home?: string
  backends: Backend[]
  rev: number
  screen?: ScreenState
  urls?: string[]
  computerId?: string
  signKey?: string
  boxKey?: string
  minApp?: string
  /** `installHostUpdate` can update this computer (absent before hosts could be updated from a client). */
  canUpdate?: boolean
  /** The latest release the computer found; null while unknown (development builds, Homebrew, Windows; absent from older hosts). */
  latestVersion?: string | null
  cloud?: string | null
  /** Product analytics on this computer: on unless its owner turned it off. */
  analytics?: boolean
}

export interface PairingInfo {
  pairingUrl: string
  urls: string[]
  expiresAt?: number
}

export interface DirItem {
  name: string
  path: string
  isGit: boolean
}

export interface RecentWorkspace { path: string; usedAt: number }

export interface DirListing {
  recent?: RecentWorkspace[]
  ancestors?: DirItem[]
  path: string
  parent?: string | null
  isGit: boolean
  dirs: DirItem[]
}

/** A pending request as the computer shows it (loopback `accessRequests`). */
export interface AccessRequest {
  requestId: string
  deviceKey: string
  deviceName: string
  platform?: string | null
  email?: string | null
  /** The SAS; nil until the phone revealed its nonce (Approve stays disabled). */
  code?: string | null
  createdAt?: number | null
  expiresAt?: number | null
}

/** A signed-in phone that scanned a computer in no account, waiting to be let in (loopback `pairingRequests`). */
export interface PairingRequest {
  requestId: string
  deviceName: string
  platform?: string | null
  /** The phone's account, which the computer joins. */
  email?: string | null
  /** The 6 digits the phone shows too. */
  code: string
  expiresAt?: number | null
}

/** A device the host authorized (loopback `devices`). */
export interface AuthorizedDevice {
  key: string
  name: string
  platform?: string | null
  /** `local` (QR) or `account`. */
  source: string
  scopes?: string[]
  leaseUntil?: number | null
  createdAt?: number | null
  lastSeenAt?: number | null
  connected?: boolean | null
}

export interface CloudOwner {
  userId: string
  email?: string | null
}

/** `code`: compare the 6-digit code at the computer; `auto`: let account devices in. */
export type AccountApproval = 'code' | 'auto'

export interface CloudStatus {
  enabled: boolean
  url?: string | null
  /** A private relay's registration token is saved (the token itself never leaves the host). */
  tokenSet?: boolean | null
  registered?: boolean | null
  connected?: boolean | null
  owner?: CloudOwner | null
  lastError?: string | null
  approval?: AccountApproval | null
  /** Whose relay: a `private` one has no account, access requests or relayed screen. Absent from older hosts (always Codync's). */
  relay?: 'codync' | 'private' | null
  /** The user's cloud assistant may message this computer's bots (`setAssistants`). */
  assistants?: boolean | null
  /** In no account while it could join one: a signed-in phone that pairs asks to bring it into its account. Absent from older hosts. */
  canJoinAccount?: boolean | null
  /** Turned on, the cloud would leave it in no account on a Codync cloud; known with the cloud off too. Absent from older hosts, which just turn on. */
  joinsAccountOnEnable?: boolean | null
}

export interface ClaimSignature {
  computerId: string
  signKey: string
  boxKey: string
  name: string
  platform: string
  device?: string | null
  version: string
  sig: string
}

/** A computer this device talks to. Holds no secrets: only pinned public keys and addresses. */
export interface Computer {
  id: string
  name: string
  signKey: string
  boxKey?: string | null
  urls: string[]
  cloud?: string | null
  device?: string | null
  color?: string | null
}

/** Create / update payload for a bot. */
export { draftOf } from './bot-draft.ts'
export type { BotDraft } from './bot-draft.ts'

export interface GroupDraft {
  id?: string
  kind: 'group'
  name: string
  description: string
  members: string[]
  pinned?: boolean
}
