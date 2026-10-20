<script lang="ts">
  type Hunk = { oldStart: number; newStart: number; lines: string[] }

  let { hunks, limit = 1500 }: { hunks: Hunk[]; limit?: number } = $props()

  type Row = { kind: 'head' | 'add' | 'del' | 'same'; old?: number; new?: number; text: string }

  const rows = $derived.by(() => {
    const out: Row[] = []
    for (const hunk of hunks) {
      let old = hunk.oldStart
      let now = hunk.newStart
      const lines = hunk.lines.filter((l) => !l.startsWith('\\'))
      const oldCount = lines.filter((l) => l[0] !== '+').length
      const newCount = lines.filter((l) => l[0] !== '-').length
      if (oldCount > 0) out.push({ kind: 'head', text: `@@ -${hunk.oldStart},${oldCount} +${hunk.newStart},${newCount} @@` })
      for (const line of lines) {
        const text = line.slice(1)
        if (line[0] === '+') out.push({ kind: 'add', new: now++, text })
        else if (line[0] === '-') out.push({ kind: 'del', old: old++, text })
        else out.push({ kind: 'same', old: old++, new: now++, text })
      }
    }
    return out
  })
</script>

<div class="diff">
  {#each rows.slice(0, limit) as row, i (i)}
    {#if row.kind === 'head'}
      <div class="head">{row.text}</div>
    {:else}
      <div class="line {row.kind}">
        <span class="num">{row.old ?? ''}</span>
        <span class="num">{row.new ?? ''}</span>
        <span class="sign">{row.kind === 'add' ? '+' : row.kind === 'del' ? '−' : ''}</span>
        <span class="text">{row.text}</span>
      </div>
    {/if}
  {/each}
  {#if rows.length > limit}
    <div class="head">… {rows.length - limit} lignes de plus</div>
  {/if}
</div>

<style>
  .diff {
    padding: 4px 0;
    background: var(--bg-app);
    font: 12px/20px var(--mono);
    color: var(--text-3);
  }

  .head {
    padding: 0 12px;
    color: var(--text-dim);
  }

  .line {
    display: grid;
    grid-template-columns: 40px 40px 16px 1fr;
  }

  .num {
    padding-right: 4px;
    text-align: right;
    color: #918579;
    user-select: none;
  }

  .sign {
    text-align: center;
    user-select: none;
  }

  .text {
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    padding-right: 12px;
  }

  .add {
    background: var(--ok-bg);
    color: var(--ok-text);
  }

  .add .num {
    color: var(--ok-line);
  }

  .del {
    background: var(--del-bg);
    color: var(--err-text);
  }

  .del .num {
    color: var(--del-line);
  }
</style>
