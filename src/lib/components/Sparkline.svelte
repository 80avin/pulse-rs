<script lang="ts">
  import { T } from '$lib/tokens';
  let { data, w = 60, h = 14, color = T.cyan }: {
    data: number[]; w?: number; h?: number; color?: string;
  } = $props();

  const points = $derived.by(() => {
    if (!data || data.length < 2) return '';
    const max = Math.max(...data);
    const min = Math.min(...data);
    const range = max - min || 1;
    const step = w / (data.length - 1);
    return data.map((v, i) =>
      `${(i * step).toFixed(1)},${(h - ((v - min) / range) * h).toFixed(1)}`
    ).join(' ');
  });
</script>

{#if data && data.length >= 2}
  <svg width={w} height={h} style="display:block;">
    <polyline points={points} fill="none" stroke={color} stroke-width="1" />
  </svg>
{/if}
