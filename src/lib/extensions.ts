import type { Key, Locale, Params } from './i18n/index.svelte'
import { ACTIONS, type Action } from './shortcuts'
import type { Slot } from './slots'

export type Text = { en: string } & Partial<Record<Locale, string>>

export type ShareStep = string | { tool: string }

export type ShareAction = { confirm: Text; lines?: string[]; shared?: Text; share: ShareStep; update?: ShareStep; unshare?: ShareStep; delete?: Text; deleteHint?: Text }

export type ExtensionAction =
  | { 'terminal.run': string }
  | { 'prompt.insert': string }
  | { 'url.open': string }
  | { squadron: Action }
  | { run: string }
  | { 'panel.open': string }
  | { 'artifact.share': ShareAction }

export type ExtensionItem = { slot: Slot; id: string; label: Text; hint?: Text; icon?: string; action: ExtensionAction; words?: string[]; spotlight?: boolean }

export type ExtensionTool = { name: string; description: string; command?: string; input?: Record<string, unknown> }

export type ExtensionSecret = { id: string; label: Text; hint?: Text }

export type ManifestPanel = { id: string; title: Text; page: string }

export type Panel = ManifestPanel & { key: `extension:${string}`; extension: string }

export const AGENT_ACCESS = ['read', 'archive'] as const

export type Permissions = { exec?: string[]; network?: string[]; agents?: (typeof AGENT_ACCESS)[number][] }

export type Manifest = {
  id: string
  version: string
  api: 1
  name: Text
  description?: Text
  wasm?: string
  permissions?: Permissions
  contributes?: { args?: string[]; skills?: string[]; items?: ExtensionItem[]; tools?: ExtensionTool[]; secrets?: ExtensionSecret[]; panels?: ManifestPanel[]; theme?: Record<string, string> }
}

export type Found = { dir: string; linked: boolean; manifest: unknown; error: string | null }

export type Problem = [Key, Params?]

export type Extension = { dir: string; linked: boolean; id: string; enabled: boolean; manifest?: Manifest; problems: Problem[]; commands: string[]; pending: string[] }

export type Item = ExtensionItem & { key: string; extension: string; allowed: boolean }

export type Exposed = ExtensionTool & { extension: string; tool: string }

export const EXTENSION_SLOTS: Slot[] = ['agent.tools', 'agent.menu', 'menu.squadron', 'artifact.actions']

export const ICONS = ['editor', 'pull', 'terminal', 'diff', 'kanban', 'remote', 'subagents', 'artifact', 'branch', 'options', 'context', 'turns', 'tasks', 'persona', 'plus', 'copy', 'folder', 'archive', 'stop', 'pin', 'grid', 'jump', 'update', 'mcp']

export const NEEDS_AGENT = ['terminal.run', 'prompt.insert', 'panel.open']

const ID = /^[a-z0-9][a-z0-9-]{0,39}$/

const TOOL = /^[a-z0-9_]{1,40}$/

const oneLine = (v: unknown) => typeof v === 'string' && !!v.trim() && !/[\x00-\x1f\x7f]/.test(v)

const RESERVED = ['-p', '--print', '--input-format', '--output-format', '--replay-user-messages', '--model', '--effort', '--permission-mode', '--append-system-prompt', '--agent', '--advisor', '-r', '--resume', '-c', '--continue', '--fork-session', '--resume-session-at', '--session-id', '--mcp-config', '--strict-mcp-config', '--permission-prompt-tool', '--dangerously-skip-permissions', '--allow-dangerously-skip-permissions', '-w', '--worktree']

const COLOR = /^(#[0-9a-f]{3,8}|(rgb|rgba|hsl|hsla)\([\d.,%\s/]+\))$/i

const isObject = (v: unknown): v is Record<string, unknown> => typeof v === 'object' && v !== null && !Array.isArray(v)

const isText = (v: unknown): v is Text => isObject(v) && typeof v.en === 'string' && !!v.en.trim() && Object.values(v).every((s) => typeof s === 'string')

export const text = (value: Text, locale: Locale) => value[locale] || value.en

const inside = (path: unknown, extension: string) =>
  typeof path === 'string' && path.endsWith(extension) && !path.startsWith('/') && !path.split(/[\\/]/).some((p) => !p || p === '..')

const stepOk = (step: unknown, wasm: boolean) => (typeof step === 'string' ? !!step.trim() : isObject(step) && typeof step.tool === 'string' && TOOL.test(step.tool) && wasm)

function shareProblem(value: unknown, slot: Slot, wasm: boolean): Problem | null {
  if (slot !== 'artifact.actions') return ['extensions.shareSlot']
  if (!isObject(value) || !isText(value.confirm) || !stepOk(value.share, wasm)) return ['extensions.badShare']
  if ([value.update, value.unshare].some((s) => s !== undefined && !stepOk(s, wasm))) return ['extensions.badShare']
  if ([value.shared, value.delete, value.deleteHint].some((s) => s !== undefined && !isText(s))) return ['extensions.badShare']
  if (value.lines !== undefined && !(Array.isArray(value.lines) && value.lines.every(oneLine))) return ['extensions.badShare']
  return null
}

function actionProblem(action: unknown, slot: Slot, wasm: boolean, panels: string[]): Problem | null {
  if (!isObject(action) || Object.keys(action).length !== 1) return ['extensions.badAction']
  const [kind, value] = Object.entries(action)[0]
  if (kind === 'artifact.share') return shareProblem(value, slot, wasm)
  if (typeof value !== 'string' || !value.trim()) return ['extensions.badAction']
  if (NEEDS_AGENT.includes(kind) && slot === 'menu.squadron') return ['extensions.needsAgent', { action: kind }]
  if (kind === 'terminal.run') return oneLine(value) ? null : ['extensions.badCommand']
  if (kind === 'prompt.insert') return null
  if (kind === 'url.open') return /^https:\/\/\S+$/.test(value) ? null : ['extensions.badUrl']
  if (kind === 'run') return wasm ? null : ['extensions.needsWasm']
  if (kind === 'panel.open') return panels.includes(value) ? null : ['extensions.unknownPanel', { panel: value }]
  if (kind === 'squadron') return ACTIONS.some(([id]) => id === value) ? null : ['extensions.badSquadron', { action: value }]
  return ['extensions.badAction']
}

export function validate(raw: unknown, taken: string[] = []): { manifest?: Manifest; problems: Problem[] } {
  if (!isObject(raw)) return { problems: [['extensions.notObject']] }
  const problems: Problem[] = []
  if (typeof raw.id !== 'string' || !ID.test(raw.id)) problems.push(['extensions.badId'])
  else if (taken.includes(raw.id)) problems.push(['extensions.takenId', { id: raw.id }])
  if (raw.api !== 1) problems.push(['extensions.badApi'])
  if (typeof raw.version !== 'string' || !raw.version.trim()) problems.push(['extensions.noVersion'])
  if (!isText(raw.name)) problems.push(['extensions.noName'])
  if (raw.description !== undefined && !isText(raw.description)) problems.push(['extensions.badDescription'])
  const wasm = raw.wasm !== undefined
  if (wasm && !inside(raw.wasm, '.wasm')) problems.push(['extensions.badWasm'])
  const exec = isObject(raw.permissions) ? raw.permissions.exec : undefined
  if ((raw.permissions !== undefined && !isObject(raw.permissions)) || (exec !== undefined && (!Array.isArray(exec) || !exec.every((p) => typeof p === 'string' && /^[\w.+-]+$/.test(p)))))
    problems.push(['extensions.badExec'])
  const network = isObject(raw.permissions) ? raw.permissions.network : undefined
  if (network !== undefined && (!Array.isArray(network) || !network.every((h) => typeof h === 'string' && /^[a-z0-9-]+(\.[a-z0-9-]+)+$/.test(h)))) problems.push(['extensions.badNetwork'])
  const access = isObject(raw.permissions) ? raw.permissions.agents : undefined
  if (access !== undefined && (!Array.isArray(access) || !access.every((a) => (AGENT_ACCESS as readonly unknown[]).includes(a)))) problems.push(['extensions.badAgents'])
  const contributes = raw.contributes ?? {}
  if (!isObject(contributes)) problems.push(['extensions.badContributes'])
  else {
    const args = contributes.args ?? []
    if (!Array.isArray(args) || !args.every(oneLine) || (args.length && !args[0].startsWith('-'))) problems.push(['extensions.badArgs'])
    else for (const arg of args) if (RESERVED.includes(arg.split('=')[0])) problems.push(['extensions.reservedArg', { arg }])
    const skills = contributes.skills ?? []
    if (!Array.isArray(skills) || !skills.every((s) => typeof s === 'string' && !s.startsWith('/') && !s.split(/[\\/]/).some((p) => !p || p === '..' || p === '.')))
      problems.push(['extensions.badSkills'])
    else if (new Set(skills.map((s) => s.split(/[\\/]/).pop())).size < skills.length) problems.push(['extensions.badSkills'])
    const panels = contributes.panels ?? []
    if (!Array.isArray(panels) || !panels.every((p) => isObject(p) && typeof p.id === 'string' && ID.test(p.id) && isText(p.title) && inside(p.page, '.html')))
      problems.push(['extensions.badPanels'])
    else if (new Set(panels.map((p) => p.id)).size < panels.length) problems.push(['extensions.badPanels'])
    const panelIds = Array.isArray(panels) ? panels.flatMap((p) => (isObject(p) && typeof p.id === 'string' ? [p.id] : [])) : []
    const items = contributes.items ?? []
    if (!Array.isArray(items)) problems.push(['extensions.badItems'])
    else {
      const seen: string[] = []
      items.forEach((item, index) => {
        const at = { item: isObject(item) && typeof item.id === 'string' ? item.id : `#${index + 1}` }
        if (!isObject(item)) return problems.push(['extensions.badItem', at])
        if (typeof item.id !== 'string' || !ID.test(item.id)) problems.push(['extensions.badItemId', at])
        else if (seen.includes(item.id)) problems.push(['extensions.twiceItem', at])
        else seen.push(item.id)
        if (!EXTENSION_SLOTS.includes(item.slot as Slot)) problems.push(['extensions.badSlot', { ...at, slots: EXTENSION_SLOTS.join(', ') }])
        if (!isText(item.label)) problems.push(['extensions.noLabel', at])
        if (item.hint !== undefined && !isText(item.hint)) problems.push(['extensions.badHint', at])
        if (item.icon !== undefined && !ICONS.includes(item.icon as string)) problems.push(['extensions.badIcon', { ...at, icons: ICONS.join(', ') }])
        if (item.words !== undefined && (!Array.isArray(item.words) || !item.words.every(oneLine))) problems.push(['extensions.badWords', at])
        if (item.spotlight !== undefined && typeof item.spotlight !== 'boolean') problems.push(['extensions.badSpotlight', at])
        const wrong = actionProblem(item.action, item.slot as Slot, wasm, panelIds)
        if (wrong) problems.push([wrong[0], { ...at, ...wrong[1] }])
      })
    }
    const tools = contributes.tools ?? []
    if (!Array.isArray(tools)) problems.push(['extensions.badTools'])
    else {
      const seen: string[] = []
      tools.forEach((tool, index) => {
        const at = { tool: isObject(tool) && typeof tool.name === 'string' ? tool.name : `#${index + 1}` }
        if (!isObject(tool)) return problems.push(['extensions.badTool', at])
        if (typeof tool.name !== 'string' || !TOOL.test(tool.name)) problems.push(['extensions.badToolName', at])
        else if (seen.includes(tool.name)) problems.push(['extensions.twiceTool', at])
        else seen.push(tool.name)
        if (typeof tool.description !== 'string' || !tool.description.trim()) problems.push(['extensions.noToolDescription', at])
        if (tool.command === undefined ? !wasm : !oneLine(tool.command)) problems.push([tool.command === undefined ? 'extensions.toolNeedsWasm' : 'extensions.badToolCommand', at])
        if (tool.input !== undefined && !(isObject(tool.input) && tool.input.type === 'object')) problems.push(['extensions.badToolInput', at])
      })
    }
    const secrets = contributes.secrets ?? []
    if (!Array.isArray(secrets) || !secrets.every((s) => isObject(s) && typeof s.id === 'string' && ID.test(s.id) && isText(s.label) && (s.hint === undefined || isText(s.hint))))
      problems.push(['extensions.badSecrets'])
    else if (new Set(secrets.map((s) => s.id)).size < secrets.length) problems.push(['extensions.badSecrets'])
    const theme = contributes.theme ?? {}
    if (!isObject(theme) || Object.entries(theme).some(([k, v]) => !/^[a-z0-9-]+$/.test(k) || typeof v !== 'string' || !COLOR.test(v.trim())))
      problems.push(['extensions.badTheme'])
  }
  return problems.length ? { problems } : { manifest: raw as Manifest, problems }
}

export const EXEC = 'exec:'

export const NET = 'net:'

export const ARGS = 'args:'

export const AGENTS = 'agents:'

export const SKILLS = 'skills:'

export const commandsOf = (manifest: Manifest) => [
  ...new Set([
    ...(manifest.contributes?.items ?? []).flatMap((i) => ('terminal.run' in i.action ? [i.action['terminal.run']] : [])),
    ...(manifest.contributes?.tools ?? []).flatMap((t) => (t.command ? [t.command] : [])),
    ...(manifest.permissions?.exec ?? []).map((p) => EXEC + p),
    ...(manifest.permissions?.network ?? []).map((h) => NET + h),
    ...(manifest.permissions?.agents ?? []).map((a) => AGENTS + a),
    ...(manifest.contributes?.args?.length ? [ARGS + JSON.stringify(manifest.contributes.args)] : []),
    ...(manifest.contributes?.skills?.length ? [SKILLS + JSON.stringify(manifest.contributes.skills)] : []),
  ]),
]

export const grantsOf = (extension: Extension) => extension.commands.filter((c) => [EXEC, NET, AGENTS].some((p) => c.startsWith(p)) && !extension.pending.includes(c))

export function load(found: Found[], disabled: string[], approved: Record<string, string[]> = {}): Extension[] {
  const taken: string[] = []
  return found.map(({ dir, linked, manifest: raw, error }) => {
    const id = isObject(raw) && typeof raw.id === 'string' ? raw.id : (dir.split(/[\\/]/).pop() ?? dir)
    const { manifest, problems } = error ? { manifest: undefined, problems: [['extensions.unreadable', { error }] as Problem] } : validate(raw, taken)
    if (manifest) taken.push(manifest.id)
    const commands = manifest ? commandsOf(manifest) : []
    return { dir, linked, id, enabled: !disabled.includes(id), manifest, problems, commands, pending: commands.filter((c) => !approved[id]?.includes(c)) }
  })
}

export const active = (extensions: Extension[]) => extensions.filter((m): m is Extension & { manifest: Manifest } => m.enabled && !!m.manifest)

export const itemsOf = (extensions: Extension[], slot: Slot): Item[] =>
  active(extensions).flatMap((m) =>
    (m.manifest.contributes?.items ?? [])
      .filter((i) => i.slot === slot)
      .map((i) => ({ ...i, key: `${m.manifest.id}.${i.id}`, extension: m.manifest.id, allowed: !('terminal.run' in i.action) || !m.pending.includes(i.action['terminal.run']) })),
  )

export const spotlightOf = (extensions: Extension[]): Item[] =>
  EXTENSION_SLOTS.filter((slot) => slot !== 'artifact.actions')
    .flatMap((slot) => itemsOf(extensions, slot))
    .filter((i) => i.spotlight !== false)

export const needsAgent = (item: ExtensionItem) => NEEDS_AGENT.some((kind) => kind in item.action)

export const toolsOf = (extensions: Extension[]): Exposed[] =>
  active(extensions).flatMap((m) =>
    (m.manifest.contributes?.tools ?? [])
      .filter((t) => !t.command || !m.pending.includes(t.command))
      .map((t) => ({ ...t, name: `${m.manifest.id}_${t.name}`, tool: t.name, extension: m.manifest.id })),
  )

export const argsOf = (extensions: Extension[]) => active(extensions).flatMap((m) => (m.commands.some((c) => c.startsWith(ARGS) && !m.pending.includes(c)) ? (m.manifest.contributes?.args ?? []) : []))

export const skillsOf = (extensions: Extension[]) =>
  active(extensions).flatMap((m) => (m.commands.some((c) => c.startsWith(SKILLS) && !m.pending.includes(c)) ? [{ id: m.manifest.id, dir: m.dir, skills: m.manifest.contributes?.skills ?? [] }] : []))

export const panelsOf = (extensions: Extension[]): Panel[] =>
  active(extensions).flatMap((m) => (m.manifest.contributes?.panels ?? []).map((p) => ({ ...p, key: `extension:${m.manifest.id}.${p.id}` as const, extension: m.manifest.id })))

export const OFFICIAL = 'Jgrasp/squadron-extensions'

export type CatalogEntry = {
  id: string
  version: string
  name: Text
  description?: Text
  repository: string
  archive: string
  sha256: string
  permissions?: Permissions
  source: string
  official: boolean
}

export type Source = { source: string; official: boolean; error?: string }

const entryOf = (e: unknown): e is Omit<CatalogEntry, 'source' | 'official'> =>
  isObject(e) && typeof e.id === 'string' && ID.test(e.id) && typeof e.version === 'string' && isText(e.name) && typeof e.archive === 'string' && typeof e.sha256 === 'string'

export function catalogOf(raw: unknown): { entries: CatalogEntry[]; sources: Source[] } {
  const entries: CatalogEntry[] = []
  const sources: Source[] = []
  for (const s of Array.isArray(raw) ? raw : []) {
    if (!isObject(s) || typeof s.source !== 'string') continue
    const official = s.official === true
    sources.push({ source: s.source, official, error: typeof s.error === 'string' ? s.error : undefined })
    const found = isObject(s.index) && Array.isArray(s.index.extensions) ? s.index.extensions.filter(entryOf) : []
    entries.push(...found.map((e) => ({ ...e, source: s.source as string, official })))
  }
  return { entries, sources }
}

export const originOf = (origins: Record<string, string>, id: string) => origins[id] ?? OFFICIAL

export const shownOf = (entries: CatalogEntry[], origins: Record<string, string>) =>
  [...new Set(entries.map((e) => e.id))].map((id) => entries.find((e) => e.id === id && e.source === origins[id]) ?? entries.find((e) => e.id === id)!)

export function newer(a: string, b: string) {
  const [x, y] = [a, b].map((v) => v.split('.').map((n) => parseInt(n) || 0))
  for (let i = 0; i < Math.max(x.length, y.length); i++) if ((x[i] ?? 0) !== (y[i] ?? 0)) return (x[i] ?? 0) > (y[i] ?? 0)
  return false
}

export const installedOf = (list: Extension[], id: string) => list.find((e) => e.id === id && !e.linked)

export const updatesOf = (list: Extension[], entries: CatalogEntry[], origins: Record<string, string>) =>
  entries.filter((c) => {
    const here = installedOf(list, c.id)?.manifest
    return !!here && originOf(origins, c.id) === c.source && newer(c.version, here.version)
  })

export const themeOf = (extensions: Extension[]) => Object.assign({}, ...active(extensions).map((m) => m.manifest.contributes?.theme ?? {})) as Record<string, string>
