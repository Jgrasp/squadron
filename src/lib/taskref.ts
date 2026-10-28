const PREFIX = /^\s*((?=[A-Z-]*\d)[A-Z][A-Z0-9]*(?:-[A-Z0-9]+)*)\s+[—–:-]\s+/

export const codeOf = (title: string) => PREFIX.exec(title)?.[1]

export const nameOf = (title: string) => title.replace(PREFIX, '')

export const taskId = (href: string) => decodeURIComponent(href.replace(/^squadron:task\//, ''))

export function leads(item: Element, node: Node) {
  for (let at: Node = node; at !== item && at.parentNode; at = at.parentNode)
    for (let before = at.previousSibling; before; before = before.previousSibling) if (before.textContent?.trim()) return false
  return true
}
