<script lang="ts">
  import { T, SOURCE_KIND } from '$lib/tokens';
  let { kind, size = 11 }: { kind: string; size?: number } = $props();
  const s = $derived(SOURCE_KIND[kind] ?? SOURCE_KIND.rss);
</script>

<span style="
  display:inline-flex;align-items:center;justify-content:center;
  width:18px;height:12px;
  font:600 {size}px/1 {T.mono};
  color:{s.color};
  letter-spacing:0;
">{s.glyph}</span>
