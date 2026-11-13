const BLOCK = /\n*<ticket source="[^"]*" key="([^"]+)"([^>]*)>\n?([\s\S]*?)<\/ticket>/g

export function ticketBlocks(text: string) {
  const blocks = [...text.matchAll(BLOCK)].map((m) => ({ key: m[1], url: m[2].match(/ url="([^"]+)"/)?.[1] ?? '', title: m[3].match(/^# \S+ (.+)/)?.[1] ?? '' }))
  return { text: blocks.length ? text.replace(BLOCK, '').trimEnd() : text, blocks }
}
