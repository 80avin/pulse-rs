<script lang="ts">
  import { T, SOURCE_KIND, sourcePillLabel, sourcePillHue } from '$lib/tokens';
  import type { FeedItem, Source, Density } from '$lib/types';
  import Icon from './Icon.svelte';
  import TagChip from './TagChip.svelte';
  import { longpress } from './longpress.svelte';
  import { ContextMenu } from 'bits-ui';
  import { openExternal, shareItem } from '$lib/utils';
  import { markRead, toggleSaved, hideItem } from '$lib/stores/data.svelte';

  let { item, source, isFocused = false, density = 'normal', onclick, onTagClick, onLongPress }: {
    item: FeedItem;
    source: Source | undefined;
    isFocused?: boolean;
    density?: Density;
    onclick: () => void;
    onTagClick?: (tag: string) => void;
    onLongPress?: () => void;
  } = $props();

  function copyUrl(u: string) { navigator.clipboard.writeText(u).catch(() => {}); }
  function copyTitle(t: string) { navigator.clipboard.writeText(t).catch(() => {}); }
  const isHnSelf = $derived(item.url?.includes('news.ycombinator.com/item') ?? false);

  const dim         = $derived(item.read);
  const isDense     = $derived(density === 'dense');

  // Font scaling with density
  const titleFont   = $derived(density === 'dense' ? '12px/1.25' : density === 'roomy' ? '15px/1.4' : '14px/1.32');
  const snippetFont = $derived(density === 'roomy' ? '13px/1.45' : '12px/1.4');
  const metaFont    = $derived(density === 'dense' ? '10px/1' : density === 'roomy' ? '12px/1' : '11px/1');
  const urlFont     = $derived(density === 'dense' ? '9px/1' : density === 'roomy' ? '11px/1' : '10px/1');
  const pillFont    = $derived(density === 'dense' ? '8px/1' : density === 'roomy' ? '10px/1' : '9px/1');

  // Source pill
  const sk          = $derived(SOURCE_KIND[source?.kind ?? 'rss'] ?? SOURCE_KIND.rss);
  const pillLabel   = $derived(source ? sourcePillLabel(source.name) : '??');
  const pillHue     = $derived(source ? (source.hue ?? sourcePillHue(source.id)) : 200);
  const pillSize    = $derived(isDense ? 20 : 28);
  const pillRadius  = $derived(source?.kind === 'reddit' ? '50%' : '3px');
  const pillBg      = $derived(`oklch(0.45 0.14 ${pillHue})`);

  // Platform icon for meta row
  const platformIcon = $derived(source?.kind === 'reddit' ? 'reddit' : source?.kind === 'hn' ? 'hn' : 'rss');
  const showPlatformIcon = $derived(!!source);

  // URL bar: show for link posts and RSS articles (not self-posts, Ask HN, images, videos, crossposts)
  const isSelfPost  = $derived(
    item.kind === 'text' ||
    item.kind === 'image' ||
    item.kind === 'video' ||
    item.kind === 'crosspost' ||
    (source?.kind === 'hn' && !item.url) ||
    (source?.kind === 'hn' && !!item.url?.includes('news.ycombinator.com/item'))
  );
  const displayUrl  = $derived(!isSelfPost && !isDense ? (item.externalUrl ?? item.url ?? '') : '');
  const urlDisplay  = $derived.by(() => {
    if (!displayUrl) return '';
    try {
      const u = new URL(displayUrl);
      return u.hostname.replace(/^www\./, '') + u.pathname;
    } catch {
      return displayUrl;
    }
  });

  // Comment count format
  const commentLabel = $derived(isDense ? `${item.n}c` : `${item.n} comment${item.n !== 1 ? 's' : ''}`);

  const thumbSize = $derived(isDense ? 36 : 64);
</script>

<ContextMenu.Root>
  <ContextMenu.Trigger
    tabindex={0}
    {onclick}
    onkeydown={(e) => { if (e.key === 'Enter') onclick(); }}
    // use:longpress={{ onLongpress: () => onLongPress?.() }}
    class="relative flex p-0 border-0 border-b border-bd-0 cursor-pointer select-none w-full text-left min-h-14"
    style="background:{isFocused ? 'rgba(78,205,214,0.05)' : 'transparent'};-webkit-touch-callout:none;"
  >
    <!-- Platform-colored left spine -->
    <span class="shrink-0" style="width:3px;background:{sk.spine};"></span>

    <!-- Unread / focus indicator -->
    <span class="absolute" style="left:0;top:0;bottom:0;width:3px;background:{isFocused ? T.cyan : (item.read ? 'transparent' : T.cyanDim)};z-index:1;"></span>

    <!-- Body -->
    <div class="flex-1 min-w-0 flex gap-3" style="padding:var(--item-pad-y,13px) 14px var(--item-pad-y,13px) 12px;" use:longpress={{ onLongpress: () => onLongPress?.() }} >

      <!-- Source pill -->
      <div class="flex items-center justify-center shrink-0 mt-0.5 text-white" style="
        width:{pillSize}px;height:{pillSize}px;
        border-radius:{pillRadius};background:{pillBg};
        font:700 {pillFont} {T.mono};
      ">{pillLabel}</div>

      <!-- Content column -->
      <div class="flex-1 min-w-0">

        <!-- Crosspost banner -->
        {#if item.kind === 'crosspost' && item.crossFrom}
          <div class="flex items-center gap-1 mb-1 text-[9px] leading-none font-mono">
            <Icon name="crosspost" size={10} color={T.violet} />
            <span class="text-violet">crosspost from</span>
            <span class="text-ink-1 font-medium">{item.crossFrom}</span>
          </div>
        {/if}

        <!-- Title -->
        <div class="overflow-hidden text-ellipsis tracking-[-0.1px]" style="
          font:{dim ? '400' : '500'} {titleFont} {T.sans};
          color:{dim ? T.ink2 : T.ink0};
          display:-webkit-box;-webkit-line-clamp:{isDense ? 1 : 2};-webkit-box-orient:vertical;
        ">{item.title}</div>

        <!-- URL bar (between title and snippet) -->
        {#if displayUrl}
          <div class="flex items-center gap-1 overflow-hidden mt-0.75" style="font:{urlFont} {T.mono};color:{sk.accent};">
            <Icon name="link" size={10} color={sk.accent} />
            <span class="truncate">{urlDisplay}</span>
          </div>
        {/if}

        <!-- Snippet -->
        {#if item.snippet && !isDense}
          <div class="truncate text-ink-2" style="margin-top:{displayUrl ? 5 : 4}px;font:{snippetFont} {T.sans};">
            {item.snippet}
          </div>
        {/if}

        <!-- Meta row -->
        <div class="flex items-center gap-1.5 flex-wrap" style="margin-top:{!isDense ? 6 : 4}px;font:{metaFont} {T.mono};">

          {#if showPlatformIcon}
            <Icon name={platformIcon} size={12} color={sk.accent} />
          {/if}

          {#if source}
            <span class="font-semibold text-ink-0 truncate max-w-35">{source.name}</span>
            <span class="text-ink-3">·</span>
          {/if}

          <span class="text-ink-2">{item.age}</span>

          {#if item.score > 0}
            <span class="text-ink-3">·</span>
            <span class="text-amber">▲ {item.score}</span>
          {/if}

          {#if item.n > 0}
            <span class="text-ink-3">·</span>
            <span class="text-ink-2">{commentLabel}</span>
          {/if}

          {#if item.saved}
            <span class="inline-flex items-center gap-px">
              <Icon name="bookmark" size={12} color={T.amber} />
              {#if item.note}<span class="text-[8px] leading-none font-mono text-amber">*</span>{/if}
            </span>
          {/if}

          <span class="flex-1"></span>

          <!-- Muted tags -->
          {#if item.tags.length > 0 && !isDense}
            {#each item.tags.slice(0, 4) as tag}
              <TagChip {tag} size={9} dim onclick={onTagClick ? () => { onTagClick!(tag); } : undefined} />
            {/each}
          {/if}
        </div>
      </div>

      <!-- og_image trailing edge -->
      {#if item.ogImage && !isDense}
        <img
          src={item.ogImage}
          alt=""
          loading="lazy"
          style="width:{thumbSize}px;height:{thumbSize}px;" class="object-cover rounded border border-bd-0 shrink-0 self-start"
          onerror={() => { item.ogImage = null; }}
        />
      {/if}
    </div>
  </ContextMenu.Trigger>

  <ContextMenu.Portal>
    <ContextMenu.Content class="bg-bg-1 border border-bd-1 rounded overflow-hidden w-50 shadow-[0_8px_32px_rgba(0,0,0,0.6)] z-20">
      {#if item.url && !isHnSelf}
        <ContextMenu.Item onclick={() => openExternal(item.url!)} class="flex items-center gap-2.5 w-full bg-transparent border-none border-b border-bd-0 cursor-pointer text-left p-[9px_12px] text-[11px] leading-none font-mono text-ink-0">
          <Icon name="ext" size={11} color={T.ink2} />
          <span>Open in browser</span>
        </ContextMenu.Item>
      {/if}
      {#if item.url}
        <ContextMenu.Item onclick={() => copyUrl(item.url!)} class="flex items-center gap-2.5 w-full bg-transparent border-none border-b border-bd-0 cursor-pointer text-left p-[9px_12px] text-[11px] leading-none font-mono text-ink-0">
          <Icon name="link" size={11} color={T.ink2} />
          <span>Copy URL</span>
        </ContextMenu.Item>
      {/if}
      <ContextMenu.Item onclick={() => copyTitle(item.title)} class="flex items-center gap-2.5 w-full bg-transparent border-none border-b border-bd-1 cursor-pointer text-left p-[9px_12px] text-[11px] leading-none font-mono text-ink-0">
        <Icon name="edit" size={11} color={T.ink2} />
        <span>Copy title</span>
      </ContextMenu.Item>
      {#if item.title && (item.url || item.externalUrl)}
        <ContextMenu.Item onclick={() => shareItem(item.title, item.url ?? item.externalUrl)} class="flex items-center gap-2.5 w-full bg-transparent border-none border-b border-bd-1 cursor-pointer text-left p-[9px_12px] text-[11px] leading-none font-mono text-ink-0">
          <Icon name="share" size={11} color={T.ink2} />
          <span>Share</span>
        </ContextMenu.Item>
      {/if}
      <div class="bg-bd-0 h-px my-1"></div>
      <ContextMenu.Item onclick={() => markRead(item.id, !item.read)} class="flex items-center gap-2.5 w-full bg-transparent border-none border-b border-bd-0 cursor-pointer text-left p-[9px_12px] text-[11px] leading-none font-mono" style="color:{item.read ? T.ink1 : T.cyan};">
        <Icon name="check" size={11} color={item.read ? T.ink2 : T.cyan} />
        <span>{item.read ? 'Mark as unread' : 'Mark as read'}</span>
      </ContextMenu.Item>
      <ContextMenu.Item onclick={() => toggleSaved(item.id)} class="flex items-center gap-2.5 w-full bg-transparent border-none border-b border-bd-1 cursor-pointer text-left p-[9px_12px] text-[11px] leading-none font-mono" style="color:{item.saved ? T.amber : T.ink1};">
        <Icon name="bookmark" size={11} color={item.saved ? T.amber : T.ink2} />
        <span>{item.saved ? 'Unsave' : 'Save'}</span>
      </ContextMenu.Item>
      <ContextMenu.Item onclick={() => hideItem(item.id)} class="flex items-center gap-2.5 w-full bg-transparent border-none border-b border-bd-1 cursor-pointer text-left p-[9px_12px] text-[11px] leading-none font-mono text-red">
        <Icon name="eye-off" size={11} color={T.red} />
        <span>Hide</span>
      </ContextMenu.Item>
    </ContextMenu.Content>
  </ContextMenu.Portal>
</ContextMenu.Root>
