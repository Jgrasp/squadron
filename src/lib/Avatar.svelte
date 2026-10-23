<script lang="ts">
  let { name, src = '', size = 24 }: { name: string; src?: string; size?: number } = $props()

  let broken = $state(false)
  const initials = $derived(
    name
      .split(/\s+/)
      .slice(0, 2)
      .map((w) => w[0]?.toUpperCase() ?? '')
      .join(''),
  )
</script>

<span class="avatar" style:--size="{size}px">
  {#if src && !broken}
    <img {src} alt="" referrerpolicy="no-referrer" onerror={() => (broken = true)} />
  {:else}
    {initials}
  {/if}
</span>

<style>
  .avatar {
    flex: none;
    width: var(--size);
    height: var(--size);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    overflow: hidden;
    border-radius: 50%;
    background: var(--surface-4);
    color: var(--text-2);
    font: 700 calc(var(--size) * 0.4) / 1 var(--sans);
  }

  img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
</style>
