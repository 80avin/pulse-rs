<script lang="ts">
  import { T, TAG_COLORS } from '$lib/tokens';
  import { groups, sources, markAllRead } from '$lib/stores/data.svelte';
  import { doSync as storeSync, syncState } from '$lib/stores/sync.svelte';
  import { timelineFilter, applyFilter, setFeedFilter, setTagFilter, pageCounts } from '$lib/stores/timeline.svelte';
  import { tagStats } from '$lib/stores/ai.svelte';
  import GroupTabs from './GroupTabs.svelte';
  import FilterStrip from './FilterStrip.svelte';
  import Icon from './Icon.svelte';
  import TimelineList from './TimelineList.svelte';
  import type { FeedItem } from '$lib/types';

  // One timeline pane for both breakpoints. `mode` switches the toolbar/filter
  // layout; the list, filter state, counts, and tag logic are shared.
  let { mode, items, onOpen, onSearch, searchQuery = '', openId = '' }: {
    mode: 'wide' | 'narrow';
    items: FeedItem[];
    onOpen: (id: string, allIds: string[]) => void;
    onSearch?: () => void;
    searchQuery?: string;
    openId?: string;
  } = $props();

  let sort = $state('time');
  let showFilter = $state(true);

  const activeGroup = $derived(timelineFilter.groupId ?? 'all');
  const displayItems = $derived.by(() => {
    if (sort === 'score') return [...items].sort((a, b) => b.aiScore - a.aiScore);
    return items;
  });

  // Which filter tab is highlighted — derived from server-side filter state.
  const filter = $derived(
    timelineFilter.isRead === false ? 'unread' :
    timelineFilter.isSaved === true ? 'saved' : 'all'
  );

  // Counts from the backend (accurate, not derived from paginated items).
  const counts = $derived({ all: pageCounts.total, unread: pageCounts.unread, saved: pageCounts.saved, signal: pageCounts.signal });

  // Top 5 tags from global tag stats.
  const topTags = $derived(tagStats.tagCounts.slice(0, 5).map(([tag]) => tag));

  const feedFilterName = $derived(sources.find(s => s.id === timelineFilter.feedId)?.name ?? timelineFilter.feedId);
  const viewLabel = $derived(
    timelineFilter.feedId ? feedFilterName : (groups.find(g => g.id === activeGroup)?.name ?? 'all')
  );

  function handleTagClick(tag: string) {
    setTagFilter(timelineFilter.tag === tag ? null : tag);
    showFilter = true;
  }
  function handleFilter(f: string) {
    if (f === 'unread') applyFilter({ isRead: false, isSaved: null });
    else if (f === 'saved') applyFilter({ isRead: null, isSaved: true });
    else applyFilter({ isRead: null, isSaved: null });
  }
  async function doSync() { await storeSync(); }
</script>

<div class="flex flex-col h-full bg-bg-0 text-ink-0 min-w-0">
  {#if mode === 'wide'}
    <!-- Wide toolbar -->
    <div class="flex flex-col border-b border-bd-0 bg-bg-1 shrink-0">
      <div class="flex items-center gap-2.5 text-ink-2 px-2.5 py-1.5 text-[10px] leading-none font-mono">
        <span class="text-ink-0 truncate">{viewLabel}</span>
        <span class="text-ink-3">·</span>
        <span><span class="text-cyan">{counts.unread}</span><span class="text-ink-3"> unread</span></span>
        {#if searchQuery}<span class="text-ink-3">·</span><span class="text-amber">"{searchQuery}"</span>{/if}
        <span class="flex-1"></span>
        {#if counts.unread > 0}
          <button onclick={() => markAllRead(displayItems.map(i => i.id))} class="bg-transparent border-none cursor-pointer text-ink-2 text-[10px] leading-none font-mono">mark all read</button>
        {/if}
      </div>
      {#if timelineFilter.tag || topTags.length > 0}
        <div class="flex items-center gap-1.5 overflow-x-auto flex-nowrap px-2 pb-1.5" style="scrollbar-width:none">
          {#if timelineFilter.tag}
            {@const tc = TAG_COLORS[timelineFilter.tag] ?? { fg: T.cyan, bg: 'rgba(78,205,214,0.10)', bd: 'rgba(78,205,214,0.30)' }}
            <button onclick={() => setTagFilter(null)} class="shrink-0 inline-flex items-center whitespace-nowrap cursor-pointer gap-1 px-1.75 py-0.5 rounded tracking-[0.2px] text-[10px] leading-none font-mono" style="background:{tc.bg};border:1px solid {tc.bd};color:{tc.fg}">
              <span class="text-ink-3">tag:</span>{timelineFilter.tag} ×
            </button>
            {#if topTags.length > 0}<span class="shrink-0 text-ink-3 text-[10px] leading-none font-mono">·</span>{/if}
          {/if}
          {#each topTags as tag}
            {#if tag !== timelineFilter.tag}
              {@const tc = TAG_COLORS[tag] ?? { fg: T.ink2, bg: 'transparent', bd: T.bd1 }}
              <button onclick={() => handleTagClick(tag)} class="shrink-0 inline-flex items-center bg-transparent whitespace-nowrap cursor-pointer px-1.75 py-0.5 rounded text-[10px] leading-none font-mono border border-bd-1" style="color:{tc.fg}">{tag}</button>
            {/if}
          {/each}
        </div>
      {/if}
    </div>
  {:else}
    <!-- Narrow top bar -->
    <div class="h-11 flex items-center gap-1.5 shrink-0 bg-bg-1 border-b border-bd-0 px-2">
      <div class="flex items-center gap-1.5 flex-1">
        <span class="font-semibold text-[14px] leading-none font-mono text-ink-0 tracking-[1px]">PULSE<span class="text-cyan">.</span></span>
      </div>
      <button onclick={doSync} aria-label="Sync feeds" class="inline-flex items-center justify-center bg-transparent border-none cursor-pointer rounded min-h-11 min-w-11">
        <span class={syncState.syncing ? 'syncing' : ''}>
          <Icon name="sync" size={18} color={syncState.syncing ? T.cyan : T.ink1} />
        </span>
      </button>
      <button onclick={() => { showFilter = !showFilter; }} aria-label={showFilter ? 'Hide filter bar' : 'Show filter bar'} class="inline-flex items-center justify-center border-none cursor-pointer rounded min-h-11 min-w-11" style="background:{showFilter ? 'rgba(78,205,214,0.06)' : 'transparent'};">
        <Icon name="filter" size={18} color={showFilter ? T.cyan : T.ink1} />
      </button>
      {#if onSearch}
        <button onclick={onSearch} aria-label="Search" class="inline-flex items-center justify-center bg-transparent border-none cursor-pointer rounded min-h-11 min-w-11">
          <Icon name="search" size={18} color={T.ink1} />
        </button>
      {/if}
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
  {/if}

  <!-- Source filter banner / group tabs -->
  {#if timelineFilter.feedId}
    <div class="flex items-center gap-2 shrink-0 p-1.5 px-3 bg-[rgba(78,205,214,0.06)] border-b border-b-bd-0 text-[10px] leading-none font-mono">
      <span class="text-ink-3">filtered by source:</span>
      <span class="text-cyan">{feedFilterName}</span>
      <span class="flex-1"></span>
      <button onclick={() => setFeedFilter(null)} class="bg-transparent border-none cursor-pointer flex items-center gap-1 text-[10px] leading-none font-mono text-ink-2">
        <Icon name="x" size={11} color={T.ink2} /> clear
      </button>
    </div>
  {:else if mode === 'narrow'}
    <GroupTabs {groups} active={activeGroup} onSelect={(id) => { applyFilter({ isRead: null, isSaved: null, groupId: id === 'all' ? null : id }); }} />
  {/if}

  <!-- Tag filter banner -->
  {#if timelineFilter.tag}
    <div class="flex items-center gap-2 shrink-0 p-1.25 px-3 bg-[rgba(78,205,214,0.06)] border-b border-b-bd-0 text-[10px] leading-none font-mono">
      <span class="text-ink-3">tag:</span>
      <span class="text-cyan">{timelineFilter.tag}</span>
      <span class="flex-1"></span>
      <button onclick={() => setTagFilter(null)} class="bg-transparent border-none cursor-pointer flex items-center gap-1 text-[10px] leading-none font-mono text-ink-2">
        <Icon name="x" size={11} color={T.ink2} /> clear
      </button>
    </div>
  {/if}

  <!-- List -->
  <TimelineList
    items={displayItems}
    emptyMessage={timelineFilter.feedId || timelineFilter.tag ? 'no matching items' : filter !== 'all' ? `no ${filter} items in this view` : 'no items'}
    {openId}
    onItemClick={(id, allIds) => onOpen(id, allIds)}
    onTagClick={handleTagClick}
  />

  <!-- Filter strip (narrow, toggleable) -->
  {#if mode === 'narrow' && showFilter}
    <FilterStrip
      {filter} onFilter={handleFilter}
      {sort} onSort={(s) => { sort = s; }}
      {counts}
      onMarkAllRead={() => markAllRead(displayItems.map(i => i.id))}
      activeTag={timelineFilter.tag}
      onClearTagFilter={() => setTagFilter(null)}
      {topTags}
      onTagFilter={handleTagClick}
    />
  {/if}
</div>
