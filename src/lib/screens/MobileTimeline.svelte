<script lang="ts">
  import { T } from '$lib/tokens';
  import { Dialog } from 'bits-ui';
  import type { FeedItem } from '$lib/types';
  import { groups, sources, items, markAllRead, markRead, toggleSaved, hideItem } from '$lib/stores/data.svelte';
  import { doSync as storeSync, syncState } from '$lib/stores/sync.svelte';
  import { timelineFilter, setFeedFilter, setGroupFilter, setTagFilter, setReadFilter, setSavedFilter, pageCounts } from '$lib/stores/timeline.svelte';
  import { aiStats } from '$lib/stores/ai.svelte';
  import { openExternal, shareItem } from '$lib/utils';
  import { settings } from '$lib/settings.svelte';
  import GroupTabs from '$lib/components/GroupTabs.svelte';
  import FilterStrip from '$lib/components/FilterStrip.svelte';
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
  let signalActive = $state(false);

  // Group, feed, tag, read, and saved filters are applied server-side in `items`.
  // Signal filtering is client-side for now (TODO: move to server-side).
  const displayItems = $derived.by(() => {
    let list = items as typeof items;
    if (signalActive) list = list.filter(i => i.aiScore >= settings.confidenceThreshold);
    if (sort === 'score') list = [...list].sort((a, b) => b.aiScore - a.aiScore);
    return list;
  });

  // Which tab is highlighted in FilterStrip — derived from server-side filter state.
  const filter = $derived(
    signalActive ? 'signal' :
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
    aiStats.tagCounts.slice(0, 5).map(([tag]) => tag)
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

<div style="display:flex;flex-direction:column;height:100%;background:{T.bg0};color:{T.ink0};">
  <!-- Top bar -->
  <div style="height:44px;display:flex;align-items:center;padding:0 8px;border-bottom:1px solid {T.bd0};background:{T.bg1};flex-shrink:0;gap:6px;">
    <div style="display:flex;align-items:center;gap:6px;flex:1;">
      <span style="font:600 14px/1 {T.mono};color:{T.ink0};letter-spacing:1px;">PULSE<span style="color:{T.cyan};">.</span></span>
    </div>
    <button
      onclick={doSync}
      style="width:34px;height:34px;display:inline-flex;align-items:center;justify-content:center;background:transparent;border:none;cursor:pointer;border-radius:4px;"
    >
      <span class={syncState.syncing ? 'syncing' : ''}>
        <Icon name="sync" size={18} color={syncState.syncing ? T.cyan : T.ink1} />
      </span>
    </button>
    <button
      onclick={() => { showFilter = !showFilter; }}
      style="width:34px;height:34px;display:inline-flex;align-items:center;justify-content:center;background:{showFilter ? 'rgba(78,205,214,0.06)' : 'transparent'};border:none;cursor:pointer;border-radius:4px;"
      title={showFilter ? 'Hide filter bar' : 'Show filter bar'}
    >
      <Icon name="filter" size={18} color={showFilter ? T.cyan : T.ink1} />
    </button>
    <button style="width:34px;height:34px;display:inline-flex;align-items:center;justify-content:center;background:transparent;border:none;cursor:pointer;border-radius:4px;" onclick={() => onTabChange('search')}>
      <Icon name="search" size={18} color={T.ink1} />
    </button>
  </div>

  <!-- Status strip -->
  <div style="display:flex;align-items:center;justify-content:space-between;padding:5px 10px;border-bottom:1px solid {T.bd0};background:{T.bg0};font:10px/1 {T.mono};color:{T.ink2};flex-shrink:0;">
    <div style="display:flex;align-items:center;gap:10px;">
      <span>
        <span style="color:{T.ink3};">sync</span>
        <span style="color:{syncState.syncing ? T.amber : T.green};"> ●</span>
        <span style="color:{T.ink1};"> {syncState.lastSyncAt}</span>
      </span>
      {#if syncState.lastNewCount > 0}
        <span style="color:{T.ink3};">·</span>
        <span><span style="color:{T.ink3};">new</span> <span style="color:{T.cyan};">+{syncState.lastNewCount}</span></span>
      {/if}
    </div>
    <div style="display:flex;align-items:center;gap:8px;">
      <span><span style="color:{T.ink3};">ai</span> <span style="color:{settings.aiTagging ? T.amber : T.ink3};">{settings.aiTagging ? 'on' : 'off'}</span></span>
      <span style="color:{T.ink3};">·</span>
      <span style="color:{T.ink1};">{pageCounts.unread}</span>
      <span style="color:{T.ink3};">unread</span>
    </div>
  </div>

  <!-- Source filter banner (when browsing a specific source) -->
  {#if timelineFilter.feedId}
    <div style="display:flex;align-items:center;gap:8px;padding:6px 12px;background:rgba(78,205,214,0.06);border-bottom:1px solid {T.bd0};font:10px/1 {T.mono};flex-shrink:0;">
      <span style="color:{T.ink3};">filtered by source:</span>
      <span style="color:{T.cyan};">{feedFilterName}</span>
      <span style="flex:1;"></span>
      <button
        onclick={() => setFeedFilter(null)}
        style="background:transparent;border:none;cursor:pointer;display:flex;align-items:center;gap:4px;font:10px/1 {T.mono};color:{T.ink2};"
      >
        <Icon name="x" size={11} color={T.ink2} /> clear
      </button>
    </div>
  {:else}
    <!-- Group tabs -->
    <GroupTabs {groups} active={activeGroup} onSelect={(id) => { setReadFilter(null); setSavedFilter(null); signalActive = false; setGroupFilter(id === 'all' ? null : id); }} />
  {/if}

  <!-- Tag filter banner -->
  {#if timelineFilter.tag}
    <div style="display:flex;align-items:center;gap:8px;padding:5px 12px;background:rgba(78,205,214,0.06);border-bottom:1px solid {T.bd0};font:10px/1 {T.mono};flex-shrink:0;">
      <span style="color:{T.ink3};">tag:</span>
      <span style="color:{T.cyan};">{timelineFilter.tag}</span>
      <span style="flex:1;"></span>
      <button
        onclick={() => setTagFilter(null)}
        style="background:transparent;border:none;cursor:pointer;display:flex;align-items:center;gap:4px;font:10px/1 {T.mono};color:{T.ink2};"
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
      {filter} onFilter={(f) => {
        if (f === 'unread') { setReadFilter(false); setSavedFilter(null); signalActive = false; }
        else if (f === 'saved') { setReadFilter(null); setSavedFilter(true); signalActive = false; }
        else if (f === 'signal') { setReadFilter(null); setSavedFilter(null); signalActive = true; }
        else { setReadFilter(null); setSavedFilter(null); signalActive = false; }
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
      <Dialog.Overlay style="position:fixed;inset:0;background:rgba(0,0,0,0.55);z-index:100;" />
      <Dialog.Content
        preventScroll={false}
        style="position:fixed;bottom:0;left:0;right:0;width:100%;background:{T.bg2};border-top:1px solid {T.bd1};padding:14px 14px 24px;font:12px/1.4 {T.sans};color:{T.ink0};max-height:70vh;overflow-y:auto;user-select:none;-webkit-user-select:none;-webkit-touch-callout:none;z-index:100;"
      >
        {#if actionSheetItem}
          {@const ci = actionSheetItem}
          {@const isHnSelf = ci.url?.includes('news.ycombinator.com/item') ?? false}
        <div style="display:flex;align-items:center;justify-content:space-between;margin-bottom:12px;">
          <span style="font:10px/1 {T.mono};color:{T.ink3};text-transform:uppercase;letter-spacing:0.5px;">actions</span>
          <Dialog.Close style="background:transparent;border:none;color:{T.ink2};cursor:pointer;display:flex;">
            <Icon name="x" size={14} />
          </Dialog.Close>
        </div>

        {#if ci.url && !isHnSelf}
          <button
            onclick={() => { openExternal(ci.url!); actionSheetItem = null; }}
            style="display:flex;align-items:center;gap:10px;width:100%;padding:11px 0;background:transparent;border:none;border-bottom:1px solid {T.bd0};color:{T.ink0};cursor:pointer;text-align:left;font:12px/1 {T.sans};"
          >
            <Icon name="ext" size={13} color={T.ink2} />
            <span>Open in browser</span>
          </button>
        {/if}
        {#if ci.url}
          <button
            onclick={() => { navigator.clipboard.writeText(ci.url!); actionSheetItem = null; }}
            style="display:flex;align-items:center;gap:10px;width:100%;padding:11px 0;background:transparent;border:none;border-bottom:1px solid {T.bd0};color:{T.ink0};cursor:pointer;text-align:left;font:12px/1 {T.sans};"
          >
            <Icon name="link" size={13} color={T.ink2} />
            <span>Copy URL</span>
          </button>
        {/if}
        <button
          onclick={() => { navigator.clipboard.writeText(ci.title); actionSheetItem = null; }}
          style="display:flex;align-items:center;gap:10px;width:100%;padding:11px 0;background:transparent;border:none;border-bottom:1px solid {T.bd1};color:{T.ink0};cursor:pointer;text-align:left;font:12px/1 {T.sans};"
        >
          <Icon name="edit" size={13} color={T.ink2} />
          <span>Copy title</span>
        </button>
        {#if ci.title && (ci.url || ci.externalUrl)}
          <button
            onclick={() => { shareItem(ci.title, ci.url ?? ci.externalUrl); actionSheetItem = null; }}
            style="display:flex;align-items:center;gap:10px;width:100%;padding:11px 0;background:transparent;border:none;border-bottom:1px solid {T.bd1};color:{T.ink0};cursor:pointer;text-align:left;font:12px/1 {T.sans};"
          >
            <Icon name="share" size={13} color={T.ink2} />
            <span>Share</span>
          </button>
        {/if}

        <div style="height:1px;background:{T.bd0};margin:4px 0;"></div>

        <button
          onclick={() => { markRead(ci.id, !ci.read); actionSheetItem = null; }}
          style="display:flex;align-items:center;gap:10px;width:100%;padding:11px 0;background:transparent;border:none;border-bottom:1px solid {T.bd0};color:{ci.read ? T.ink1 : T.cyan};cursor:pointer;text-align:left;font:12px/1 {T.sans};"
        >
          <Icon name="check" size={13} color={ci.read ? T.ink2 : T.cyan} />
          <span>{ci.read ? 'Mark as unread' : 'Mark as read'}</span>
        </button>
        <button
          onclick={() => { toggleSaved(ci.id); actionSheetItem = null; }}
          style="display:flex;align-items:center;gap:10px;width:100%;padding:11px 0;background:transparent;border:none;border-bottom:1px solid {T.bd0};color:{ci.saved ? T.amber : T.ink1};cursor:pointer;text-align:left;font:12px/1 {T.sans};"
        >
          <Icon name="bookmark" size={13} color={ci.saved ? T.amber : T.ink2} />
          <span>{ci.saved ? 'Unsave' : 'Save'}</span>
        </button>
        <button
          onclick={() => { hideItem(ci.id); actionSheetItem = null; }}
          style="display:flex;align-items:center;gap:10px;width:100%;padding:11px 0;background:transparent;border:none;border-bottom:1px solid {T.bd1};color:{T.red};cursor:pointer;text-align:left;font:12px/1 {T.sans};"
        >
          <Icon name="eye-off" size={13} color={T.red} />
          <span>Hide</span>
        </button>

        <div style="height:1px;background:{T.bd0};margin:4px 0;"></div>

        <button
          onclick={() => { onOpen(ci.id, displayItems.map(i => i.id)); actionSheetItem = null; }}
          style="display:flex;align-items:center;gap:10px;width:100%;padding:11px 0;background:transparent;border:none;color:{T.ink1};cursor:pointer;text-align:left;font:12px/1 {T.sans};"
        >
          <Icon name="cpu" size={13} color={T.ink2} />
          <span>Tag info</span>
        </button>
        {/if}
      </Dialog.Content>
    </Dialog.Portal>
  </Dialog.Root>
