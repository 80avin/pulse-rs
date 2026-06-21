<script lang="ts">
  import { T } from '$lib/tokens';
  let { value, w = 22, color }: { value: number; w?: number; color?: string } = $props();
  const c = $derived(color ?? (value >= 0.8 ? T.cyan : value >= 0.5 ? T.amber : T.ink3));
  const pct = $derived(Math.round(value * 100));
</script>

<span class="inline-flex items-center gap-1 text-[10px] leading-none font-mono" style="color:{c};">
  <span class="h-1 rounded-[1px] relative overflow-hidden bg-ink-4" style="width:{w}px;">
    <span class="absolute left-0 top-0 bottom-0" style="width:{pct}%;background:{c};"></span>
  </span>
  <span class="tabular-nums">{value.toFixed(2)}</span>
</span>
