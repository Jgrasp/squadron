import { describe, expect, it } from 'vitest'
import { pushedTo } from './pushed'
import { newAgent, type ToolCall } from './stream'

const ran = (...list: [string, string][]) => {
  const agent = newAgent('a', '/p', 'a', { model: '', effort: '', mode: '' } as never)
  const calls: ToolCall[] = list.map(([command, text], i) => ({ id: String(i), name: 'Bash', input: { command }, result: { text, error: false } }))
  agent.entries.push({ kind: 'agent', blocks: [{ type: 'tools', calls }] } as never)
  return agent
}

describe('pushedTo', () => {
  it('takes the pull request link gh pr create prints', () => {
    const agent = ran(
      ['rtk git push -u origin ia/T41 2>&1 | tail -2', 'ok'],
      ['gh pr create --base dev --head ia/T41 --title "x" 2>&1 | tail -2', 'Warning: 1 uncommitted change\nhttps://github.com/o/r/pull/1559'],
    )
    const github = (text: string) => [...text.matchAll(/https:\/\/github\.com\/[\w.-]+\/[\w.-]+\/pull\/\d+/g)].map((m) => ({ url: m[0], at: m.index }))
    expect(pushedTo(agent, github)).toEqual({ url: 'https://github.com/o/r/pull/1559' })
    expect(pushedTo(agent)).toEqual({ branch: 'ia/T41' })
  })

  it('takes the merge request link glab or az print', () => {
    expect(pushedTo(ran(['glab mr create --fill --yes', 'https://gitlab.corp.fr/team/sub/app/-/merge_requests/12\n']))).toEqual({
      url: 'https://gitlab.corp.fr/team/sub/app/-/merge_requests/12',
    })
    expect(pushedTo(ran(['az repos pr create --source-branch x', 'see https://dev.azure.com/org/My%20Proj/_git/shop/pullrequest/42']))).toEqual({
      url: 'https://dev.azure.com/org/My%20Proj/_git/shop/pullrequest/42',
    })
  })

  it('takes a link an extension recognizes, the last one printed', () => {
    const forge = (text: string) => [...text.matchAll(/https:\/\/git\.acme\.io\/[\w.-]+\/[\w.-]+\/pulls\/\d+/g)].map((m) => ({ url: m[0], at: m.index }))
    const agent = ran(['gh pr create --fill', 'https://github.com/o/r/pull/3\nhttps://git.acme.io/o/r/pulls/9'])
    expect(pushedTo(agent, forge)).toEqual({ url: 'https://git.acme.io/o/r/pulls/9' })
    expect(pushedTo(agent)).toBeNull()
  })

  it('takes the branch of the last push', () => {
    expect(pushedTo(ran(['cd /w && rtk git push -u origin ia/T41 2>&1 | tail -2', 'ok']))).toEqual({ branch: 'ia/T41' })
    expect(pushedTo(ran(['git push --force-with-lease origin HEAD:refs/heads/fix/x', '']))).toEqual({ branch: 'fix/x' })
  })

  it('ignores a push without a branch', () => {
    expect(pushedTo(ran(['git push', ''], ['git push origin HEAD', '']))).toBeNull()
  })

  it('forgets the pushes of the branch it left', () => {
    expect(pushedTo(ran(['git push -u origin ia/A', ''], ['git push origin HEAD', '']))).toBeNull()
    expect(pushedTo(ran(['gh pr create', 'https://github.com/o/r/pull/1655'], ['git checkout -b ia/B origin/dev', '']))).toBeNull()
  })
})
