const URL = /https?:\/\/[^\s<>"'`]+/g

export function urlsIn(text: string) {
  const found: { from: number; to: number; url: string }[] = []
  for (const m of text.matchAll(URL)) {
    let url = m[0].replace(/[.,;:!?]+$/, '')
    while (url.endsWith(')') && url.split('(').length < url.split(')').length) url = url.slice(0, -1).replace(/[.,;:!?]+$/, '')
    if (url.length > url.indexOf('//') + 2) found.push({ from: m.index, to: m.index + url.length, url })
  }
  return found
}

export function splitUrls(text: string) {
  const parts: { text: string; url?: string }[] = []
  let last = 0
  for (const { from, to, url } of urlsIn(text)) {
    if (from > last) parts.push({ text: text.slice(last, from) })
    parts.push({ text: url, url })
    last = to
  }
  if (last < text.length) parts.push({ text: text.slice(last) })
  return parts
}
