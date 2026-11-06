import type { Key, Locale, Params } from './i18n/index.svelte'
import { ACTIONS, type Action } from './shortcuts'
import type { Slot } from './slots'

export type Text = { en: string } & Partial<Record<Locale, string>>

export type ModAction = { 'terminal.run': string } | { 'prompt.insert': string } | { 'url.open': string } | { squadron: Action } | { mod: string }

export type ModItem = { slot: Slot; id: string; label: Text; hint?: Text; icon?: string; action: ModAction }

export type ModTool = { name: string; description: string; command?: string; input?: Record<string, unknown> }

export type ModSecret = { id: string; label: Text; hint?: Text }

export type Manifest = {
  id: string
  version: string
  api: 1
  name: Text
  description?: Text
  wasm?: string
  permissions?: { exec?: string[]; network?: string[] }
  contributes?: { items?: ModItem[]; tools?: ModTool[]; secrets?: ModSecret[]; theme?: Record<string, string> }
}

export type Found = { dir: string; linked: boolean; manifest: unknown; error: string | null }

export type Problem = [Key, Params?]

export type Mod = { dir: string; linked: boolean; id: string; enabled: boolean; manifest?: Manifest; problems: Problem[]; commands: string[]; pending: string[] }

export type Item = ModItem & { key: string; mod: string; allowed: boolean }

export type Exposed = ModTool & { mod: string; tool: string }

export const MOD_SLOTS: Slot[] = ['agent.tools', 'agent.menu', 'menu.squadron']

export const ICONS = ['editor', 'pull', 'terminal', 'diff', 'kanban', 'remote', 'subagents', 'artifact', 'branch', 'options', 'context', 'turns', 'tasks', 'persona', 'plus', 'copy', 'folder', 'archive', 'stop', 'pin', 'grid', 'jump', 'update', 'mcp']

const NEEDS_AGENT = ['terminal.run', 'prompt.insert']

const ID = /^[a-z0-9][a-z0-9-]{0,39}$/

const TOOL = /^[a-z0-9_]{1,40}$/

const oneLine = (v: unknown) => typeof v === 'string' && !!v.trim() && !/[\x00-\x1f\x7f]/.test(v)

const COLOR = /^(#[0-9a-f]{3,8}|(rgb|rgba|hsl|hsla)\([\d.,%\s/]+\))$/i

const isObject = (v: unknown): v is Record<string, unknown> => typeof v === 'object' && v !== null && !Array.isArray(v)

const isText = (v: unknown): v is Text => isObject(v) && typeof v.en === 'string' && !!v.en.trim() && Object.values(v).every((s) => typeof s === 'string')

export const text = (value: Text, locale: Locale) => value[locale] || value.en

function actionProblem(action: unknown, slot: Slot, wasm: boolean): Problem | null {
  if (!isObject(action) || Object.keys(action).length !== 1) return ['mods.badAction']
  const [kind, value] = Object.entries(action)[0]
  if (typeof value !== 'string' || !value.trim()) return ['mods.badAction']
  if (NEEDS_AGENT.includes(kind) && slot === 'menu.squadron') return ['mods.needsAgent', { action: kind }]
  if (kind === 'terminal.run') return oneLine(value) ? null : ['mods.badCommand']
  if (kind === 'prompt.insert') return null
  if (kind === 'url.open') return /^https:\/\/\S+$/.test(value) ? null : ['mods.badUrl']
  if (kind === 'mod') return wasm ? null : ['mods.needsWasm']
  if (kind === 'squadron') return ACTIONS.some(([id]) => id === value) ? null : ['mods.badSquadron', { action: value }]
  return ['mods.badAction']
}

export function validate(raw: unknown, taken: string[] = []): { manifest?: Manifest; problems: Problem[] } {
  if (!isObject(raw)) return { problems: [['mods.notObject']] }
  const problems: Problem[] = []
  if (typeof raw.id !== 'string' || !ID.test(raw.id)) problems.push(['mods.badId'])
  else if (taken.includes(raw.id)) problems.push(['mods.takenId', { id: raw.id }])
  if (raw.api !== 1) problems.push(['mods.badApi'])
  if (typeof raw.version !== 'string' || !raw.version.trim()) problems.push(['mods.noVersion'])
  if (!isText(raw.name)) problems.push(['mods.noName'])
  if (raw.description !== undefined && !isText(raw.description)) problems.push(['mods.badDescription'])
  const wasm = raw.wasm !== undefined
  if (wasm && (typeof raw.wasm !== 'string' || !raw.wasm.endsWith('.wasm') || raw.wasm.startsWith('/') || raw.wasm.split(/[\\/]/).includes('..'))) problems.push(['mods.badWasm'])
  const exec = isObject(raw.permissions) ? raw.permissions.exec : undefined
  if ((raw.permissions !== undefined && !isObject(raw.permissions)) || (exec !== undefined && (!Array.isArray(exec) || !exec.every((p) => typeof p === 'string' && /^[\w.+-]+$/.test(p)))))
    problems.push(['mods.badExec'])
  const network = isObject(raw.permissions) ? raw.permissions.network : undefined
  if (network !== undefined && (!Array.isArray(network) || !network.every((h) => typeof h === 'string' && /^[a-z0-9-]+(\.[a-z0-9-]+)+$/.test(h)))) problems.push(['mods.badNetwork'])
  const contributes = raw.contributes ?? {}
  if (!isObject(contributes)) problems.push(['mods.badContributes'])
  else {
    const items = contributes.items ?? []
    if (!Array.isArray(items)) problems.push(['mods.badItems'])
    else {
      const seen: string[] = []
      items.forEach((item, index) => {
        const at = { item: isObject(item) && typeof item.id === 'string' ? item.id : `#${index + 1}` }
        if (!isObject(item)) return problems.push(['mods.badItem', at])
        if (typeof item.id !== 'string' || !ID.test(item.id)) problems.push(['mods.badItemId', at])
        else if (seen.includes(item.id)) problems.push(['mods.twiceItem', at])
        else seen.push(item.id)
        if (!MOD_SLOTS.includes(item.slot as Slot)) problems.push(['mods.badSlot', { ...at, slots: MOD_SLOTS.join(', ') }])
        if (!isText(item.label)) problems.push(['mods.noLabel', at])
        if (item.hint !== undefined && !isText(item.hint)) problems.push(['mods.badHint', at])
        if (item.icon !== undefined && !ICONS.includes(item.icon as string)) problems.push(['mods.badIcon', { ...at, icons: ICONS.join(', ') }])
        const wrong = actionProblem(item.action, item.slot as Slot, wasm)
        if (wrong) problems.push([wrong[0], { ...at, ...wrong[1] }])
      })
    }
    const tools = contributes.tools ?? []
    if (!Array.isArray(tools)) problems.push(['mods.badTools'])
    else {
      const seen: string[] = []
      tools.forEach((tool, index) => {
        const at = { tool: isObject(tool) && typeof tool.name === 'string' ? tool.name : `#${index + 1}` }
        if (!isObject(tool)) return problems.push(['mods.badTool', at])
        if (typeof tool.name !== 'string' || !TOOL.test(tool.name)) problems.push(['mods.badToolName', at])
        else if (seen.includes(tool.name)) problems.push(['mods.twiceTool', at])
        else seen.push(tool.name)
        if (typeof tool.description !== 'string' || !tool.description.trim()) problems.push(['mods.noToolDescription', at])
        if (tool.command === undefined ? !wasm : !oneLine(tool.command)) problems.push([tool.command === undefined ? 'mods.toolNeedsWasm' : 'mods.badToolCommand', at])
        if (tool.input !== undefined && !(isObject(tool.input) && tool.input.type === 'object')) problems.push(['mods.badToolInput', at])
      })
    }
    const secrets = contributes.secrets ?? []
    if (!Array.isArray(secrets) || !secrets.every((s) => isObject(s) && typeof s.id === 'string' && ID.test(s.id) && isText(s.label) && (s.hint === undefined || isText(s.hint))))
      problems.push(['mods.badSecrets'])
    else if (new Set(secrets.map((s) => s.id)).size < secrets.length) problems.push(['mods.badSecrets'])
    const theme = contributes.theme ?? {}
    if (!isObject(theme) || Object.entries(theme).some(([k, v]) => !/^[a-z0-9-]+$/.test(k) || typeof v !== 'string' || !COLOR.test(v.trim())))
      problems.push(['mods.badTheme'])
  }
  return problems.length ? { problems } : { manifest: raw as Manifest, problems }
}

export const EXEC = 'exec:'

export const NET = 'net:'

export const commandsOf = (manifest: Manifest) => [
  ...new Set([
    ...(manifest.contributes?.items ?? []).flatMap((i) => ('terminal.run' in i.action ? [i.action['terminal.run']] : [])),
    ...(manifest.contributes?.tools ?? []).flatMap((t) => (t.command ? [t.command] : [])),
    ...(manifest.permissions?.exec ?? []).map((p) => EXEC + p),
    ...(manifest.permissions?.network ?? []).map((h) => NET + h),
  ]),
]

export const grantsOf = (mod: Mod) => mod.commands.filter((c) => (c.startsWith(EXEC) || c.startsWith(NET)) && !mod.pending.includes(c))

export function load(found: Found[], disabled: string[], approved: Record<string, string[]> = {}): Mod[] {
  const taken: string[] = []
  return found.map(({ dir, linked, manifest: raw, error }) => {
    const id = isObject(raw) && typeof raw.id === 'string' ? raw.id : (dir.split(/[\\/]/).pop() ?? dir)
    const { manifest, problems } = error ? { manifest: undefined, problems: [['mods.unreadable', { error }] as Problem] } : validate(raw, taken)
    if (manifest) taken.push(manifest.id)
    const commands = manifest ? commandsOf(manifest) : []
    return { dir, linked, id, enabled: !disabled.includes(id), manifest, problems, commands, pending: commands.filter((c) => !approved[id]?.includes(c)) }
  })
}

export const active = (mods: Mod[]) => mods.filter((m): m is Mod & { manifest: Manifest } => m.enabled && !!m.manifest)

export const itemsOf = (mods: Mod[], slot: Slot): Item[] =>
  active(mods).flatMap((m) =>
    (m.manifest.contributes?.items ?? [])
      .filter((i) => i.slot === slot)
      .map((i) => ({ ...i, key: `${m.manifest.id}.${i.id}`, mod: m.manifest.id, allowed: !('terminal.run' in i.action) || !m.pending.includes(i.action['terminal.run']) })),
  )

export const toolsOf = (mods: Mod[]): Exposed[] =>
  active(mods).flatMap((m) =>
    (m.manifest.contributes?.tools ?? [])
      .filter((t) => !t.command || !m.pending.includes(t.command))
      .map((t) => ({ ...t, name: `${m.manifest.id}_${t.name}`, tool: t.name, mod: m.manifest.id })),
  )

export const themeOf = (mods: Mod[]) => Object.assign({}, ...active(mods).map((m) => m.manifest.contributes?.theme ?? {})) as Record<string, string>
