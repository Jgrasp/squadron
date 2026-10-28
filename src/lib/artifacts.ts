import { calls, type Agent, type Artifact } from './stream'

export const ARTIFACT_MAX = 500_000
export const ARTIFACT_KINDS: Artifact['kind'][] = ['markdown', 'html', 'svg']
export const DOC_LINK = /^https:\/\/(preview\.)?claude\.ai\/(code\/)?artifact\/[\w-]+$/

export type Doc = { id: string; title: string; kind: 'doc'; link: string }

const EXTENSIONS: Record<Artifact['kind'], string> = { markdown: 'md', html: 'html', svg: 'svg' }

export function fileName(artifact: Artifact) {
  const base = artifact.title
    .normalize('NFD')
    .replace(/[̀-ͯ]/g, '')
    .replace(/[^\w-]+/g, '-')
    .replace(/^-+|-+$/g, '')
    .toLowerCase()
  return `${base || 'artifact'}.${EXTENSIONS[artifact.kind]}`
}

export function docsOf(agent: Agent): Doc[] {
  const linked = new Set((agent.artifacts ?? []).flatMap((a) => (a.link ? [a.link] : [])))
  const hidden = new Set(agent.hiddenDocs ?? [])
  return calls(agent).flatMap((call) => {
    const create = (call.input.container as { create?: { name?: unknown } } | undefined)?.create
    if (!/^mcp__.*Claude_Docs__batch$/.test(call.name) || !create || !call.result || call.result.error) return []
    const link = call.result.text.match(/https:\/\/(preview\.)?claude\.ai\/(code\/)?artifact\/[\w-]+/)?.[0]
    if (!link || linked.has(link) || hidden.has(link)) return []
    linked.add(link)
    return [{ id: link, title: String(create.name ?? '') || link, kind: 'doc' as const, link }]
  })
}

export const sharePrompt = (artifact: Artifact) =>
  `Publish the artifact ${artifact.id} “${artifact.title}” as a Claude Doc on claude.ai with the Claude Docs connector, keeping its content as it is. Then call publish_artifact with id ${artifact.id} and the doc's link, without content, so that I can share it from Squadron.`
