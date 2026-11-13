<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import Dialog from './Dialog.svelte'
  import ExtensionLogo from './ExtensionLogo.svelte'
  import ExtensionSettingsFrame from './ExtensionSettingsFrame.svelte'
  import Icon from './Icon.svelte'
  import { ui } from './actions.svelte'
  import { ask } from './asking.svelte'
  import { keepLabel } from './cleanup.svelte'
  import { errorText, t } from './i18n/index.svelte'
  import type { Account, Extension, ExtensionOption, OptionValue, Text } from './extensions'
  import { approve, commandLine, extensions, install, removeAccount, saveAccount, setOption, uninstall } from './extensions.svelte'
  import { attempt, fromOf, named, nameOf, originOf, pendingOf, problemOf, updateOf, work } from './extension-pages.svelte'
  import { home } from './projects.svelte'
  import { saveSettings, settings } from './settings.svelte'

  let { id }: { id: string } = $props()

  const e = $derived(extensions.list.find((x) => x.id === id))
  const update = $derived(e && updateOf(e))
  const contributes = $derived(e?.manifest?.contributes)
  const cleanup = $derived(contributes?.items?.flatMap((i) => ('artifact.share' in i.action && i.action['artifact.share'].cleanup ? [i.action['artifact.share'].cleanup.days] : []))[0])
  const options = $derived(contributes?.settings?.options ?? [])
  const page = $derived(contributes?.settings?.page)
  const values = $derived(extensions.options[id] ?? {})
  const empty = $derived(!contributes?.secrets?.length && !contributes?.accounts && cleanup === undefined && !options.length && !page)

  const change = (o: ExtensionOption, value: OptionValue) => (typeof value === 'number' && Number.isNaN(value) ? undefined : attempt(() => setOption(id, o.id, value)))

  let about = $state(false)
  let saved = $state<string[]>([])
  let typed = $state<Record<string, string>>({})
  let draft = $state<{ account: string | null; values: Record<string, string> } | null>(null)

  $effect(() => {
    void id
    about = false
    draft = null
    typed = {}
    work.error = ''
  })

  $effect(() => {
    const names = (contributes?.secrets ?? []).map((s) => s.id)
    if (names.length) invoke<string[]>('extensions_secrets', { id, names }).then((found) => (saved = found), (err) => (work.error = errorText(err)))
  })

  const from = $derived.by(() => {
    if (!e) return ''
    const kind = fromOf(e)
    return kind === 'local' ? t('extensions.fromLocal') : t(kind === 'official' ? 'extensions.fromOfficial' : 'extensions.fromImported', { source: originOf(e) ?? '' })
  })

  const adds = $derived([
    ...(contributes?.items ?? []).map((i) => [named(i.label), i.slot]),
    ...(contributes?.panels ?? []).map((p) => [named(p.title), t('extensions.addsPanel')]),
    ...(contributes?.skills ?? []).map((s) => [s.split(/[\\/]/).pop() ?? s, t('extensions.addsSkill')]),
  ])

  const enable = (on: boolean) => attempt(() => invoke('extensions_enable', { id, on }))

  async function keep(name: string, value: string | null) {
    if (!(await attempt(() => invoke('extensions_secret_set', { id, name, value })))) return
    saved = value === null ? saved.filter((n) => n !== name) : [...new Set([...saved, name])]
    typed[name] = ''
  }

  const fields = $derived(contributes?.accounts?.fields ?? [])
  const shownValues = (a: Account) => fields.filter((f) => f.kind !== 'secret').map((f) => a.values[f.id]).filter(Boolean)
  const kept = $derived(draft?.account ? (extensions.accounts[id]?.find((a) => a.id === draft!.account)?.secrets ?? []) : [])
  const complete = $derived(!!draft && fields.every((f) => draft!.values[f.id]?.trim() || kept.includes(f.id)))

  async function keepAccount(event: SubmitEvent) {
    event.preventDefault()
    if (!draft) return
    const { account, values } = $state.snapshot(draft)
    if (await attempt(() => saveAccount(id, account, values))) draft = null
  }

  async function dropAccount(a: Account) {
    const name = shownValues(a).join(' · ') || a.id
    if (!(await ask(t('extensions.accountRemoveText', { name }), { title: t('extensions.accountRemoveTitle'), kind: 'warning', okLabel: t('common.remove') }))) return
    draft = null
    attempt(() => removeAccount(id, a.id))
  }

  function setCleanup(value: string) {
    const { [id]: _, ...rest } = settings.artifacts_extensions
    saveSettings({ ...$state.snapshot(settings), artifacts_extensions: value === 'own' ? rest : { ...rest, [id]: Number(value) } }).catch((err) => (work.error = errorText(err)))
  }

  async function remove(x: Extension) {
    if (x.linked) {
      about = false
      if (await attempt(() => invoke('extensions_unlink', { path: x.dir }))) ui.settings = 'extensions'
      return
    }
    const name = nameOf(x)
    if (!(await ask(t('extensions.uninstallText', { name }), { title: t('extensions.uninstallTitle', { name }), kind: 'warning', okLabel: t('extensions.uninstall') }))) return
    about = false
    if (await attempt(() => uninstall(x))) ui.settings = 'extensions'
  }

  const reveal = (x: Extension) => attempt(() => invoke('reveal_attachment', { path: x.dir }))
</script>

<div class="page">
{#if e}
  {@const problem = problemOf(e)}
  {@const pending = pendingOf(e)}
  <div class="head" class:off={!e.enabled}>
    <ExtensionLogo id={e.id} name={nameOf(e)} size={40} />
    <div class="title">
      <span class="l1">
        <strong>{nameOf(e)}</strong>
        {#if e.manifest}
          <input class="switch" type="checkbox" role="switch" checked={e.enabled} aria-label={t('extensions.enabledFor', { name: nameOf(e) })} onchange={(event) => enable(event.currentTarget.checked)} />
        {/if}
      </span>
      <span class="meta">
        <span class="mono">{e.manifest?.version ?? ''}</span>
        <i>·</i>
        <span class:ok={fromOf(e) === 'official'}>{fromOf(e) === 'official' ? `✓ ${t('extensions.official')}` : from}</span>
        {#if e.linked}<i>·</i><span class="mono">{home(e.dir)}</span>{/if}
      </span>
    </div>
    <div class="actions">
      {#if update}
        <button class="btn btn-secondary" disabled={!!work.busy} onclick={() => attempt(() => install(update), e.id)}><Icon name="arrow-up" />{work.busy === e.id ? t('extensions.updating') : t('extensions.update', { version: update.version })}</button>
      {/if}
      {#if e.linked}
        <button class="btn btn-ghost" onclick={() => reveal(e)}><Icon name="folder" />{t('extensions.openItsFolder')}</button>
      {/if}
      <button class="btn btn-ghost" onclick={() => (about = true)}><Icon name="info" />{t('extensions.about')}</button>
    </div>
  </div>
  {#if e.manifest?.description}<p class="desc">{named(e.manifest.description)}</p>{/if}

  {#if problem}<p class="bad" role="alert">{problem}</p>{/if}
  {#if work.error && !about}<p class="bad" role="alert">{work.error}</p>{/if}
  {#if pending.length}
    <div class="banner wait">
      <span>{t('extensions.wants', { name: nameOf(e), list: pending.join(', ') })}</span>
      <button class="btn btn-primary" onclick={() => attempt(() => approve(e))}>{t('extensions.allow')}</button>
    </div>
  {/if}
  {#if e.manifest && !e.enabled}
    <div class="notice">
      <span>{t('extensions.offNote')}</span>
      <button class="btn btn-secondary" onclick={() => enable(true)}>{t('extensions.turnOn')}</button>
    </div>
  {/if}

  {#snippet link(field: string, hint: Text | undefined)}
    <button type="button" class="link" onclick={() => invoke('extensions_field_link', { id, field }).catch((err) => (work.error = errorText(err)))}>{hint ? named(hint) : t('extensions.fieldLink')}</button>
  {/snippet}

  <div class="settings" class:dim={!e.enabled} inert={!e.enabled}>
    {#each contributes?.secrets ?? [] as s (s.id)}
      {@const isSaved = saved.includes(s.id)}
      <h3>{named(s.label)}</h3>
      <form class="secret" onsubmit={(event) => (event.preventDefault(), keep(s.id, typed[s.id]))}>
        <input class="field mono" type="password" autocomplete="off" spellcheck="false" placeholder={isSaved ? t('extensions.secretSet') : t('extensions.secretUnset')} aria-label={named(s.label)} bind:value={typed[s.id]} />
        <button class="btn btn-secondary" disabled={!typed[s.id]?.trim()}>{t('common.save')}</button>
        {#if isSaved}<button type="button" class="btn danger" onclick={() => keep(s.id, null)}>{t('common.remove')}</button>{/if}
      </form>
      <p class="hint">{#if s.url}{@render link(s.id, s.hint)}{' '}{:else if s.hint}{named(s.hint)}{' '}{/if}{t('extensions.secretHint')}</p>
    {/each}

    {#if contributes?.accounts}
      {@const accounts = extensions.accounts[id] ?? []}
      <h3>{named(contributes.accounts.label)} <span class="n">{accounts.length}</span></h3>
      {#snippet form()}
        <form class="edit" onsubmit={keepAccount}>
          {#each fields as f (f.id)}
            {@const label = named(f.label)}
            <label class:full={f.kind === 'secret'}>
              {label}
              <input
                class="field"
                class:mono={f.kind !== 'text' && f.kind !== undefined}
                type={f.kind === 'secret' ? 'password' : 'text'}
                autocomplete="off"
                spellcheck="false"
                placeholder={kept.includes(f.id) ? t('extensions.accountKept', { label }) : ''}
                bind:value={draft!.values[f.id]}
              />
              {#if f.url}<small>{@render link(f.id, f.hint)}</small>{:else if f.hint}<small>{named(f.hint)}</small>{/if}
            </label>
          {/each}
          <div class="bar">
            {#if draft?.account}
              {@const account = accounts.find((a) => a.id === draft!.account)}
              {#if account}<button type="button" class="btn danger" onclick={() => dropAccount(account)}>{t('common.remove')}</button>{/if}
            {/if}
            <span class="grow"></span>
            <button type="button" class="btn btn-ghost" onclick={() => (draft = null)}>{t('common.cancel')}</button>
            <button class="btn btn-primary" disabled={!complete || !!work.busy}>{draft?.account ? t('common.save') : t('extensions.accountAdd')}</button>
          </div>
        </form>
      {/snippet}
      <div class="accounts">
        {#each accounts as a (a.id)}
          {@const values = shownValues(a)}
          <div class="account" class:editing={draft?.account === a.id}>
            <div class="who">
              <span class="names"><strong>{values[0] ?? a.id}</strong>{#if values.length > 1}<small class="mono">{values.slice(1).join(' · ')}</small>{/if}</span>
              {#if draft?.account !== a.id}<button class="btn btn-ghost" onclick={() => (draft = { account: a.id, values: { ...a.values } })}>{t('common.edit')}</button>{/if}
            </div>
            {#if draft?.account === a.id}{@render form()}{/if}
          </div>
        {/each}
        {#if draft && !draft.account}
          <div class="account editing">{@render form()}</div>
        {:else}
          <button class="new" onclick={() => (draft = { account: null, values: {} })}><Icon name="plus" />{t('extensions.accountNew')}</button>
        {/if}
      </div>
      <p class="hint">{t('extensions.accountsHint')}</p>
    {/if}

    {#if options.length || cleanup !== undefined}
      <h3>{t('extensions.options')}</h3>
      <div class="options">
        {#each options as o (o.id)}
          {@const value = values[o.id]}
          <label class="option">
            <span class="label"><strong>{named(o.label)}</strong>{#if o.hint}<small>{named(o.hint)}</small>{/if}</span>
            {#if o.kind === 'toggle'}
              <input class="switch" type="checkbox" role="switch" checked={value === true} onchange={(event) => change(o, event.currentTarget.checked)} />
            {:else if o.kind === 'select'}
              <select class="field" value={String(value ?? '')} onchange={(event) => change(o, event.currentTarget.value)}>
                {#each o.choices ?? [] as c (c.value)}<option value={c.value}>{named(c.label)}</option>{/each}
              </select>
            {:else if o.kind === 'number'}
              <input class="field mono" type="number" min={o.min} max={o.max} value={value ?? ''} onchange={(event) => change(o, event.currentTarget.valueAsNumber)} />
            {:else}
              <input class="field" value={value ?? ''} spellcheck="false" autocomplete="off" onchange={(event) => change(o, event.currentTarget.value)} />
            {/if}
          </label>
        {/each}
        {#if cleanup !== undefined}
        <label class="option">
          <span class="label"><strong>{t('settings.extensionCleanup')}</strong><small>{t('settings.extensionCleanupHint')}</small></span>
          <select class="field" value={String(settings.artifacts_extensions?.[id] ?? 'own')} onchange={(event) => setCleanup(event.currentTarget.value)}>
            <option value="own">{t('settings.extensionCleanupOwn', { time: keepLabel(cleanup) })}</option>
            <option value="-1">{t('settings.extensionCleanupRule', { time: keepLabel(settings.artifacts_after_days) })}</option>
            {#each [30, 90, 365, 0] as days (days)}<option value={String(days)}>{keepLabel(days)}</option>{/each}
          </select>
        </label>
        {/if}
      </div>
    {/if}

    {#if page && e.enabled}
      <h3>{contributes?.settings?.title ? named(contributes.settings.title) : t('extensions.settingsPage')}</h3>
      <ExtensionSettingsFrame extension={id} {page} title={nameOf(e)} />
    {/if}

    {#if empty}
      <div class="notice"><span>{t('extensions.nothingToSet')}</span></div>
    {/if}
  </div>

  {#if about}
    <Dialog title={nameOf(e)} width={560} onclose={() => ((about = false), (work.error = ''))}>
      <div class="about">
        <dl>
          <dt>{t('extensions.version')}</dt>
          <dd class="mono">{update ? t('extensions.versionNext', { version: e.manifest?.version ?? '', next: update.version }) : (e.manifest?.version ?? '')}</dd>
          <dt>{t('extensions.from')}</dt>
          <dd>{from}</dd>
          <dt>{t('extensions.location')}</dt>
          <dd class="mono">{home(e.dir)}</dd>
        </dl>
        <section>
          <h4>{t('extensions.permissions')}</h4>
          {#each e.commands as line (line)}<p class="line" class:fresh={e.pending.includes(line)}>{commandLine(line)}</p>{:else}<p class="line muted">{t('extensions.nothing')}</p>{/each}
        </section>
        {#if adds.length}
          <section>
            <h4>{t('extensions.addsTitle')}</h4>
            {#each adds as [label, where], i (i)}<p class="line">{label}<span class="slot">{where}</span></p>{/each}
          </section>
        {/if}
        {#if contributes?.tools?.length}
          <section>
            <h4>{t('extensions.toolsTitle')}</h4>
            {#each contributes.tools as tool (tool.name)}<code>{`${e.id}_${tool.name}`}</code>{/each}
          </section>
        {/if}
        {#if work.error}<p class="bad" role="alert">{work.error}</p>{/if}
      </div>
      <footer>
        <button class="btn danger" onclick={() => remove(e)}>{e.linked ? t('extensions.removeLocal') : t('extensions.uninstall')}</button>
        <span class="grow"></span>
        <button class="btn btn-secondary" onclick={() => ((about = false), (work.error = ''))}>{t('common.close')}</button>
      </footer>
    </Dialog>
  {/if}
{/if}
</div>

<style>
  .head {
    display: flex;
    align-items: center;
    gap: 12px;
    padding-top: 6px;
  }

  .head.off :global(.logo),
  .head.off .title strong,
  .head.off .meta {
    opacity: 0.55;
  }

  .title {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .l1 {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .l1 strong {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--text-strong);
    font-size: 16px;
    font-weight: 700;
  }

  .meta {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
    font-size: 12px;
    color: var(--text-dim);
  }

  .meta .mono {
    font-size: 11.5px;
  }

  .meta i {
    font-style: normal;
    color: var(--text-faint);
  }

  .meta .ok {
    color: var(--ok);
  }

  .actions {
    flex: none;
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .btn {
    height: 30px;
    padding: 0 12px;
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }

  .btn-secondary {
    background: var(--surface-4);
  }

  .btn-secondary:hover:not(:disabled) {
    background: var(--surface-5);
  }

  .btn.danger {
    background: #3a2420;
    color: #f8d5cc;
    font-weight: 700;
  }

  .switch {
    appearance: none;
    flex: none;
    position: relative;
    width: 32px;
    height: 18px;
    margin: 0;
    border-radius: 999px;
    background: var(--surface-5);
    cursor: pointer;
    transition: background 0.15s;
  }

  .switch::before {
    content: '';
    position: absolute;
    top: 3px;
    left: 3px;
    width: 12px;
    height: 12px;
    border-radius: 50%;
    background: var(--text-dim);
    transition: transform 0.15s;
  }

  .switch:checked {
    background: var(--accent);
  }

  .switch:checked::before {
    transform: translateX(14px);
    background: var(--ink);
  }

  .switch:focus-visible {
    outline: 2px solid var(--accent-soft);
    outline-offset: 2px;
  }

  .desc {
    margin: 10px 0 0;
    max-width: 720px;
    color: var(--text-muted);
    font-size: 13px;
    line-height: 1.5;
  }

  .bad {
    margin: 12px 0 0;
    color: var(--err-text);
  }

  .banner {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 16px;
    padding: 8px 8px 8px 12px;
    border-radius: 6px;
    background: var(--wait-bg);
    box-shadow: inset 0 0 0 1px var(--wait-border);
    color: var(--wait-text);
  }

  .banner span {
    flex: 1;
    min-width: 0;
  }

  .banner .btn-primary {
    background: var(--accent);
    color: var(--accent-ink);
  }

  .notice {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-top: 16px;
    padding: 12px 14px;
    border-radius: 8px;
    box-shadow: inset 0 0 0 1px var(--border);
    color: var(--text-muted);
    font-size: 12.5px;
  }

  .notice span {
    flex: 1;
  }

  .settings.dim {
    opacity: 0.5;
  }

  h3 {
    display: flex;
    align-items: baseline;
    gap: 8px;
    margin: 24px 0 10px;
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-dim);
  }

  h3 .n {
    font: 11px var(--mono);
    letter-spacing: 0;
    color: var(--text-faint);
  }

  .secret {
    display: flex;
    gap: 6px;
    max-width: 560px;
  }

  .secret .field {
    flex: 1;
    height: 32px;
    padding: 0 10px;
    font-size: 12.5px;
  }

  .hint {
    margin: 8px 0 0;
    color: var(--text-dim);
    font-size: 12px;
    line-height: 1.5;
  }

  .link {
    color: var(--accent-link);
    text-align: left;
  }

  .link:hover {
    color: var(--accent-soft);
  }

  .accounts {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .account {
    padding: 10px 12px;
    border-radius: 4px;
    background: var(--surface-1);
  }

  .account.editing {
    box-shadow: inset 0 0 0 1px var(--border-strong);
  }

  .who {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .names {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  .names strong {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 13px;
    font-weight: 600;
  }

  .names small {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--text-dim);
    font-size: 11.5px;
  }

  .edit {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 10px;
    padding-top: 10px;
  }

  .account.editing .edit:only-child {
    padding-top: 0;
  }

  .edit label {
    display: flex;
    flex-direction: column;
    gap: 4px;
    color: var(--text-dim);
    font-size: 12px;
  }

  .edit label.full {
    grid-column: 1 / -1;
  }

  .edit .field {
    height: 32px;
    padding: 0 10px;
    font-size: 12.5px;
  }

  .edit small {
    color: var(--text-faint);
  }

  .bar {
    grid-column: 1 / -1;
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .grow {
    flex: 1;
  }

  .new {
    height: 38px;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    border: 1px dashed var(--border-dashed);
    border-radius: 4px;
    color: var(--text-2);
    font-size: 12.5px;
  }

  .new:hover {
    border-color: var(--border-focus);
    background: var(--surface-1);
  }

  .options {
    border-radius: 6px;
    background: var(--surface-1);
    box-shadow: inset 0 0 0 1px var(--border);
  }

  .option {
    display: flex;
    align-items: center;
    gap: 14px;
    min-height: 52px;
    padding: 8px 14px;
  }

  .option + .option {
    border-top: 1px solid var(--border);
  }

  .option .label {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  .option strong {
    font-size: 13px;
    font-weight: 600;
  }

  .option small {
    color: var(--text-dim);
    font-size: 12px;
  }

  .option .field {
    width: 260px;
    height: 32px;
    padding: 0 10px;
    font-size: 12.5px;
  }

  .mono {
    font-family: var(--mono);
  }

  .about {
    display: flex;
    flex-direction: column;
    gap: 16px;
    padding: 4px 22px 20px;
  }

  .about .bad {
    margin: 0;
  }

  dl {
    display: grid;
    grid-template-columns: 110px 1fr;
    gap: 6px 12px;
    margin: 0;
    font-size: 12.5px;
  }

  dt {
    color: var(--text-dim);
  }

  dd {
    margin: 0;
    overflow-wrap: anywhere;
    color: var(--text-2);
  }

  dd.mono {
    font-size: 12px;
  }

  h4 {
    margin: 0 0 8px;
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-dim);
  }

  .line {
    margin: 0 0 4px;
    color: var(--text-2);
    font-size: 12.5px;
    overflow-wrap: anywhere;
  }

  .line.fresh {
    color: var(--accent-soft);
    font-weight: 600;
  }

  .line.muted {
    color: var(--text-dim);
  }

  .slot {
    margin-left: 6px;
    font: 11.5px var(--mono);
    color: var(--text-faint);
  }

  code {
    display: inline-block;
    margin: 0 4px 4px 0;
    padding: 1px 6px;
    border-radius: 3px;
    background: var(--surface-3);
    color: var(--text-2);
    font: 12px var(--mono);
  }

  footer {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 16px 22px 20px;
    border-top: 1px solid var(--border);
  }

  footer .btn {
    height: 38px;
    padding: 0 16px;
  }
</style>
