import { invoke } from '@tauri-apps/api/core'
import type { Draft } from './newAgentDraft'
import { workdir, type Agent, type Options } from './stream'

export type OpenDraft = { id: string; extension: string; project: string; agent: string; draft: Draft; updates: Draft[] }

export const drafts = $state({ open: null as OpenDraft | null })

export type Kept = { name: string; text: string; goal: string; files: string[]; worktree: string | null; options: Options }

export const composing = $state<Record<string, boolean>>({})

export const kept = new Map<string, Kept>()

const tell = (open: OpenDraft, name: string, data: Record<string, unknown>, agent: unknown = null) =>
  invoke('extensions_wasm_event', { id: open.extension, name, agent, data: JSON.stringify({ draft: open.id, ...data }) }).catch(() => {})

export const draftRegenerate = (open: OpenDraft, choices: string[]) => tell(open, 'new-agent.regenerate', { choices })

export const draftStarted = (open: OpenDraft, agent: Agent) => tell(open, 'new-agent.started', {}, { id: agent.id, name: agent.name, project: agent.project, dir: workdir(agent) })

export const draftClosed = (open: OpenDraft) => tell(open, 'new-agent.closed', {})
