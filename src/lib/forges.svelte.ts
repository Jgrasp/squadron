import { invoke } from '@tauri-apps/api/core'
import { catalog, extensions } from './extensions.svelte'
import { active, forgePattern, shownOf, VERDICTS, type CatalogEntry, type Extension, type Forge, type ForgeCall, type Manifest, type Verdict } from './extensions'
import { nameOf } from './extension-pages.svelte'
import { projects } from './projects.svelte'

export const BUILT_IN: Record<string, string> = { bitbucket: 'Bitbucket', gitlab: 'GitLab', azure: 'Azure DevOps' }

type Provider = Extension & { manifest: Manifest & { contributes: { forge: Forge } } }

export const providers = (): Provider[] => active(extensions.list).filter((e): e is Provider => !!e.manifest.contributes?.forge && !extensions.unloaded.includes(e.id))

export const providerOf = (host: string) => providers().find((e) => e.id === host) ?? null

export function forgeName(host: string) {
  const provider = providerOf(host)
  return provider ? nameOf(provider) : (BUILT_IN[host] ?? host)
}

export function can(host: string, call: ForgeCall) {
  const provider = providerOf(host)
  return !provider || provider.manifest.contributes.forge.calls.includes(call)
}

export const verdictsOf = (host: string): readonly Verdict[] => providerOf(host)?.manifest.contributes.forge.verdicts ?? VERDICTS

export const offers = (host: string, verdict: Verdict) => can(host, 'post') && verdictsOf(host).includes(verdict)

const hostsOf = (e: Extension) => (extensions.accounts[e.id] ?? []).flatMap((a) => (a.host ? [a.host] : []))

const sourcesOf = (e: Provider) => e.manifest.contributes.forge.links.flatMap((p) => forgePattern(p, hostsOf(e))?.source ?? [])

const linkSources = () => providers().flatMap(sourcesOf)

export const linkProvider = (url: string) => providers().find((e) => sourcesOf(e).some((source) => new RegExp(`^(?:${source})(?:[/?#].*)?$`).test(url))) ?? null

export const isProvidedLink = (url: string) => !!linkProvider(url)

export function providedLinks(text: string) {
  return linkSources().flatMap((source) => [...text.matchAll(new RegExp(source, 'g'))].map((m) => ({ url: m[0], at: m.index ?? 0 })))
}

export type Coverage = { path: string; remote: string | null; by: string | null }

export const coverage = $state<Record<string, Coverage>>({})

export async function lookUpCoverage(paths = projects.list.map((p) => p.path)) {
  for (const found of await invoke<Coverage[]>('forge_coverage', { projects: paths }).catch(() => [])) coverage[found.path] = found
}

export const remoteHost = (remote: string) => remote.split('/')[0]

export function offerFor(remote: string, entries: CatalogEntry[] = catalog.entries ?? []): CatalogEntry | null {
  const installed = new Set(extensions.list.map((e) => e.id))
  return (
    shownOf(entries, extensions.origins)
      .filter((entry) => !installed.has(entry.id))
      .find((entry) => (Array.isArray(entry.forge?.remotes) ? entry.forge.remotes : []).some((p) => !p.includes('{account}') && forgePattern(`^(?:${p})$`, [])?.test(remote))) ?? null
  )
}

export type Offer = { entry: CatalogEntry; host: string; projects: string[] }

export function installOffers(): Offer[] {
  const found = new Map<string, Offer>()
  for (const project of projects.list) {
    const covered = coverage[project.path]
    if (!covered?.remote || covered.by) continue
    const entry = offerFor(covered.remote)
    if (!entry) continue
    const offer = found.get(entry.id) ?? { entry, host: remoteHost(covered.remote), projects: [] }
    offer.projects.push(project.name)
    found.set(entry.id, offer)
  }
  return [...found.values()]
}
