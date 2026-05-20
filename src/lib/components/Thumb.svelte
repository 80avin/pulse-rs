<script lang="ts">
  import { T } from '$lib/tokens';
  import Icon from './Icon.svelte';
  import type { FeedItem } from '$lib/types';

  let { item, w = 44, h = 44 }: { item: FeedItem; w?: number; h?: number } = $props();

  const hue = $derived(item.thumb?.h ?? 200);
  const bgA = $derived(`oklch(0.30 0.08 ${hue})`);
  const bgB = $derived(`oklch(0.20 0.06 ${hue})`);
  const fg  = $derived(`oklch(0.85 0.10 ${hue})`);

  const iconSize = $derived(Math.round(w * 0.36));
  const labelSize = $derived(Math.max(9, Math.round(w * 0.24)));
  const linkIconSize = $derived(Math.round(w * 0.22));
</script>

{#if item.kind !== 'text' && item.thumb}
  <div style="
    width:{w}px;height:{h}px;flex-shrink:0;
    border-radius:3px;
    border:1px solid {T.bd1};
    background:linear-gradient(135deg,{bgA},{bgB});
    position:relative;overflow:hidden;
    display:flex;align-items:center;justify-content:center;
  ">
    {#if item.kind === 'image' || (item.kind === 'crosspost' && !item.domain)}
      <svg width={w} height={h} style="position:absolute;inset:0;opacity:0.18;">
        <defs>
          <pattern id="p-{item.id}" width="6" height="6" patternUnits="userSpaceOnUse" patternTransform="rotate(45)">
            <line x1="0" y1="0" x2="0" y2="6" stroke={fg} stroke-width="1.5"/>
          </pattern>
        </defs>
        <rect width="100%" height="100%" fill="url(#p-{item.id})"/>
      </svg>
      <Icon name="image" size={iconSize} color={fg} />
    {:else if item.kind === 'video'}
      <Icon name="video" size={Math.round(w * 0.32)} color={fg} />
      {#if item.dur}
        <span style="
          position:absolute;bottom:1px;right:1px;
          background:rgba(0,0,0,0.65);color:{T.ink0};
          font:9px/1 {T.mono};padding:1px 3px;border-radius:2px;
          letter-spacing:0.2px;
        ">{item.dur}</span>
      {/if}
    {:else}
      <div style="font:600 {labelSize}px/1 {T.mono};color:{fg};letter-spacing:0.3px;">
        {item.thumb.label}
      </div>
      <Icon name="link" size={linkIconSize} color={fg} style="position:absolute;bottom:2px;right:2px;opacity:0.7;" />
    {/if}
  </div>
{/if}
