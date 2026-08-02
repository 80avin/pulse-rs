<script lang="ts">
  import { T } from '$lib/tokens';
  import { Dialog } from 'bits-ui';
  import type { FeedItem } from '$lib/types';
  import { groups, sources, items, markAllRead, markRead, toggleSaved, hideItem } from '$lib/stores/data.svelte';
  import { doSync as storeSync, syncState } from '$lib/stores/sync.svelte';
  import { timelineFilter, applyFilter, setFeedFilter, setTagFilter, pageCounts } from '$lib/stores/timeline.svelte';
  import { tagStats } from '$lib/stores/ai.svelte';
  import { openExternal, shareItem } from '$lib/utils';
  import GroupTabs from '$lib/components/GroupTabs.svelte';  import FilterStrip from '$lib/components/FilterStrip.svelte';
  import PulseBottomNav from '$lib/components/PulseBottomNav.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import TimelineList from '$lib/components/TimelineList.svelte';

  let { tab, onTabChange, onOpen }: {
    tab: string;
    onTabChange: (id: string) => void;
    onOpen: (id: string, ids: string[]) => void;
  } = $props();

  let activeGroup = $derived(timelineFilter.groupId ?? 'all');
  let sort = $state('time');
  let showFilter = $state(true);
  let actionSheetItem = $state<FeedItem | null>(null);

  // Group, feed, tag, read, and saved filters are applied server-side in `items`.
  const displayItems = $derived.by(() => {
    let list = items as typeof items;
    if (sort === 'score') list = [...list].sort((a, b) => b.aiScore - a.aiScore);
    return list;
  });

  // Which tab is highlighted in FilterStrip — derived from server-side filter state.
  const filter = $derived(
    timelineFilter.isRead === false ? 'unread' :
    timelineFilter.isSaved === true ? 'saved' :
    'all'
  );

  // Counts from the backend (accurate, not derived from paginated items).
  const counts = $derived({
    all:    pageCounts.total,
    unread: pageCounts.unread,
    saved:  pageCounts.saved,
    signal: pageCounts.signal,
  });

  // Top 5 tags from global AI stats (accurate, not from paginated items).
  const topTags = $derived(
    tagStats.tagCounts.slice(0, 5).map(([tag]) => tag)
  );

  function handleTagClick(tag: string) {
    setTagFilter(timelineFilter.tag === tag ? null : tag);
    showFilter = true;
  }

  async function doSync() {
    await storeSync();
  }

  const feedFilterName = $derived(sources.find(s => s.id === timelineFilter.feedId)?.name ?? timelineFilter.feedId);
</script>

<div class="flex flex-col h-full bg-bg-0 text-ink-0">
  <!-- Top bar -->
  <div class="h-11 flex items-center gap-1.5 shrink-0 bg-bg-1 border-b border-bd-0 px-2">
    <div class="flex items-center gap-1.5 flex-1">
      <span class="font-semibold text-[14px] leading-none font-mono text-ink-0 tracking-[1px]">PULSE<span class="text-cyan">.</span></span>
    </div>
    <button
      onclick={doSync}
      class="inline-flex items-center justify-center bg-transparent border-none cursor-pointer rounded w-8.5 h-8.5"
    >
      <span class={syncState.syncing ? 'syncing' : ''}>
        <Icon name="sync" size={18} color={syncState.syncing ? T.cyan : T.ink1} />
      </span>
    </button>
    <button
      onclick={() => { showFilter = !showFilter; }}
      class="inline-flex items-center justify-center border-none cursor-pointer rounded w-8.5 h-8.5" style="background:{showFilter ? 'rgba(78,205,214,0.06)' : 'transparent'};"
      title={showFilter ? 'Hide filter bar' : 'Show filter bar'}
    >
      <Icon name="filter" size={18} color={showFilter ? T.cyan : T.ink1} />
    </button>
    <button class="inline-flex items-center justify-center bg-transparent border-none cursor-pointer rounded w-8.5 h-8.5" onclick={() => onTabChange('search')}>
      <Icon name="search" size={18} color={T.ink1} />
    </button>
  </div>

  <!-- Status strip -->
  <div class="flex items-center justify-between bg-bg-0 text-ink-2 shrink-0 p-1.25 px-2.5 border-b border-b-bd-0 text-[10px] leading-none font-mono">
    <div class="flex items-center gap-2.5">
      <span>
        <span class="text-ink-3">sync</span>
        <span style="color:{syncState.syncing ? T.amber : T.green};"> ●</span>
        <span class="text-ink-1"> {syncState.lastSyncAt}</span>
      </span>
      {#if syncState.lastNewCount > 0}
        <span class="text-ink-3">·</span>
        <span><span class="text-ink-3">new</span> <span class="text-cyan">+{syncState.lastNewCount}</span></span>
      {/if}
    </div>
    <div class="flex items-center gap-2">
      <span class="text-ink-1">{pageCounts.unread}</span>
      <span class="text-ink-3">unread</span>
    </div>
  </div>

  <!-- Source filter banner (when browsing a specific source) -->
  {#if timelineFilter.feedId}
    <div class="flex items-center gap-2 shrink-0 p-1.5 px-3 bg-[rgba(78,205,214,0.06)] border-b border-b-bd-0 text-[10px] leading-none font-mono">
      <span class="text-ink-3">filtered by source:</span>
      <span class="text-cyan">{feedFilterName}</span>
      <span class="flex-1"></span>
      <button
        onclick={() => setFeedFilter(null)}
        class="bg-transparent border-none cursor-pointer flex items-center gap-1 text-[10px] leading-none font-mono text-ink-2"
      >
        <Icon name="x" size={11} color={T.ink2} /> clear
      </button>
    </div>
  {:else}
    <!-- Group tabs -->
    <GroupTabs {groups} active={activeGroup} onSelect={(id) => { applyFilter({ isRead: null, isSaved: null, groupId: id === 'all' ? null : id }); }} />
  {/if}

  <!-- Tag filter banner -->
  {#if timelineFilter.tag}
    <div class="flex items-center gap-2 shrink-0 p-1.25 px-3 bg-[rgba(78,205,214,0.06)] border-b border-b-bd-0 text-[10px] leading-none font-mono">
      <span class="text-ink-3">tag:</span>
      <span class="text-cyan">{timelineFilter.tag}</span>
      <span class="flex-1"></span>
      <button
        onclick={() => setTagFilter(null)}
        class="bg-transparent border-none cursor-pointer flex items-center gap-1 text-[10px] leading-none font-mono text-ink-2"
      >
        <Icon name="x" size={11} color={T.ink2} /> clear
      </button>
    </div>
  {/if}

  <!-- Timeline list -->
  <TimelineList
    items={displayItems}
    emptyMessage={timelineFilter.feedId || timelineFilter.tag ? 'no matching items' : filter !== 'all' ? `no ${filter} items in this view` : 'no items'}
    onItemClick={(id, allIds) => onOpen(id, allIds)}
    onTagClick={handleTagClick}
    onLongPress={(item) => { window.getSelection()?.removeAllRanges(); actionSheetItem = item; }}
  />

  <!-- Filter strip (toggleable) -->
  {#if showFilter}
    <FilterStrip
      {filter}       onFilter={(f) => {
        if (f === 'unread') { applyFilter({ isRead: false, isSaved: null }); }
        else if (f === 'saved') { applyFilter({ isRead: null, isSaved: true }); }
        else { applyFilter({ isRead: null, isSaved: null }); }
      }}
      {sort} onSort={(s) => { sort = s; }}
      {counts}
      onMarkAllRead={() => markAllRead(displayItems.map(i => i.id))}
      activeTag={timelineFilter.tag}
      onClearTagFilter={() => setTagFilter(null)}
      {topTags}
      onTagFilter={(tag) => handleTagClick(tag)}
    />
  {/if}

  <!-- Bottom nav -->
  <PulseBottomNav active={tab} onChange={onTabChange} />
</div>

<!-- Long-press action sheet -->
  <Dialog.Root open={actionSheetItem !== null} onOpenChange={(open) => { if (!open) actionSheetItem = null; }}>
    <Dialog.Portal>
      <Dialog.Overlay class="fixed inset-0 bg-black/55 z-[100]" />
      <Dialog.Content
        preventScroll={false}
        class="fixed bg-bg-2 text-ink-0 select-none overflow-y-auto bottom-0 left-0 right-0 w-full p-[14px_14px_24px] border-t border-t-bd-1 text-[12px] leading-[1.4] font-sans max-h-[70vh] z-[100]" style="-webkit-touch-callout:none;"
      >
        {#if actionSheetItem}
          {@const ci = actionSheetItem}
          {@const isHnSelf = ci.url?.includes('news.ycombinator.com/item') ?? false}
        <div class="flex items-center justify-between mb-3">
          <span class="uppercase text-[10px] leading-none font-mono text-ink-3 tracking-[0.5px]">actions</span>
          <Dialog.Close class="bg-transparent border-none text-ink-2 cursor-pointer flex">
            <Icon name="x" size={14} />
          </Dialog.Close>
        </div>

        {#if ci.url && !isHnSelf}
          <button
            onclick={() => { openExternal(ci.url!); actionSheetItem = null; }}
            class="flex items-center gap-2.5 w-full bg-transparent border-none cursor-pointer text-left py-[11px] text-[12px] leading-none font-sans text-ink-0 border-b border-b-bd-0"
          >
            <Icon name="ext" size={13} color={T.ink2} />
            <span>Open in browser</span>
          </button>
        {/if}
        {#if ci.url}
          <button
            onclick={() => { navigator.clipboard.writeText(ci.url!); actionSheetItem = null; }}
            class="flex items-center gap-2.5 w-full bg-transparent border-none cursor-pointer text-left py-[11px] text-[12px] leading-none font-sans text-ink-0 border-b border-b-bd-0"
          >
            <Icon name="link" size={13} color={T.ink2} />
            <span>Copy URL</span>
          </button>
        {/if}
        <button
          onclick={() => { navigator.clipboard.writeText(ci.title); actionSheetItem = null; }}
          class="flex items-center gap-2.5 w-full bg-transparent border-none cursor-pointer text-left py-[11px] text-[12px] leading-none font-sans text-ink-0 border-b border-b-bd-1"
        >
          <Icon name="edit" size={13} color={T.ink2} />
          <span>Copy title</span>
        </button>
        {#if ci.title && (ci.url || ci.externalUrl)}
          <button
            onclick={() => { shareItem(ci.title, ci.url ?? ci.externalUrl); actionSheetItem = null; }}
            class="flex items-center gap-2.5 w-full bg-transparent border-none cursor-pointer text-left py-[11px] text-[12px] leading-none font-sans text-ink-0 border-b border-b-bd-1"
          >
            <Icon name="share" size={13} color={T.ink2} />
            <span>Share</span>
          </button>
        {/if}

        <div class="bg-bd-0 h-px my-1"></div>

        <button
          onclick={() => { markRead(ci.id, !ci.read); actionSheetItem = null; }}
          class="flex items-center gap-2.5 w-full bg-transparent border-none cursor-pointer text-left py-[11px] text-[12px] leading-none font-sans border-b border-b-bd-0" style="color:{ci.read ? T.ink1 : T.cyan};"
        >
          <Icon name="check" size={13} color={ci.read ? T.ink2 : T.cyan} />
          <span>{ci.read ? 'Mark as unread' : 'Mark as read'}</span>
        </button>
        <button
          onclick={() => { toggleSaved(ci.id); actionSheetItem = null; }}
          class="flex items-center gap-2.5 w-full bg-transparent border-none cursor-pointer text-left py-[11px] text-[12px] leading-none font-sans border-b border-b-bd-0" style="color:{ci.saved ? T.amber : T.ink1};"
        >
          <Icon name="bookmark" size={13} color={ci.saved ? T.amber : T.ink2} />
          <span>{ci.saved ? 'Unsave' : 'Save'}</span>
        </button>
        <button
          onclick={() => { hideItem(ci.id); actionSheetItem = null; }}
          class="flex items-center gap-2.5 w-full bg-transparent border-none cursor-pointer text-left py-[11px] text-[12px] leading-none font-sans text-red border-b border-b-bd-1"
        >
          <Icon name="eye-off" size={13} color={T.red} />
          <span>Hide</span>
        </button>

        <div class="bg-bd-0 h-px my-1"></div>

        <button
          onclick={() => { onOpen(ci.id, displayItems.map(i => i.id)); actionSheetItem = null; }}
          class="flex items-center gap-2.5 w-full bg-transparent border-none cursor-pointer text-left py-[11px] text-[12px] leading-none font-sans text-ink-1"
        >
          <Icon name="cpu" size={13} color={T.ink2} />
          <span>Tag info</span>
        </button>
        {/if}
      </Dialog.Content>
    </Dialog.Portal>
  </Dialog.Root>
