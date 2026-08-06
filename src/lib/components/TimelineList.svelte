<script lang="ts">
  import { T } from '$lib/tokens';
  import type { FeedItem } from '$lib/types';
  import type { Snippet } from 'svelte';
  import { sources, storeReady, rememberItem } from '$lib/stores/data.svelte';
  import { loadingMore, fetchNextPage } from '$lib/stores/timeline.svelte';
  import { settings } from '$lib/settings.svelte';
  import ItemRow from './ItemRow.svelte';
  import { createVirtualizer } from '@tanstack/svelte-virtual';
  import { get } from 'svelte/store';

  let {
    items,
    searchQuery = '',
    emptyMessage = '',
    openId = '',
    onItemClick,
    onTagClick,
    hasMore,
    onLoadMore,
    renderAction,
  }: {
    items: FeedItem[];
    searchQuery?: string;
    emptyMessage?: string;
    openId?: string;
    onItemClick: (id: string, allIds: string[]) => void;
    onTagClick?: (tag: string) => void;
    hasMore?: boolean;
    onLoadMore?: () => void;
    renderAction?: Snippet<[FeedItem]>;
  } = $props();

  const density = $derived(settings.density);
  const allIds = $derived(items.map(i => i.id));
  // Cache rendered items so the reader can open them after pagination evicts them
  $effect(() => { for (const it of items) rememberItem(it); });
  // hasMore/onLoadMore override the timeline store's pagination (MobileSaved)
  const more = $derived(hasMore ?? !!loadingMore.cursor);
  const loadMore = $derived(onLoadMore ?? fetchNextPage);
  const busy = $derived(onLoadMore !== undefined ? false : loadingMore.active);

  let listScrollEl: HTMLElement | null = $state(null);
  const listVirtualizer = createVirtualizer({
    count: 0,
    getScrollElement: () => listScrollEl,
    estimateSize: () => (density === 'dense' ? 52 : 82),
    overscan: 10,
  });
  $effect(() => {
    get(listVirtualizer).setOptions({ count: items.length });
  });

  function measureItem(el: HTMLElement) {
    get(listVirtualizer).measureElement(el);
  }

  $effect(() => {
    const el = listScrollEl;
    if (!el) return;
    function onScroll() {
      if (!more || busy) return;
      const e = el as HTMLElement;
      const d = e.scrollHeight - e.scrollTop - e.clientHeight;
      if (d < 300) loadMore();
    }
    el.addEventListener('scroll', onScroll, { passive: true });
    return () => el.removeEventListener('scroll', onScroll);
  });

  $effect(() => {
    if (items.length === 0) return;
    if (!listScrollEl || !more || busy) return;
    requestAnimationFrame(() => {
      if (!listScrollEl || !more || busy) return;
      if (listScrollEl.scrollHeight <= listScrollEl.clientHeight + 100) {
        loadMore();
      }
    });
  });
</script>

<div bind:this={listScrollEl} class="flex-1 overflow-y-auto overflow-x-hidden relative">
  {#if items.length === 0 && !storeReady.loading}
    <div class="p-8 text-center whitespace-pre-line text-[11px] leading-[1.6] font-mono text-ink-3">
      {emptyMessage || (searchQuery ? `no results for "${searchQuery}"` : 'no items in this view')}
    </div>
  {:else}
    <div class="relative" style="height:{$listVirtualizer.getTotalSize()}px;">
      {#each $listVirtualizer.getVirtualItems() as vItem (vItem.key)}
        {@const item = items[vItem.index]}
        {#if item}
          {@const source = sources.find(s => s.id === item.src)}
          <div
            data-index={vItem.index}
            use:measureItem
            class="absolute top-0 left-0 w-full" style="transform:translateY({vItem.start}px);"
          >
            <ItemRow
              {item}
              {source}
              isFocused={item.id === openId}
              {density}
              onclick={() => onItemClick(item.id, allIds)}
              {onTagClick}
            />
            {#if renderAction}
              {@render renderAction(item)}
            {/if}
          </div>
        {/if}
      {/each}
    </div>
    {#if more}
      <div class="h-9 flex items-center justify-center text-[10px] leading-none font-mono text-ink-3">
        {loadingMore.active ? 'loading…' : ''}
      </div>
    {/if}
  {/if}
</div>
