<script lang="ts">
  import { T } from '$lib/tokens';
  let { value, w = 22, color }: { value: number; w?: number; color?: string } = $props();
  const c = $derived(color ?? (value >= 0.8 ? T.cyan : value >= 0.5 ? T.amber : T.ink3));
  const pct = $derived(Math.round(value * 100));
</script>

<span style="display:inline-flex;align-items:center;gap:4px;font:10px/1 {T.mono};color:{c};">
  <span style="width:{w}px;height:4px;background:{T.ink4};border-radius:1px;position:relative;overflow:hidden;">
    <span style="position:absolute;left:0;top:0;bottom:0;width:{pct}%;background:{c};"></span>
  </span>
  <span style="font-variant-numeric:tabular-nums;">{value.toFixed(2)}</span>
</span>
