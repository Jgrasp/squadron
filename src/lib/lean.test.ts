import { describe, expect, it } from 'vitest'
import { missing, toolsFor } from './lean'

describe('tools a user message turns on', () => {
  it('reads the slash commands at the start only', () => {
    expect(toolsFor('/loop 5m /babysit')).toEqual(['ScheduleWakeup'])
    expect(toolsFor('  /loop')).toEqual(['ScheduleWakeup'])
    expect(toolsFor('/code-review high 42')).toEqual(['ReportFindings'])
    expect(toolsFor('/looping')).toEqual([])
    expect(toolsFor('run /loop later')).toEqual([])
    expect(toolsFor('/code-reviewer')).toEqual([])
  })

  it('sees an explicit ask for a workflow, in English or French', () => {
    expect(toolsFor('fix it, ultracode')).toEqual(['Workflow'])
    expect(toolsFor('Use a workflow to review every file')).toEqual(['Workflow'])
    expect(toolsFor('run a multi-agent workflow')).toEqual(['Workflow'])
    expect(toolsFor('lance un workflow sur les 18 écrans')).toEqual(['Workflow'])
    expect(toolsFor('utilise un workflow')).toEqual(['Workflow'])
    expect(toolsFor('fan out agents over the repo')).toEqual(['Workflow'])
    expect(toolsFor('orchestrate this with subagents')).toEqual(['Workflow'])
  })

  it('leaves CI workflows alone', () => {
    expect(toolsFor('the Tag workflow failed on main')).toEqual([])
    expect(toolsFor('le workflow Registry est rouge')).toEqual([])
    expect(toolsFor('edit .github/workflows/ci.yml')).toEqual([])
    expect(toolsFor('run the workflow again, it was flaky')).toEqual([])
    expect(toolsFor('a bug with the workflow')).toEqual([])
  })

  it('combines and keeps only what is not on yet', () => {
    expect(toolsFor('/code-review ultracode')).toEqual(['ReportFindings', 'Workflow'])
    expect(missing(['Workflow'], ['ReportFindings', 'Workflow'])).toEqual(['ReportFindings'])
    expect(missing(undefined, ['Workflow'])).toEqual(['Workflow'])
  })
})
