import { getVersion } from '@tauri-apps/api/app'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { run, ui, type Context } from './actions.svelte'
import { agents, archivable, archive, find, persist, shownAgent } from './agents.svelte'
import { pointed } from './artifacts.svelte'
import { draftClosed, drafts } from './drafts.svelte'
import { parseChips, parseDraft, withChips } from './newAgentDraft'
import { projects } from './projects.svelte'
import { ask, message } from './asking.svelte'
import { notify } from './notify.svelte'
import { errorText, locale, t } from './i18n/index.svelte'
import { layout, setPanel } from './layout.svelte'
import { active, AGENTS, argsOf, PROJECTS, USAGE, ARGS, catalogOf, EXEC, SKILLS, skillsOf, type Source, grantsOf, itemsOf, spotlightOf, load, NET, panelsOf, text, themeOf, toolsOf, updatesOf, type Account, type CatalogEntry, type OptionValue, type Found, type Item, type Extension, type Panel, type Text } from './extensions'
import type { Slot } from './slots'
import { workdir, type Agent } from './stream'
import { waitingKind } from './activity'
import { type as typeInTerminal } from './terminals.svelte'

export const extensions = $state({ list: [] as Extension[], sources: [] as string[], origins: {} as Record<string, string>, accounts: {} as Record<string, Account[]>, options: {} as Record<string, Record<string, OptionValue>>, errors: {} as Record<string, string>, unloaded: [] as string[], shown: {} as Record<string, { badge?: string | null; label?: string | null }> })

export const shownLabel = (key: string) => extensions.shown[key]?.label

export const badgeOf = (key: string) => extensions.shown[key]?.badge

const info = (agent: Agent) => ({ id: agent.id, name: agent.name, project: agent.project, dir: workdir(agent) })

const told = (agent: Agent) => ({ ...info(agent), name: agent.autoName ? '' : agent.name })

export const panelHovered: Record<string, boolean> = {}

const stateOf = (agent: Agent) => ({
  ...info(agent),
  status: agent.status,
  since: Math.round(agent.since),
  pinned: !!agent.pinned,
  remote: !!agent.remote,
  shown: agent.id === shownAgent(),
  branch: agent.worktree?.branch ?? null,
  archivable: archivable(agent),
})

export const commandLine = (key: string) =>
  key.startsWith(EXEC)
    ? t('extensions.anyArgs', { program: key.slice(EXEC.length) })
    : key.startsWith(NET)
      ? t('extensions.network', { host: key.slice(NET.length) })
      : key.startsWith(ARGS)
        ? t('extensions.claudeArgs', { args: (JSON.parse(key.slice(ARGS.length)) as string[]).join(' ') })
        : key.startsWith(SKILLS)
          ? t('extensions.claudeSkills', { skills: (JSON.parse(key.slice(SKILLS.length)) as string[]).map((s) => s.split(/[\\/]/).pop()).join(', ') })
        : key === `${AGENTS}read`
          ? t('extensions.agentsRead')
          : key === `${AGENTS}archive`
            ? t('extensions.agentsArchive')
            : key === `${AGENTS}activity`
              ? t('extensions.agentsActivity')
              : key === `${AGENTS}show`
                ? t('extensions.agentsShow')
                : key === `${PROJECTS}read`
                  ? t('extensions.projectsRead')
                  : key === `${USAGE}read`
                    ? t('extensions.usageRead')
                    : key

const app = getVersion().catch(() => '')

export async function loadExtensions() {
  const listing = await invoke<{ extensions: Found[]; disabled: string[]; approved: Record<string, string[]>; sources: string[]; origins: Record<string, string>; accounts?: Record<string, Account[]>; options?: Record<string, Record<string, OptionValue>> }>('extensions_list')
  extensions.list = load(listing.extensions, listing.disabled, listing.approved, await app)
  extensions.sources = listing.sources
  extensions.origins = listing.origins
  extensions.accounts = listing.accounts ?? {}
  extensions.options = listing.options ?? {}
  for (const [project, kind] of Object.entries(layout.panels)) if (kind.startsWith('extension:') && !panelsOf(extensions.list).some((p) => p.key === kind && !p.window)) setPanel(project, null)
  for (const extension of extensions.list) {
    const key = `${extension.id}\n${extension.pending.join('\n')}`
    if (!extension.enabled || !extension.manifest || extension.problems.length || !extension.pending.length || asked.has(key)) continue
    asked.add(key)
    requestApproval(extension)
  }
}

const asked = new Set<string>()

export async function requestApproval(extension: Extension) {
  if (!extension.manifest) return false
  const name = text(extension.manifest.name, locale())
  const yes = await ask(t('extensions.askText', { name }), {
    title: t('extensions.askTitle', { name }),
    kind: 'warning',
    okLabel: t('extensions.allow'),
    cancelLabel: t('extensions.later'),
    lines: extension.pending.map(commandLine),
  })
  if (yes) await approve(extension)
  return yes
}

listen('extensions-changed', () => loadExtensions())

listen<{ id: string; agent: string; tool: string; args: unknown }>('extensions-tool', async ({ payload: { id, agent, tool, args } }) => {
  const reply = (ok: boolean, text: string) => invoke('extensions_tool_reply', { id, ok, text })
  const found = toolsOf(extensions.list).find((t) => t.name === tool)
  const caller = find(agent)
  if (!found || !caller) return reply(false, found ? 'Only agents started by Squadron can use extension tools.' : `The tool ${tool} is no longer available.`)
  try {
    if (!found.command) return reply(true, await invoke<string>('extensions_wasm_tool', { id: found.extension, name: found.tool, input: JSON.stringify(args ?? {}), agent: info(caller) }))
    const secrets = (extensions.list.find((m) => m.id === found.extension)?.manifest?.contributes?.secrets ?? []).map((s) => s.id)
    const [ok, text] = await invoke<[boolean, string]>('extension_exec', { dir: workdir(caller), command: found.command, id: found.extension, secrets })
    reply(ok, text || '(no output)')
  } catch (e) {
    reply(false, String(e))
  }
})

export const extensionItems = (slot: Slot) => itemsOf(extensions.list, slot)

export const spotlightItems = () => spotlightOf(extensions.list)

export const shareTool = (extension: string, tool: string, input: unknown, agent: Agent) =>
  invoke<string>('extensions_wasm_tool', { id: extension, name: tool, input: JSON.stringify(input), agent: info(agent) })

export const releaseTool = (extension: string, tool: string, input: unknown, agent: string) =>
  invoke<string>('extensions_wasm_tool', { id: extension, name: tool, input: JSON.stringify(input), agent: { id: agent, name: '', project: '', dir: '' } })

export const extensionItem = (slot: Slot, key: string) => extensionItems(slot).find((i) => i.key === key)

export async function runItem(item: Item, { project, agent }: Context) {
  const action = item.action
  if ('squadron' in action) return run(action.squadron, { project, agent })
  if ('url.open' in action) return invoke('open_link', { url: action['url.open'] })
  if ('panel.open' in action) {
    const panel = panelsOf(extensions.list).find((p) => p.extension === item.extension && p.id === action['panel.open'])
    return panel && openPanel(panel, agent?.project ?? project?.path, true)
  }
  if ('run' in action)
    return invoke('extensions_wasm_run', { id: item.extension, action: action.run, agent: agent && info(agent) }).catch((e) => message(errorText(e), { kind: 'error', title: item.extension }))
  if (!agent || 'artifact.share' in action) return
  if ('prompt.insert' in action) pointed[agent.id] = [...(pointed[agent.id] ?? []), action['prompt.insert']]
  else {
    const extension = extensions.list.find((m) => m.id === item.extension)
    if (!item.allowed && !(extension && (await requestApproval(extension)))) return
    typeInTerminal(workdir(agent), `${action['terminal.run']}\r`)
    setPanel(agent.project, 'terminal')
  }
}

export async function openPanel(panel: Panel, project: string | undefined, toggle = false) {
  if (panel.window) return import('./extensionWindows.svelte').then((m) => m.openWindow(panel))
  if (!project) return
  return setPanel(project, toggle && layout.panels[project] === panel.key ? null : panel.key)
}

const posts = new Set<(p: { extension: string; panel: string; message: string }) => void>()

export function onPanelPost(extension: string, panel: string, deliver: (message: unknown) => void) {
  const listener = (p: { extension: string; panel: string; message: string }) => {
    if (p.extension !== extension || p.panel !== panel) return
    try {
      deliver(JSON.parse(p.message))
    } catch {
      deliver(p.message)
    }
  }
  posts.add(listener)
  return () => posts.delete(listener)
}

export const panelSent: Record<string, number> = {}

export function panelMessage(panel: Panel, from: Agent | string | null, message: unknown) {
  panelSent[panel.extension] = Date.now()
  return invoke('extensions_wasm_event', { id: panel.extension, name: 'panel.message', agent: typeof from === 'string' ? { id: '', name: '', project: from, dir: from } : from && told(from), data: JSON.stringify({ panel: panel.id, message: message ?? null }) })
}

export const panelClosed = (panel: Panel) =>
  invoke('extensions_wasm_event', { id: panel.extension, name: 'panel.closed', agent: null, data: JSON.stringify({ panel: panel.id }) }).catch(() => {})

export const SETTINGS_PAGE = ':settings'

export const settingsMessage = (extension: string, message: unknown) =>
  invoke('extensions_wasm_event', { id: extension, name: 'settings.message', agent: null, data: JSON.stringify({ message: message ?? null }) })

export async function setOption(extension: string, option: string, value: OptionValue | null) {
  extensions.options[extension] = await invoke<Record<string, OptionValue>>('extensions_option_set', { id: extension, option, value })
}

export const extensionPanel = (kind: string | null) => panelsOf(extensions.list).find((p) => p.key === kind)

export const catalog = $state({ entries: null as CatalogEntry[] | null, sources: [] as Source[], error: '' })

export async function loadCatalog() {
  try {
    Object.assign(catalog, catalogOf(await invoke('extensions_catalog'), await app), { error: '' })
  } catch (e) {
    catalog.error = errorText(e)
  }
}

export const install = (entry: CatalogEntry) =>
  invoke('extensions_install', { source: entry.source, id: entry.id, archive: entry.archive, sha256: entry.sha256 }).then(loadExtensions)

export const saveAccount = (extension: string, account: string | null, values: Record<string, string>) =>
  invoke<string>('extensions_account_save', { id: extension, account, values }).then(async (saved) => (await loadExtensions(), saved))

export type AccountCheck = { ok: boolean; title: string | Text | null; detail: string | Text | null; field: string | null }

export type CheckState = { running: true } | (AccountCheck & { running?: false; at: number })

export const checks = $state<Record<string, Record<string, CheckState>>>({})

const failed = (e: unknown): AccountCheck => ({ ok: false, title: null, detail: errorText(e), field: null })

export const testAccount = (extension: string, account: string | null, values: Record<string, string>) =>
  invoke<AccountCheck>('extensions_account_check', { id: extension, account, values }).catch(failed)

export function keepCheck(extension: string, account: string, result: AccountCheck) {
  checks[extension] ??= {}
  checks[extension][account] = { ...result, at: Date.now() }
}

export async function checkAccount(extension: string, account: string) {
  checks[extension] ??= {}
  if (checks[extension][account]?.running) return
  checks[extension][account] = { running: true }
  keepCheck(extension, account, await invoke<AccountCheck>('extensions_account_check', { id: extension, account, values: null }).catch(failed))
}

export const checksAccounts = (e: Extension) => e.enabled && !!e.manifest?.wasm && e.manifest.contributes?.accounts?.check === true

export function checkAccounts(e: Extension, unchecked = false) {
  if (!checksAccounts(e)) return
  for (const a of extensions.accounts[e.id] ?? []) if (!unchecked || !checks[e.id]?.[a.id]) void checkAccount(e.id, a.id)
}

export const brokenAccounts = (e: Extension) =>
  checksAccounts(e) &&
  (extensions.accounts[e.id] ?? []).some((a) => {
    const check = checks[e.id]?.[a.id]
    return !!check && !check.running && !check.ok
  })

export const removeAccount = (extension: string, account: string) => invoke('extensions_account_remove', { id: extension, account }).then(loadExtensions)

export const addSource = (source: string) => invoke('extensions_source_add', { source }).then(loadExtensions).then(loadCatalog)

export const removeSource = (source: string) => invoke('extensions_source_remove', { source }).then(loadExtensions).then(loadCatalog)

export async function uninstall(extension: Extension) {
  for (const secret of extension.manifest?.contributes?.secrets ?? []) await invoke('extensions_secret_set', { id: extension.id, name: secret.id, value: null })
  await invoke('extensions_uninstall', { id: extension.id })
  await loadExtensions()
}

export const asks = (entry: CatalogEntry) => [
  ...(entry.permissions?.exec ?? []).map((p) => commandLine(EXEC + p)),
  ...(entry.permissions?.network ?? []).map((h) => commandLine(NET + h)),
  ...(entry.permissions?.agents ?? []).map((a) => commandLine(AGENTS + a)),
  ...(entry.permissions?.projects ?? []).map((a) => commandLine(PROJECTS + a)),
  ...(entry.permissions?.usage ?? []).map((a) => commandLine(USAGE + a)),
]

const TOLD = 'squadron.extension-updates'

export async function checkUpdates() {
  if (getCurrentWindow().label !== 'main') return
  await loadCatalog()
  const told: string[] = JSON.parse(localStorage.getItem(TOLD) ?? '[]')
  const fresh = updatesOf(extensions.list, catalog.entries ?? [], extensions.origins).filter((c) => !told.includes(`${c.id}@${c.version}`))
  if (!fresh.length) return
  localStorage.setItem(TOLD, JSON.stringify([...told, ...fresh.map((c) => `${c.id}@${c.version}`)]))
  for (const c of fresh) await notify('updates', t('extensions.updateTitle'), t('extensions.updateBody', { name: text(c.name, locale()), version: c.version }), { settings: 'discover' })
}

const syncSkills = (list: Extension[]) =>
  invoke<string[]>('extensions_set_skills', { list: skillsOf(list) }).then((errors) => {
    for (const error of errors) console.warn(error)
  })

const syncWasm = (list: Extension[]) =>
  invoke<Record<string, string>>('extensions_wasm_sync', {
    list: active(list)
      .filter((m) => m.manifest.wasm)
      .map((m) => ({
        id: m.id,
        dir: m.dir,
        wasm: m.manifest.wasm,
        allowed: grantsOf(m),
        panels: (m.manifest.contributes?.panels ?? []).map((p) => p.id),
        forge: m.manifest.contributes?.forge ?? null,
      })),
  }).then((errors) => {
    extensions.errors = errors
    extensions.unloaded = Object.keys(errors)
  })

export async function allowed(id: string) {
  const extension = extensions.list.find((m) => m.id === id)
  if (!extension?.pending.length) return !!extension
  if (!(await requestApproval(extension))) return false
  await Promise.all([syncSkills(extensions.list), syncWasm(extensions.list)])
  return true
}

export const approve = (extension: Extension) => invoke('extensions_approve', { id: extension.id, commands: extension.commands }).then(loadExtensions)

listen<{ extension: string; call: string; [key: string]: unknown }>('extensions-host', async ({ payload: p }) => {
  if (p.call === 'error') extensions.errors[p.extension] = String(p.error)
  if (p.call === 'panel-post') for (const post of posts) post({ extension: p.extension, panel: String(p.panel), message: String(p.message) })
  if (p.call === 'settings-post') for (const post of posts) post({ extension: p.extension, panel: SETTINGS_PAGE, message: String(p.message) })
  if (p.call === 'set-badge' || p.call === 'set-label') {
    const key = `${p.extension}.${p.item}`
    extensions.shown[key] = { ...extensions.shown[key], [p.call === 'set-badge' ? 'badge' : 'label']: (p.value as string | null) ?? null }
  }
  if ((p.call === 'agents' || p.call === 'archive-agent') && getCurrentWindow().label === 'main') answerHost(String(p.reply), p.call, String(p.agent))
  if (p.call === 'insert-prompt' && String(p.agent) !== shownAgent()) console.warn(`${p.extension}: insert-prompt only reaches the agent on screen`)
  if (p.call === 'insert-prompt' && find(String(p.agent)) && String(p.agent) === shownAgent()) pointed[String(p.agent)] = [...(pointed[String(p.agent)] ?? []), String(p.text)]
  if (p.call === 'open-new-agent' && getCurrentWindow().label === 'main') openDraft(p.extension, String(p.draft), String(p.agent), String(p.fields))
  if (p.call === 'update-new-agent' && drafts.open?.id === p.draft) drafts.open?.updates.push(parseDraft(String(p.fields)))
  if (p.call === 'set-agent-chips' && getCurrentWindow().label === 'main') setChips(p.extension, String(p.agent), String(p.chips))
  if (p.call === 'notify') await notify('extensions', String(p.title), String(p.body), { settings: `ext:${p.extension}` })
})

function setChips(extension: string, id: string, json: string) {
  const agent = find(id)
  const chips = parseChips(extension, json)
  if (typeof chips === 'string') return console.warn(`${extension}: set-agent-chips: ${chips}`)
  if (!agent) return console.warn(`${extension}: set-agent-chips: no open agent has the id ${id}`)
  const next = withChips(agent.chips ?? [], extension, chips)
  if (JSON.stringify(next) === JSON.stringify(agent.chips ?? [])) return
  agent.chips = next
  persist(agent)
}

function openDraft(extension: string, id: string, from: string, fields: string) {
  const agent = find(from)
  if (agent && agent.id !== shownAgent()) console.warn(`${extension}: open-new-agent only opens from the agent on screen`)
  if (agent ? agent.id !== shownAgent() : !!shownAgent()) return
  const project = agent?.project ?? projects.current
  if (!project || !projects.list.some((p) => p.path === project && p.exists)) return
  if (drafts.open) draftClosed(drafts.open)
  ui.creating = false
  projects.overview = projects.board = projects.pulls = false
  projects.current = project
  drafts.open = { id, extension, project, agent: from, draft: parseDraft(fields), updates: [] }
}

async function answerHost(reply: string, call: string, id: string) {
  const answer = (value: unknown, error: string | null = null) => invoke('extensions_wasm_answer', { reply, value, error })
  if (call === 'agents') return answer(agents.list.map(stateOf))
  const agent = find(id)
  if (!agent) return answer(null, `No open agent has the id ${id}.`)
  if (!archivable(agent)) return answer(null, `${agent.name} stays: it works, waits for the user, is pinned, remote or shown.`)
  try {
    await archive(agent.id)
    answer(null)
  } catch (e) {
    answer(null, errorText(e))
  }
}

const seen = new Map<string, string>()

let themed: string[] = []

$effect.root(() => {
  $effect(() => {
    const theme = themeOf(extensions.list)
    const style = document.documentElement.style
    for (const name of themed) style.removeProperty(`--${name}`)
    themed = Object.keys(theme)
    for (const [name, value] of Object.entries(theme)) style.setProperty(`--${name}`, value)
  })
  $effect(() => {
    if (getCurrentWindow().label !== 'main') return
    invoke('extensions_set_tools', { list: toolsOf(extensions.list).map(({ name, description, input }) => ({ name, description, input })) })
  })
  $effect(() => {
    if (getCurrentWindow().label !== 'main') return
    invoke('extensions_set_args', { args: argsOf(extensions.list) })
  })
  $effect(() => {
    if (getCurrentWindow().label !== 'main') return
    syncSkills(extensions.list)
  })
  $effect(() => {
    if (getCurrentWindow().label !== 'main') return
    syncWasm(extensions.list)
  })
  $effect(() => {
    if (getCurrentWindow().label !== 'main') return
    for (const agent of agents.list) {
      const before = seen.get(agent.id)
      seen.set(agent.id, agent.status)
      if (before === undefined || before === agent.status) continue
      if (agent.status === 'done' || agent.status === 'waiting')
        invoke('extensions_wasm_event', { id: null, name: `agent.${agent.status}`, agent: told(agent), data: JSON.stringify(agent.status === 'waiting' ? { waiting: agent.action, kind: waitingKind(agent) } : {}) })
    }
  })
})
