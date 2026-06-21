<script lang="ts">
  import { SOURCE_KIND } from '$lib/tokens';
  let { kind, size = 11 }: { kind: string; size?: number } = $props();
  const s = $derived(SOURCE_KIND[kind] ?? SOURCE_KIND.rss);
</script>

<span class="inline-flex items-center justify-center w-4.5 h-3 font-semibold leading-none font-mono" style="font-size:{size}px;color:{s.color};">{s.glyph}</span>
