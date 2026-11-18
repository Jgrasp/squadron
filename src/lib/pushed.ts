import { calls, type Agent } from './stream'

const LINK = /https?:\/\/(?:www\.)?(?:github\.com\/[\w.-]+\/[\w.-]+\/pull|bitbucket\.org\/[\w.-]+\/[\w.-]+\/pull-requests|[\w.-]+(?:\/[\w.-]+){2,}\/-\/merge_requests|(?:dev\.azure\.com\/[\w.-]+|[\w-]+\.visualstudio\.com(?:\/DefaultCollection)?)\/[\w.%-]+\/_git\/[\w.%-]+\/pullrequest)\/\d+/g

type Found = { url: string; at: number }

export function pushedTo(agent: Agent, more: (text: string) => Found[] = () => []): { url: string } | { branch: string } | null {
  for (const call of calls(agent).reverse()) {
    if (!call.result || call.result.error) continue
    const command = call.name === 'Bash' ? String(call.input.command ?? '') : ''
    if (/\b(gh\s+pr|glab\s+mr|az\s+repos\s+pr)\s+create\b/.test(command) || /create.*(pull|merge.?request)|(pull|merge.?request).*create/i.test(call.name)) {
      const text = call.result.text
      const found = [...text.matchAll(LINK)].map((m) => ({ url: m[0], at: m.index })).concat(more(text))
      const url = found.sort((a, b) => a.at - b.at).at(-1)?.url
      if (url) return { url }
    }
    if (/\bgit\s+(switch|checkout)\b/.test(command)) return null
    const push = command.match(/\bgit\s+push\b([^|&;\n]*)/)
    if (!push) continue
    const [, ref] = push[1].split(/\s+/).filter((word) => word && !word.startsWith('-') && !/^\d*>/.test(word))
    const branch = ref?.split(':').at(-1)?.replace(/^\+/, '').replace(/^refs\/heads\//, '')
    return branch && branch !== 'HEAD' ? { branch } : null
  }
  return null
}
