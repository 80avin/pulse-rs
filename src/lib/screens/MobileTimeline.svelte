<script lang="ts">
  import { T } from '$lib/tokens';
  import { groups, sources, items, storeReady, markAllRead, doSync as storeSync, syncState, loadingMore, loadMoreItems } from '$lib/store.svelte';
  import { settings } from '$lib/settings.svelte';
  import GroupTabs from '$lib/components/GroupTabs.svelte';
  import FilterStrip from '$lib/components/FilterStrip.svelte';
  import PulseBottomNav from '$lib/components/PulseBottomNav.svelte';
  import ItemRow from '$lib/components/ItemRow.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import IconBtn from '$lib/components/IconBtn.svelte';

  let { tab, onTabChange, onOpen, filterSource = null, onClearSourceFilter, activeTag = null, onClearTagFilter, onTagFilter }: {
    tab: string;
    onTabChange: (id: string) => void;
    onOpen: (id: string, ids: string[]) => void;
    filterSource?: string | null;
    onClearSourceFilter?: () => void;
    activeTag?: string | null;
    onClearTagFilter?: () => void;
    onTagFilter?: (tag: string) => void;
  } = $props();

  let activeGroup = $state('all');
  let filter = $state('all');
  let sort = $state('time');
  let syncing = $state(false);
  let showFilter = $state(true);


  // Group- and source-filtered items (base for counts and further filtering)
  const groupItems = $derived.by(() => {
    let list = items as typeof items;
    if (filterSource) return list.filter(i => i.src === filterSource);
    if (activeGroup !== 'all') {
      const ids = new Set(sources.filter(s => s.group === activeGroup).map(s => s.id));
      list = list.filter(i => ids.has(i.src));
    }
    return list;
  });

  const filteredItems = $derived.by(() => {
    let list = groupItems as typeof groupItems;
    if (filter === 'unread') list = list.filter(i => !i.read);
    else if (filter === 'saved') list = list.filter(i => i.saved);
    else if (filter === 'signal') list = list.filter(i => i.aiScore >= settings.confidenceThreshold);
    if (activeTag) list = list.filter(i => i.tags.includes(activeTag!));
    if (sort === 'score') list = [...list].sort((a, b) => b.aiScore - a.aiScore);
    return list;
  });

  // Counts from group-filtered items (not all items)
  const counts = $derived({
    all:    groupItems.length,
    unread: groupItems.filter(i => !i.read).length,
    saved:  groupItems.filter(i => i.saved).length,
    signal: groupItems.filter(i => i.aiScore >= settings.confidenceThreshold).length,
  });

  const unread = $derived(groupItems.filter(i => !i.read).length);

  // Top 5 tags by frequency across the current filtered list
  const topTags = $derived.by(() => {
    const tagCounts: Record<string, number> = {};
    for (const item of filteredItems) {
      for (const t of item.tags) tagCounts[t] = (tagCounts[t] ?? 0) + 1;
    }
    return Object.entries(tagCounts).sort((a, b) => b[1] - a[1]).slice(0, 5).map(([t]) => t);
  });

  async function doSync() {
    if (syncing) return;
    syncing = true;
    await storeSync();
    syncing = false;
  }
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
      <span class={syncing ? 'syncing' : ''}>
        <Icon name="sync" size={18} color={syncing ? T.cyan : T.ink1} />
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
      <span style="color:{T.ink1};">{unread}</span>
      <span style="color:{T.ink3};">unread</span>
    </div>
  </div>

  <!-- Source filter banner (when browsing a specific source) -->
  {#if filterSource}
    {@const src = sources.find(s => s.id === filterSource)}
    <div style="display:flex;align-items:center;gap:8px;padding:6px 12px;background:rgba(78,205,214,0.06);border-bottom:1px solid {T.bd0};font:10px/1 {T.mono};flex-shrink:0;">
      <span style="color:{T.ink3};">filtered by source:</span>
      <span style="color:{T.cyan};">{src?.name ?? filterSource}</span>
      <span style="flex:1;"></span>
      <button
        onclick={() => onClearSourceFilter?.()}
        style="background:transparent;border:none;cursor:pointer;display:flex;align-items:center;gap:4px;font:10px/1 {T.mono};color:{T.ink2};"
      >
        <Icon name="x" size={11} color={T.ink2} /> clear
      </button>
    </div>
  {:else}
    <!-- Group tabs -->
    <GroupTabs {groups} active={activeGroup} onSelect={(id) => { activeGroup = id; filter = 'all'; }} />
  {/if}

  <!-- Tag filter banner -->
  {#if activeTag}
    <div style="display:flex;align-items:center;gap:8px;padding:5px 12px;background:rgba(78,205,214,0.06);border-bottom:1px solid {T.bd0};font:10px/1 {T.mono};flex-shrink:0;">
      <span style="color:{T.ink3};">tag:</span>
      <span style="color:{T.cyan};">{activeTag}</span>
      <span style="flex:1;"></span>
      <button
        onclick={() => onClearTagFilter?.()}
        style="background:transparent;border:none;cursor:pointer;display:flex;align-items:center;gap:4px;font:10px/1 {T.mono};color:{T.ink2};"
      >
        <Icon name="x" size={11} color={T.ink2} /> clear
      </button>
    </div>
  {/if}

  <!-- Timeline list -->
  <div style="flex:1;overflow-y:auto;overflow-x:hidden;">
    {#each filteredItems as item}
      {@const source = sources.find(s => s.id === item.src)}
      <ItemRow
        {item}
        {source}
        isFocused={false}
        density={settings.density}
        onclick={() => onOpen(item.id, filteredItems.map(i => i.id))}
        onTagClick={onTagFilter}
      />
    {/each}
    {#if filteredItems.length === 0 && !storeReady.loading}
      <div style="padding:32px;text-align:center;font:11px/1.6 {T.mono};color:{T.ink3};">
        {filter !== 'all' ? `no ${filter} items in this view` : 'no items'}
      </div>
    {:else}
      <div style="padding:14px 10px;font:10px/1 {T.mono};color:{T.ink3};text-align:center;">
        — {filteredItems.length} shown · {items.length} cached —
      </div>
      {#if loadingMore.cursor}
        <div style="padding:10px 14px 18px;display:flex;justify-content:center;">
          <button
            onclick={() => loadMoreItems(activeGroup !== 'all' ? activeGroup : undefined)}
            disabled={loadingMore.active}
            style="
              padding:8px 24px;
              background:{T.bg1};
              border:1px solid {T.bd1};
              border-radius:3px;
              font:11px/1 {T.mono};
              color:{loadingMore.active ? T.ink3 : T.ink1};
              cursor:{loadingMore.active ? 'default' : 'pointer'};
              letter-spacing:0.3px;
            "
          >{loadingMore.active ? 'loading…' : 'load more'}</button>
        </div>
      {/if}
    {/if}
  </div>

  <!-- Filter strip (toggleable) -->
  {#if showFilter}
    <FilterStrip
      {filter} onFilter={(f) => { filter = f; }}
      {sort} onSort={(s) => { sort = s; }}
      {counts}
      onMarkAllRead={() => markAllRead(filteredItems.map(i => i.id))}
      {activeTag}
      onClearTagFilter={() => onClearTagFilter?.()}
      {topTags}
      onTagFilter={(tag) => onTagFilter?.(tag)}
    />
  {/if}

  <!-- Bottom nav -->
  <PulseBottomNav active={tab} onChange={onTabChange} />
</div>
