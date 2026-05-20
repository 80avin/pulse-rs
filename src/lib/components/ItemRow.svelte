<script lang="ts">
  import { T } from '$lib/tokens';
  import type { FeedItem, Source, Density } from '$lib/types';
  import Icon from './Icon.svelte';
  import TagChip from './TagChip.svelte';
  import ScoreBar from './ScoreBar.svelte';
  import SourceGlyph from './SourceGlyph.svelte';
  import Thumb from './Thumb.svelte';

  let { item, source, isFocused = false, density = 'normal', onclick, onTagClick }: {
    item: FeedItem;
    source: Source | undefined;
    isFocused?: boolean;
    density?: Density;
    onclick: () => void;
    onTagClick?: (tag: string) => void;
  } = $props();

  const dim       = $derived(item.read);
  const padY      = $derived(density === 'dense' ? 8 : density === 'roomy' ? 14 : 11);
  const thumbSize = $derived(density === 'dense' ? 36 : density === 'roomy' ? 56 : 44);
  const hasThumb  = $derived(item.kind !== 'text' && !!item.thumb);
</script>

<div
  role="button"
  tabindex="0"
  {onclick}
  onkeydown={(e) => { if (e.key === 'Enter') onclick(); }}
  style="
    position:relative;display:flex;gap:10px;
    padding:{padY}px 12px {padY}px 14px;
    border-bottom:1px solid {T.bd0};cursor:pointer;
    background:{isFocused ? 'rgba(78,205,214,0.05)' : 'transparent'};
    min-height:56px;
  "
>
  <!-- Unread / focus indicator -->
  <span style="position:absolute;left:0;top:0;bottom:0;width:3px;background:{isFocused ? T.cyan : (item.read ? 'transparent' : T.cyanDim)};"></span>

  <!-- Thumb or source glyph -->
  {#if hasThumb}
    <Thumb {item} w={thumbSize} h={thumbSize} />
  {:else if source}
    <div style="width:{thumbSize}px;height:{thumbSize}px;flex-shrink:0;display:flex;align-items:flex-start;justify-content:center;padding-top:3px;">
      <div style="width:{thumbSize}px;height:{thumbSize}px;background:{T.bg1};border:1px solid {T.bd0};border-radius:3px;display:flex;align-items:center;justify-content:center;">
        <SourceGlyph kind={source.kind} size={13} />
      </div>
    </div>
  {/if}

  <!-- Body -->
  <div style="flex:1;min-width:0;">
    {#if item.kind === 'crosspost' && item.crossFrom}
      <div style="display:inline-flex;align-items:center;gap:4px;font:10px/1 {T.mono};color:{T.ink2};margin-bottom:4px;">
        <Icon name="crosspost" size={10} color={T.violet} />
        <span style="color:{T.violet};">crosspost</span>
        <span style="color:{T.ink3};">from</span>
        <span style="color:{T.ink1};">{item.crossFrom}</span>
      </div>
    {/if}

    <!-- Title -->
    <div style="
      font:{dim ? '400' : '500'} 14px/1.32 {T.sans};
      color:{dim ? T.ink2 : T.ink0};
      overflow:hidden;text-overflow:ellipsis;
      display:-webkit-box;-webkit-line-clamp:{density === 'dense' ? 1 : 2};-webkit-box-orient:vertical;
      letter-spacing:-0.1px;
    ">{item.title}</div>

    <!-- Snippet -->
    {#if item.snippet && density !== 'dense'}
      <div style="margin-top:4px;font:12px/1.4 {T.sans};color:{T.ink2};overflow:hidden;text-overflow:ellipsis;display:-webkit-box;-webkit-line-clamp:2;-webkit-box-orient:vertical;">
        {item.snippet}
      </div>
    {/if}

    <!-- Domain badge -->
    {#if item.domain && density !== 'dense'}
      <div style="margin-top:5px;display:inline-flex;align-items:center;gap:5px;font:10px/1 {T.mono};color:{T.ink2};padding:2px 5px;background:{T.bg1};border:1px solid {T.bd0};border-radius:2px;max-width:100%;">
        <Icon name="link" size={9} color={T.ink3} />
        <span style="overflow:hidden;text-overflow:ellipsis;white-space:nowrap;">{item.domain}</span>
      </div>
    {/if}

    <!-- Meta row -->
    <div style="margin-top:5px;display:flex;align-items:center;gap:6px;font:10px/1 {T.mono};color:{T.ink2};flex-wrap:wrap;">
      {#if source}
        <SourceGlyph kind={source.kind} size={10} />
        <span style="overflow:hidden;text-overflow:ellipsis;white-space:nowrap;max-width:80px;">{source.name}</span>
        <span style="color:{T.ink3};">·</span>
      {/if}
      <span>{item.age}</span>
      {#if item.score > 0}
        <span style="color:{T.ink3};">·</span>
        <span style="color:{T.amber};">▲ {item.score}</span>
      {/if}
      {#if item.n > 0}
        <span style="color:{T.ink3};">·</span>
        <span>{item.n}<span style="color:{T.ink3};">c</span></span>
      {/if}
      {#if item.saved}
        <Icon name="bookmark" size={12} color={T.amber} />
      {/if}
      <span style="flex:1;"></span>
      {#if density !== 'dense'}
        <ScoreBar value={item.aiScore} w={18} />
      {/if}
    </div>

    <!-- Tags -->
    {#if item.tags.length > 0 && density !== 'dense'}
      <div style="margin-top:5px;display:flex;gap:4px;flex-wrap:wrap;">
        {#each item.tags.slice(0, 4) as tag}
          <TagChip {tag} size={10} onclick={onTagClick ? () => { onTagClick!(tag); } : undefined} />
        {/each}
      </div>
    {/if}
  </div>

  <!-- og_image thumbnail (trailing edge) -->
  {#if item.ogImage && density !== 'dense'}
    <img
      src={item.ogImage}
      alt=""
      loading="lazy"
      style="width:{thumbSize}px;height:{thumbSize}px;object-fit:cover;border-radius:3px;border:1px solid {T.bd0};flex-shrink:0;align-self:flex-start;"
      onerror={(e) => { (e.target as HTMLImageElement).style.display = 'none'; }}
    />
  {/if}
</div>
