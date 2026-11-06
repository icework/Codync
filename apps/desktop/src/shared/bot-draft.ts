import type { Bot, Fallback } from './models'

export interface BotDraft {
  id?: string
  name: string
  description: string
  avatarColor: string
  avatarShape: string
  backend: string
  command?: string | null
  cwd: string
  permission: string
  /** Null for the agent's default: sent as is, so the host clears it. */
  mode?: string | null
  model?: string | null
  modelOptions?: Record<string, string>
  fastMode?: boolean
  fallbacks?: Fallback[]
  selfFallbacks?: boolean
  pinned?: boolean
  /** A name puts the bot in that roster section, null takes it out; left out, the host keeps it. */
  section?: string | null
  hidden?: boolean
  notify?: boolean | null
  connectors?: string[]
  skills?: string[]
  computer?: boolean
  manageTeam?: boolean
  offeredToNetwork?: boolean
  reachableByAssistants?: boolean
  sharedMemory?: boolean
  harnessMemory?: boolean
}

export function draftOf(bot: Bot): BotDraft {
  return {
    id: bot.id,
    name: bot.name,
    description: bot.description,
    avatarColor: bot.avatarColor,
    avatarShape: bot.avatarShape,
    backend: bot.backend,
    command: bot.command ?? null,
    cwd: bot.managedWorkspace ? '' : bot.cwd,
    permission: bot.permission,
    mode: bot.mode,
    model: bot.model ?? null,
    modelOptions: bot.modelOptions,
    fastMode: bot.fastMode,
    fallbacks: bot.fallbacks,
    selfFallbacks: bot.selfFallbacks ?? undefined,
    pinned: bot.pinned,
    hidden: bot.hidden,
    notify: bot.notify ?? null,
    connectors: bot.connectors,
    skills: bot.skills,
    computer: bot.computer,
    manageTeam: bot.manageTeam ?? undefined,
    offeredToNetwork: bot.offeredToNetwork ?? undefined,
    reachableByAssistants: bot.reachableByAssistants ?? undefined,
  }
}
