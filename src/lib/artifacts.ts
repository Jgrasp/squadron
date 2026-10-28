import type { Artifact } from './stream'

export const ARTIFACT_MAX = 500_000
export const ARTIFACT_KINDS: Artifact['kind'][] = ['markdown', 'html', 'svg']

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
