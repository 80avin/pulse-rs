<script lang="ts">
  import { T, SOURCE_KIND, sourcePillLabel, sourcePillHue } from '$lib/tokens';
  import type { FeedItem, Source, Density } from '$lib/types';
  import Icon from './Icon.svelte';
  import TagChip from './TagChip.svelte';
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

  // Long-press detection (mobile context menu)
  let pressTimer: ReturnType<typeof setTimeout> | null = null;
  let longPressed = false;
  let touchMoved = false;
  let touchStartX = 0;
  let touchStartY = 0;

  function startPress(e: TouchEvent) {
    if (!onLongPress) return;
    touchStartX = e.touches[0].clientX;
    touchStartY = e.touches[0].clientY;
    touchMoved = false;
    longPressed = false;
    pressTimer = setTimeout(() => {
      if (touchMoved) return;
      longPressed = true;
      pressTimer = null;
      onLongPress?.();
    }, 450);
  }

  function handleTouchMove(e: TouchEvent) {
    if (!onLongPress) return;
    const dx = Math.abs(e.touches[0].clientX - touchStartX);
    const dy = Math.abs(e.touches[0].clientY - touchStartY);
    if (dx > 8 || dy > 8) {
      touchMoved = true;
      if (pressTimer) { clearTimeout(pressTimer); pressTimer = null; }
    }
  }

  function cancelPress() {
    if (pressTimer) { clearTimeout(pressTimer); pressTimer = null; }
    touchMoved = false;
    longPressed = false;
  }

  function handleTouchEnd(e: TouchEvent) {
    if (!onLongPress) return;
    const wasLong = longPressed;
    cancelPress();
    if (wasLong) {
      e.preventDefault();
    }
  }

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
    ontouchstart={startPress}
    ontouchmove={handleTouchMove}
    ontouchend={handleTouchEnd}
    ontouchcancel={cancelPress}
    style="
      position:relative;display:flex;gap:0;
      padding:0;border:0;
      border-bottom:1px solid {T.bd0};cursor:pointer;
      background:{isFocused ? 'rgba(78,205,214,0.05)' : 'transparent'};
      min-height:56px;
      user-select:none;
      -webkit-user-select:none;
      -webkit-touch-callout:none;
      width:100%;text-align:left;font:inherit;
    "
  >
    <!-- Platform-colored left spine -->
    <span style="width:3px;flex-shrink:0;background:{sk.spine};"></span>

    <!-- Unread / focus indicator -->
    <span style="position:absolute;left:0;top:0;bottom:0;width:3px;background:{isFocused ? T.cyan : (item.read ? 'transparent' : T.cyanDim)};z-index:1;"></span>

    <!-- Body -->
    <div style="flex:1;min-width:0;display:flex;gap:12px;padding:var(--item-pad-y,13px) 14px var(--item-pad-y,13px) 12px;">

      <!-- Source pill -->
      <div style="
        width:{pillSize}px;height:{pillSize}px;flex-shrink:0;
        border-radius:{pillRadius};background:{pillBg};
        display:flex;align-items:center;justify-content:center;
        margin-top:2px;
        font:700 {pillFont} {T.mono};color:#fff;
      ">{pillLabel}</div>

      <!-- Content column -->
      <div style="flex:1;min-width:0;">

        <!-- Crosspost banner -->
        {#if item.kind === 'crosspost' && item.crossFrom}
          <div style="display:flex;align-items:center;gap:4px;margin-bottom:4px;font:9px/1 {T.mono};">
            <Icon name="crosspost" size={10} color={T.violet} />
            <span style="color:{T.violet};">crosspost from</span>
            <span style="color:{T.ink1};font-weight:500;">{item.crossFrom}</span>
          </div>
        {/if}

        <!-- Title -->
        <div style="
          font:{dim ? '400' : '500'} {titleFont} {T.sans};
          color:{dim ? T.ink2 : T.ink0};
          overflow:hidden;text-overflow:ellipsis;
          display:-webkit-box;-webkit-line-clamp:{isDense ? 1 : 2};-webkit-box-orient:vertical;
          letter-spacing:-0.1px;
        ">{item.title}</div>

        <!-- URL bar (between title and snippet) -->
        {#if displayUrl}
          <div style="margin-top:3px;font:{urlFont} {T.mono};color:{sk.accent};display:flex;align-items:center;gap:4px;overflow:hidden;">
            <Icon name="link" size={10} color={sk.accent} />
            <span style="overflow:hidden;text-overflow:ellipsis;white-space:nowrap;">{urlDisplay}</span>
          </div>
        {/if}

        <!-- Snippet -->
        {#if item.snippet && !isDense}
          <div style="margin-top:{displayUrl ? 5 : 4}px;font:{snippetFont} {T.sans};color:{T.ink2};overflow:hidden;text-overflow:ellipsis;white-space:nowrap;">
            {item.snippet}
          </div>
        {/if}

        <!-- Meta row -->
        <div style="margin-top:{!isDense ? 6 : 4}px;display:flex;align-items:center;gap:6px;font:{metaFont} {T.mono};flex-wrap:wrap;">

          {#if showPlatformIcon}
            <Icon name={platformIcon} size={12} color={sk.accent} />
          {/if}

          {#if source}
            <span style="font-weight:600;color:{T.ink0};overflow:hidden;text-overflow:ellipsis;white-space:nowrap;max-width:140px;">{source.name}</span>
            <span style="color:{T.ink3};">·</span>
          {/if}

          <span style="color:{T.ink2};">{item.age}</span>

          {#if item.score > 0}
            <span style="color:{T.ink3};">·</span>
            <span style="color:{T.amber};">▲ {item.score}</span>
          {/if}

          {#if item.n > 0}
            <span style="color:{T.ink3};">·</span>
            <span style="color:{T.ink2};">{commentLabel}</span>
          {/if}

          {#if item.saved}
            <span style="display:inline-flex;align-items:center;gap:1px;">
              <Icon name="bookmark" size={12} color={T.amber} />
              {#if item.note}<span style="font:8px/1 {T.mono};color:{T.amber};">*</span>{/if}
            </span>
          {/if}

          <span style="flex:1;"></span>

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
          style="width:{thumbSize}px;height:{thumbSize}px;object-fit:cover;border-radius:3px;border:1px solid {T.bd0};flex-shrink:0;align-self:flex-start;"
          onerror={() => { item.ogImage = null; }}
        />
      {/if}
    </div>
  </ContextMenu.Trigger>

  <ContextMenu.Portal>
    <ContextMenu.Content style="
      width:200px;
      background:{T.bg1};
      border:1px solid {T.bd1};
      border-radius:4px;
      box-shadow:0 8px 32px rgba(0,0,0,0.6);
      z-index:200;
      overflow:hidden;
    ">
      {#if item.url && !isHnSelf}
        <ContextMenu.Item onclick={() => openExternal(item.url!)} style="display:flex;align-items:center;gap:10px;width:100%;padding:9px 12px;background:transparent;border:none;border-bottom:1px solid {T.bd0};color:{T.ink0};cursor:pointer;text-align:left;font:11px/1 {T.mono};">
          <Icon name="ext" size={11} color={T.ink2} />
          <span>Open in browser</span>
        </ContextMenu.Item>
      {/if}
      {#if item.url}
        <ContextMenu.Item onclick={() => copyUrl(item.url!)} style="display:flex;align-items:center;gap:10px;width:100%;padding:9px 12px;background:transparent;border:none;border-bottom:1px solid {T.bd0};color:{T.ink0};cursor:pointer;text-align:left;font:11px/1 {T.mono};">
          <Icon name="link" size={11} color={T.ink2} />
          <span>Copy URL</span>
        </ContextMenu.Item>
      {/if}
      <ContextMenu.Item onclick={() => copyTitle(item.title)} style="display:flex;align-items:center;gap:10px;width:100%;padding:9px 12px;background:transparent;border:none;border-bottom:1px solid {T.bd1};color:{T.ink0};cursor:pointer;text-align:left;font:11px/1 {T.mono};">
        <Icon name="edit" size={11} color={T.ink2} />
        <span>Copy title</span>
      </ContextMenu.Item>
      {#if item.title && (item.url || item.externalUrl)}
        <ContextMenu.Item onclick={() => shareItem(item.title, item.url ?? item.externalUrl)} style="display:flex;align-items:center;gap:10px;width:100%;padding:9px 12px;background:transparent;border:none;border-bottom:1px solid {T.bd1};color:{T.ink0};cursor:pointer;text-align:left;font:11px/1 {T.mono};">
          <Icon name="share" size={11} color={T.ink2} />
          <span>Share</span>
        </ContextMenu.Item>
      {/if}
      <div style="height:1px;background:{T.bd0};margin:4px 0;"></div>
      <ContextMenu.Item onclick={() => markRead(item.id, !item.read)} style="display:flex;align-items:center;gap:10px;width:100%;padding:9px 12px;background:transparent;border:none;border-bottom:1px solid {T.bd0};color:{item.read ? T.ink1 : T.cyan};cursor:pointer;text-align:left;font:11px/1 {T.mono};">
        <Icon name="check" size={11} color={item.read ? T.ink2 : T.cyan} />
        <span>{item.read ? 'Mark as unread' : 'Mark as read'}</span>
      </ContextMenu.Item>
      <ContextMenu.Item onclick={() => toggleSaved(item.id)} style="display:flex;align-items:center;gap:10px;width:100%;padding:9px 12px;background:transparent;border:none;border-bottom:1px solid {T.bd1};color:{item.saved ? T.amber : T.ink1};cursor:pointer;text-align:left;font:11px/1 {T.mono};">
        <Icon name="bookmark" size={11} color={item.saved ? T.amber : T.ink2} />
        <span>{item.saved ? 'Unsave' : 'Save'}</span>
      </ContextMenu.Item>
      <ContextMenu.Item onclick={() => hideItem(item.id)} style="display:flex;align-items:center;gap:10px;width:100%;padding:9px 12px;background:transparent;border:none;border-bottom:1px solid {T.bd1};color:{T.red};cursor:pointer;text-align:left;font:11px/1 {T.mono};">
        <Icon name="eye-off" size={11} color={T.red} />
        <span>Hide</span>
      </ContextMenu.Item>
    </ContextMenu.Content>
  </ContextMenu.Portal>
</ContextMenu.Root>
