<script lang="ts">
  import { T } from '$lib/tokens';
  import type { FeedItem } from '$lib/types';
  import { sources, storeReady } from '$lib/stores/data.svelte';
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
    onItemContextMenu,
    onTagClick,
    onLongPress,
  }: {
    items: FeedItem[];
    searchQuery?: string;
    emptyMessage?: string;
    openId?: string;
    onItemClick: (id: string, allIds: string[]) => void;
    onItemContextMenu?: (e: MouseEvent, item: FeedItem) => void;
    onTagClick?: (tag: string) => void;
    onLongPress?: (item: FeedItem) => void;
  } = $props();

  const density = $derived(settings.density);
  const allIds = $derived(items.map(i => i.id));

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
      if (!loadingMore.cursor || loadingMore.active) return;
      const e = el as HTMLElement;
      const d = e.scrollHeight - e.scrollTop - e.clientHeight;
      if (d < 300) fetchNextPage();
    }
    el.addEventListener('scroll', onScroll, { passive: true });
    return () => el.removeEventListener('scroll', onScroll);
  });

  $effect(() => {
    if (items.length === 0) return;
    if (!listScrollEl || !loadingMore.cursor || loadingMore.active) return;
    requestAnimationFrame(() => {
      if (!listScrollEl || !loadingMore.cursor || loadingMore.active) return;
      if (listScrollEl.scrollHeight <= listScrollEl.clientHeight + 100) {
        fetchNextPage();
      }
    });
  });
</script>

<div bind:this={listScrollEl} style="flex:1;overflow-y:auto;overflow-x:hidden;position:relative;">
  {#if items.length === 0 && !storeReady.loading}
    <div style="padding:32px;text-align:center;font:11px/1.6 {T.mono};color:{T.ink3};">
      {emptyMessage || (searchQuery ? `no results for "${searchQuery}"` : 'no items in this view')}
    </div>
  {:else}
    <div style="height:{$listVirtualizer.getTotalSize()}px;position:relative;">
      {#each $listVirtualizer.getVirtualItems() as vItem (vItem.key)}
        {@const item = items[vItem.index]}
        {#if item}
          {@const source = sources.find(s => s.id === item.src)}
          <div
            data-index={vItem.index}
            use:measureItem
            style="position:absolute;top:0;left:0;width:100%;transform:translateY({vItem.start}px);"
            oncontextmenu={(e) => onItemContextMenu?.(e, item)}
          >
            <ItemRow
              {item}
              {source}
              isFocused={item.id === openId}
              {density}
              onclick={() => onItemClick(item.id, allIds)}
              {onTagClick}
              onLongPress={onLongPress ? () => onLongPress(item) : undefined}
            />
          </div>
        {/if}
      {/each}
    </div>
    {#if loadingMore.cursor}
      <div style="height:36px;display:flex;align-items:center;justify-content:center;font:10px/1 {T.mono};color:{T.ink3};">
        {loadingMore.active ? 'loading…' : ''}
      </div>
    {/if}
  {/if}
</div>
