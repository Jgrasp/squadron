import { describe, expect, it } from 'vitest'
import { fill, parseDraft } from './newAgentDraft'

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
