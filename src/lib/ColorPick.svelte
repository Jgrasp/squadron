<script lang="ts">
  const COLORS = [
    ['#F472B6', 'Pink'],
    ['#A78BFA', 'Purple'],
    ['#2DD4BF', 'Teal'],
    ['#60A5FA', 'Blue'],
    ['#FACC15', 'Yellow'],
    ['#4ADE80', 'Green'],
    ['#A3E635', 'Lime'],
    ['#F87171', 'Red'],
    ['#A8A29E', 'Grey'],
  ]

  let { value, label, onpick }: { value: string; label: string; onpick: (color: string) => void } = $props()

  const custom = $derived(!!value && !COLORS.some(([c]) => c.toLowerCase() === value.toLowerCase()))
</script>

<span class="colors" role="group" aria-label={label}>
  {#each COLORS as [color, name] (color)}
    <button
      type="button"
      class="swatch"
      style:background={color}
      style:--ring={color}
      aria-label={name}
      aria-pressed={value.toLowerCase() === color.toLowerCase()}
      onclick={() => onpick(color)}
    ></button>
  {/each}
  <label class="swatch custom" class:on={custom} style:--custom={custom ? value : null} style:--ring={custom ? value : null} title="Custom color">
    <input type="color" value={value || '#A78BFA'} aria-label="Custom color" onchange={(e) => onpick(e.currentTarget.value.toUpperCase())} />
  </label>
</span>

<style>
  .colors {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }

  .swatch {
    width: 24px;
    height: 24px;
    border-radius: 5px;
  }

  .custom {
    position: relative;
    overflow: hidden;
    background: var(--custom, conic-gradient(#f472b6, #facc15, #4ade80, #60a5fa, #a78bfa, #f472b6));
    cursor: pointer;
  }

  .custom input {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    opacity: 0;
    cursor: pointer;
  }

  .custom.on,
  .swatch[aria-pressed='true'] {
    box-shadow:
      0 0 0 2px var(--surface-1),
      0 0 0 4px var(--ring);
  }
</style>
