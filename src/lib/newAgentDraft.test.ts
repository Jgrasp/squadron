import { describe, expect, it } from 'vitest'
import { CHIPS_MAX, fill, parseChips, parseDraft, withChips } from './newAgentDraft'

describe('new agent drafts', () => {
  it('reads what an extension sends and drops the rest', () => {
    expect(parseDraft('not json')).toEqual({})
    expect(parseDraft('[]')).toEqual({})
    expect(
      parseDraft(
        JSON.stringify({
          name: 'RING-128 Card payment',
          text: 42,
          goal: 'Tests pass',
          context: '<ticket source="jira" key="RING-128"># RING-128 Card payment</ticket>',
          link: { key: 'RING-128', title: 'Card payment', url: 'http://acme.atlassian.net/browse/RING-128' },
          choices: { label: { en: 'Context', fr: 'Contexte' }, items: [{ id: 'description', label: 'Description' }, { id: 'comments', label: 'Comments', on: false }, { id: 3 }] },
          status: { state: 'running', text: 'Haiku is preparing the instructions…' },
        }),
      ),
    ).toEqual({
      name: 'RING-128 Card payment',
      goal: 'Tests pass',
      context: '<ticket source="jira" key="RING-128"># RING-128 Card payment</ticket>',
      link: { key: 'RING-128', title: 'Card payment' },
      choices: {
        label: { en: 'Context', fr: 'Contexte' },
        items: [
          { id: 'description', label: 'Description', on: true },
          { id: 'comments', label: 'Comments', on: false },
        ],
      },
      status: { state: 'running', text: 'Haiku is preparing the instructions…' },
    })
    expect(parseDraft('{"link":{"key":"A-1","title":"T","url":"https://a.b/A-1"},"status":{"state":"late"}}')).toEqual({ link: { key: 'A-1', title: 'T', url: 'https://a.b/A-1' } })
    expect(parseDraft('{"link":null,"choices":null,"status":null}')).toEqual({ link: null, choices: null, status: null })
    expect(parseDraft('{"link":{"key":"A-1","title":"T","tone":"ok","logo":false}}')).toEqual({ link: { key: 'A-1', title: 'T', tone: 'ok', logo: false } })
    expect(parseDraft('{"link":{"key":"A-1","title":"T","tone":"red","logo":true}}')).toEqual({ link: { key: 'A-1', title: 'T' } })
  })

  it('reads the chips an extension puts on an agent', () => {
    expect(parseChips('deploy', '[{"key":" Preprod ","title":"Deployed","url":"https://d.acme.dev","tone":"ok","logo":false},{"key":"#12","title":"Issue","url":"http://x"}]')).toEqual([
      { extension: 'deploy', key: 'Preprod', title: 'Deployed', url: 'https://d.acme.dev', tone: 'ok', logo: false },
      { extension: 'deploy', key: '#12', title: 'Issue' },
    ])
    expect(parseChips('deploy', '[]')).toEqual([])
    expect(parseChips('deploy', 'nope')).toBe('chips is not JSON.')
    expect(parseChips('deploy', '{"key":"A"}')).toBe('chips is not a list.')
    expect(parseChips('deploy', '[{"key":"A","title":"T"},{"key":"","title":"T"}]')).toBe('Chip 2 needs a "key" and a "title".')
    expect(parseChips('deploy', JSON.stringify(Array(CHIPS_MAX + 1).fill({ key: 'A', title: 'T' })))).toBe(`${CHIPS_MAX} chips at most.`)
  })

  it('puts an extension\'s chips where its old ones were', () => {
    const jira = { extension: 'jira', key: 'LR-1', title: 'T' }
    const deploy = { extension: 'deploy', key: 'Preprod', title: 'T' }
    const sentry = { extension: 'sentry', key: 'S-1', title: 'T' }
    const staging = { extension: 'deploy', key: 'Staging', title: 'T' }
    expect(withChips([jira, deploy, sentry], 'deploy', [staging, deploy])).toEqual([jira, staging, deploy, sentry])
    expect(withChips([jira, deploy, sentry], 'deploy', [])).toEqual([jira, sentry])
    expect(withChips([jira], 'deploy', [deploy])).toEqual([jira, deploy])
  })

  it('never writes over what the user typed', () => {
    const opened = { name: 'RING-128 Card payment', text: '', goal: '' }
    const typed = { ...opened, goal: 'My own goal' }
    const { form, applied } = fill(typed, opened, { name: 'Other', text: 'Fix 3-D Secure on iOS', goal: 'Payments pass' })
    expect(form).toEqual({ name: 'Other', text: 'Fix 3-D Secure on iOS', goal: 'My own goal' })
    expect(applied).toEqual({ name: 'Other', text: 'Fix 3-D Secure on iOS', goal: '' })
    const edited = { ...form, text: 'Fix 3-D Secure on iOS, start with the logs' }
    expect(fill(edited, applied, { text: 'Second answer', goal: 'Later' }).form).toEqual({ name: 'Other', text: 'Fix 3-D Secure on iOS, start with the logs', goal: 'My own goal' })
    expect(fill(form, applied, {}).form).toEqual(form)
  })
})
