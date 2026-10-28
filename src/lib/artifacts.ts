import { calls, type Agent } from './stream'

export type ArtifactKind = 'html' | 'markdown' | 'svg'
export type ArtifactInfo = { id: string; kind: ArtifactKind; entry: string; title: string; modified: number; link: string | null }
export type Doc = { id: string; title: string; kind: 'doc'; link: string }

export const ARTIFACT_MAX = 500_000
export const ARTIFACT_KINDS: ArtifactKind[] = ['markdown', 'html', 'svg']
export const DOC_LINK = /^https:\/\/(preview\.)?claude\.ai\/(code\/)?artifact\/[\w-]+$/

const EXTENSIONS: Record<ArtifactKind, string> = { markdown: 'md', html: 'html', svg: 'svg' }

export function slug(title: string) {
  return title
    .normalize('NFD')
    .replace(/[̀-ͯ]/g, '')
    .replace(/[^\w-]+/g, '-')
    .replace(/^[-_]+|[-_]+$/g, '')
    .toLowerCase()
    .slice(0, 60)
}

export const fileName = (artifact: { title: string; kind: ArtifactKind }) => `${slug(artifact.title) || 'artifact'}.${EXTENSIONS[artifact.kind]}`

export function freeId(title: string, taken: string[]) {
  const base = slug(title) || 'artifact'
  let id = base
  for (let n = 2; taken.includes(id); n++) id = `${base}-${n}`
  return id
}

export function insideArtifacts(root: string, agent: string, path: unknown) {
  if (!root || typeof path !== 'string') return false
  const parts = path.split('/')
  return path.startsWith(`${root}/${agent}/`) && !parts.includes('..') && !parts.includes('.')
}

export function artifactUrl(agent: string, artifact: ArtifactInfo) {
  const custom = (globalThis as { __artifactUrl?: (agent: string, artifact: ArtifactInfo) => string }).__artifactUrl
  if (custom) return custom(agent, artifact)
  const base = navigator.userAgent.includes('Windows') ? 'http://artifact.localhost' : 'artifact://localhost'
  return `${base}/${agent}/${encodeURIComponent(artifact.id)}/${artifact.entry}?v=${artifact.modified}`
}

export function docsOf(agent: Agent, linked: string[] = []): Doc[] {
  const seen = new Set(linked)
  const hidden = new Set(agent.hiddenDocs ?? [])
  return calls(agent).flatMap((call) => {
    const create = (call.input.container as { create?: { name?: unknown } } | undefined)?.create
    if (!/^mcp__.*Claude_Docs__batch$/.test(call.name) || !create || !call.result || call.result.error) return []
    const link = call.result.text.match(/https:\/\/(preview\.)?claude\.ai\/(code\/)?artifact\/[\w-]+/)?.[0]
    if (!link || seen.has(link) || hidden.has(link)) return []
    seen.add(link)
    return [{ id: link, title: String(create.name ?? '') || link, kind: 'doc' as const, link }]
  })
}

export const sharePrompt = (artifact: ArtifactInfo) =>
  `Publish the artifact ${artifact.id} “${artifact.title}” as a Claude Doc on claude.ai with the Claude Docs connector, keeping its content as it is. Then call publish_artifact with id ${artifact.id} and the doc's link, without content, so that I can share it from Squadron.`
