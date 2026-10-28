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
    expect(pushedTo(agent)).toEqual({ url: 'https://github.com/o/r/pull/1559' })
  })

  it('takes the branch of the last push', () => {
    expect(pushedTo(ran(['cd /w && rtk git push -u origin ia/T41 2>&1 | tail -2', 'ok']))).toEqual({ branch: 'ia/T41' })
    expect(pushedTo(ran(['git push --force-with-lease origin HEAD:refs/heads/fix/x', '']))).toEqual({ branch: 'fix/x' })
  })

  it('ignores a push without a branch', () => {
    expect(pushedTo(ran(['git push', ''], ['git push origin HEAD', '']))).toBeNull()
  })
})
