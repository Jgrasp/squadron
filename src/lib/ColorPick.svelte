<script lang="ts">
  import { t, type Key } from './i18n/index.svelte'

  const COLORS: [string, Key][] = [
    ['#F472B6', 'settings.colorPink'],
    ['#A78BFA', 'settings.colorPurple'],
    ['#2DD4BF', 'settings.colorTeal'],
    ['#60A5FA', 'settings.colorBlue'],
    ['#FACC15', 'settings.colorYellow'],
    ['#4ADE80', 'settings.colorGreen'],
    ['#A3E635', 'settings.colorLime'],
    ['#F87171', 'settings.colorRed'],
    ['#A8A29E', 'settings.colorGrey'],
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
      aria-label={t(name)}
      aria-pressed={value.toLowerCase() === color.toLowerCase()}
      onclick={() => onpick(color)}
    ></button>
  {/each}
  <label class="swatch custom" class:on={custom} style:--custom={custom ? value : null} style:--ring={custom ? value : null} title={t('settings.customColor')}>
    <input type="color" value={value || '#A78BFA'} aria-label={t('settings.customColor')} onchange={(e) => onpick(e.currentTarget.value.toUpperCase())} />
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
