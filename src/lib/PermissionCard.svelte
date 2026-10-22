<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { structuredPatch } from 'diff'
  import { allow, deny, toolLabel } from './agents.svelte'
  import { EDITS, type Proposal } from './diff'
  import DiffView from './DiffView.svelte'
  import { projects } from './projects.svelte'
  import { ruleFor, ruleLabel } from './rules'
  import { argument, relative, subagentOf, type Agent, type Ask, type Subagent } from './stream'

  let {
    agent,
    ask,
    onlog,
    onpreview,
  }: { agent: Agent; ask: Ask; onlog?: (sub: Subagent) => void; onpreview?: (proposal: Proposal) => void } = $props()

  const edit = $derived(EDITS.includes(ask.tool_name))
  const sub = $derived(subagentOf(agent, ask.tool_use_id))
  const others = $derived(agent.subagents.filter((s) => s.status === 'running' && s !== sub).length)
  const mcp = $derived(ask.tool_name.startsWith('mcp__'))
  const project = $derived(projects.list.find((p) => p.path === agent.project))
  const title = $derived(
    ask.tool_name === 'Bash' ? 'Run a command?' : edit ? 'Edit a file?' : 'Use a tool?',
  )
  const detail = $derived(argument(agent, { name: ask.tool_name, input: ask.input }))
  const params = $derived(
    Object.entries(ask.input)
      .filter(([, v]) => v !== undefined && v !== null && v !== '')
      .map(([k, v]) => [k, typeof v === 'string' ? v : JSON.stringify(v)] as const),
  )
  const always = $derived.by(() => {
    if (edit)
      return `${agent.name} will edit the project files without asking until its session ends. Bash commands will still ask.`
    const rule = ruleFor(ask.tool_name, ask.input)
    if (ask.tool_name === 'Bash')
      return {
        code: ruleLabel(rule),
        text: others
          ? `in ${project?.name}, for all its agents, including the ${others} other running subagent${others > 1 ? 's' : ''}. They may ask the same.`
          : `in ${project?.name}, for all its agents.`,
      }
    if (rule.startsWith('WebFetch(domain:'))
      return `${project?.name} agents will read ${rule.slice(16, -1)} without asking.`
    return `${project?.name} agents will call ${toolLabel(ask.tool_name)} without asking.`
  })

  let patch = $state<ReturnType<typeof structuredPatch> | null>(null)

  function applyEdits(text: string) {
    const edits = ask.tool_name === 'MultiEdit' ? (ask.input.edits ?? []) : [ask.input]
    for (const e of edits) {
      const from = String(e.old_string ?? '')
      const to = String(e.new_string ?? '')
      text = e.replace_all ? text.split(from).join(to) : text.replace(from, () => to)
    }
    return text
  }

  $effect(() => {
    if (!edit) return
    const path = String(ask.input.file_path ?? ask.input.notebook_path ?? '')
    invoke<string | null>('read_project_file', { project: agent.project, path })
      .catch(() => null)
      .then((current) => {
        const before = current ?? ''
        const after = ask.tool_name === 'Write' ? String(ask.input.content ?? '') : applyEdits(before)
        patch = structuredPatch(path, path, before, after, '', '', { context: 2 })
      })
  })

  const changes = $derived.by(() => {
    let add = 0
    let del = 0
    for (const h of patch?.hunks ?? [])
      for (const l of h.lines) {
        if (l[0] === '+') add++
        else if (l[0] === '-') del++
      }
    return { add, del }
  })
</script>

<section class="card" aria-label="Permission request">
  <div class="head">
    <span class="icon" aria-hidden="true">
      {#if ask.tool_name === 'Bash'}
        <svg width="16" height="16" viewBox="0 0 24 24"><path d="M4 17l6-6-6-6M12 19h8" /></svg>
      {:else if edit}
        <svg width="16" height="16" viewBox="0 0 24 24"><path d="M12 20h9M16.5 3.5a2.1 2.1 0 0 1 3 3L7 19l-4 1 1-4z" /></svg>
      {:else}
        <svg width="16" height="16" viewBox="0 0 24 24"><path d="M9 2v6M15 2v6M6 8h12v4a6 6 0 0 1-12 0zM12 18v4" /></svg>
      {/if}
    </span>
    <span class="what">
      <strong>{title}</strong>
      {#if sub}
        <span class="by">
          Requested by subagent #{sub.num} · {sub.name}{#if onlog}{' · '}<button class="log" onclick={() => onlog(sub)}>open its log</button>{/if}
        </span>
      {/if}
      <span class="mono">{edit ? `${ask.tool_name} · ${relative(agent, ask.input.file_path)}` : ask.tool_name === 'Bash' ? `Bash · ${detail}` : toolLabel(ask.tool_name)}</span>
    </span>
    {#if edit && patch}
      <span class="mono"><span class="add">+{changes.add}</span> <span class="del">−{changes.del}</span></span>
    {:else if mcp}
      <span class="tag">MCP server</span>
    {/if}
  </div>

  {#if ask.tool_name === 'Bash' && ask.input.description}
    <p class="hint">{ask.input.description}</p>
  {/if}

  {#if edit}
    {#if patch}
      <div class="diff"><DiffView hunks={patch.hunks} limit={400} /></div>
    {/if}
  {:else if ask.tool_name !== 'Bash'}
    <dl>
      {#each params as [key, value] (key)}
        <dt>{key}</dt>
        <dd class:mono={value.length < 120}>{value.length > 400 ? value.slice(0, 400) + '…' : value}</dd>
      {/each}
    </dl>
  {/if}

  <div class="actions">
    <button class="btn allow" onclick={() => allow(agent, ask)}>Allow</button>
    <button class="btn always" onclick={() => allow(agent, ask, true)}>Always</button>
    <button class="btn refuse" onclick={() => deny(agent, ask)}>Deny</button>
    {#if edit && patch && onpreview}
      <button
        class="btn preview"
        onclick={() => patch && onpreview({ ask: ask.id, path: relative(agent, ask.input.file_path ?? ask.input.notebook_path), hunks: patch.hunks })}
      >
        See in the Diff panel
      </button>
    {/if}
  </div>
  <div class="note">
    <strong>Always</strong>
    <span>{#if typeof always === 'string'}{always}{:else}Allows <code>{always.code}</code> {always.text}{/if}</span>
  </div>
</section>

<style>
  .card {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 16px 18px;
    border-radius: 6px;
    background: var(--wait-bg);
    box-shadow: 0 0 0 1px var(--wait-border);
  }

  .head {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .icon {
    width: 30px;
    height: 30px;
    flex: none;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 4px;
    background: var(--accent);
    color: var(--accent-ink);
  }

  svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .what {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .by {
    font-size: 12.5px;
    color: var(--wait-text-2);
  }

  .log {
    color: var(--accent-link);
    text-decoration: underline;
  }

  .what strong {
    font-weight: 600;
    color: var(--wait-text);
  }

  .what .mono {
    font-size: 12.5px;
    color: var(--wait-text-2);
    overflow-wrap: anywhere;
  }

  .mono {
    font-family: var(--mono);
    font-size: 12.5px;
  }

  .add {
    color: var(--ok);
  }

  .del {
    color: var(--err);
  }

  .tag {
    padding: 3px 8px;
    border-radius: 3px;
    background: var(--wait-btn);
    color: var(--wait-btn-text);
    font-size: 11.5px;
    font-weight: 600;
  }

  .hint {
    margin: -4px 0 0 40px;
    font-size: 12.5px;
    color: var(--wait-muted);
  }

  .diff {
    max-height: 240px;
    overflow: auto;
    border-radius: 4px;
    box-shadow: inset 0 0 0 1px #4a2e29;
  }

  dl {
    margin: 0;
    display: grid;
    grid-template-columns: minmax(70px, auto) 1fr;
    gap: 6px 14px;
    padding: 12px 14px;
    border-radius: 4px;
    background: var(--bg-app);
    box-shadow: inset 0 0 0 1px #4a2e29;
    font-size: 12.5px;
  }

  dt {
    color: var(--text-dim);
  }

  dd {
    margin: 0;
    max-height: 120px;
    overflow: auto;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }

  .actions {
    display: flex;
    gap: 8px;
  }

  .actions .btn {
    height: 36px;
  }

  .allow {
    background: var(--accent);
    color: var(--accent-ink);
    font-weight: 700;
  }

  .always {
    background: var(--wait-btn);
    color: var(--wait-btn-text);
  }

  .refuse {
    background: var(--surface-3);
    color: #e3d7cf;
  }

  .preview {
    margin-left: auto;
    background: transparent;
    box-shadow: inset 0 0 0 1px var(--wait-border);
    color: var(--wait-text-2);
  }

  .note {
    display: flex;
    gap: 8px;
    padding: 10px 12px;
    border-radius: 4px;
    background: var(--wait-note);
    font-size: 12.5px;
    line-height: 1.5;
    color: var(--wait-text-2);
  }

  .note code {
    font-size: 12px;
  }

  .note strong {
    color: var(--wait-btn-text);
  }
</style>
