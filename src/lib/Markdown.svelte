<script lang="ts">
  let { text }: { text: string } = $props()
</script>

<div class="prose">{text}</div>

<style>
  .prose {
    line-height: 1.65;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }
</style>
