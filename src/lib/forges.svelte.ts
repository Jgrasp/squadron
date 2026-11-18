import { extensions } from './extensions.svelte'
import { active, forgePattern, VERDICTS, type Extension, type Forge, type ForgeCall, type Manifest, type Verdict } from './extensions'
import { nameOf } from './extension-pages.svelte'

export const BUILT_IN: Record<string, string> = { bitbucket: 'Bitbucket', github: 'GitHub', gitlab: 'GitLab', azure: 'Azure DevOps' }

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
