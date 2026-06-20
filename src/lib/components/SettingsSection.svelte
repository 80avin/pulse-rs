<script lang="ts">
  let { label, children }: { label: string; children: import('svelte').Snippet } = $props();
</script>

<div style="padding:12px;background:var(--color-bg-1);border:1px solid var(--color-bd-0);border-radius:4px;">
  <div style="font:10px/1 var(--font-mono);color:var(--color-ink-3);letter-spacing:0.6px;text-transform:uppercase;margin-bottom:10px;">{label}</div>
  {@render children()}
</div>
