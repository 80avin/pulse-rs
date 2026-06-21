<script lang="ts">
  let { label, children }: { label: string; children: import('svelte').Snippet } = $props();
</script>

<div class="p-3 rounded bg-[var(--color-bg-1)] border border-[var(--color-bd-0)]">
  <div class="uppercase text-[10px] leading-none font-mono text-[var(--color-ink-3)] tracking-[0.6px] mb-2.5">{label}</div>
  {@render children()}
</div>
