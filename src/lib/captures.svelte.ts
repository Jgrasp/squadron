export const viewer = $state<{ paths: string[]; index: number }>({ paths: [], index: 0 })

export function showCapture(from: Element, path: string) {
  const gallery = from.closest('[data-gallery]')
  const paths: string[] = []
  for (const el of gallery?.querySelectorAll<HTMLElement>('[data-shot]:not(.gone), [data-captures]') ?? []) {
    const found = el.dataset.captures ? (JSON.parse(el.dataset.captures) as string[]) : [el.dataset.shot!]
    for (const p of found) if (!paths.includes(p)) paths.push(p)
  }
  if (!paths.includes(path)) paths.splice(0, paths.length, path)
  viewer.paths = paths
  viewer.index = paths.indexOf(path)
}
