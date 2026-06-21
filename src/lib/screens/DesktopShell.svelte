<script lang="ts">
  import { T, TAG_COLORS } from '$lib/tokens';
  import { items, sources, groups, storeReady, markRead, toggleSaved, markAllRead, hideItem, dbStats } from '$lib/stores/data.svelte';
  import { doSync as storeSync, syncState } from '$lib/stores/sync.svelte';
  import { taggingProgress, aiStats } from '$lib/stores/ai.svelte';
  import { searchItems } from '$lib/stores/search.svelte';
  import { timelineFilter, setFeedFilter, setGroupFilter, setTagFilter, setReadFilter, setSavedFilter, pageCounts } from '$lib/stores/timeline.svelte';
  import { settings } from '$lib/settings.svelte';
  import { openExternal, shareItem } from '$lib/utils';
  import Icon from '$lib/components/Icon.svelte';
  import KeyCap from '$lib/components/KeyCap.svelte';
  import DragHandle from '$lib/components/DragHandle.svelte';
  import FilterPills from '$lib/components/FilterPills.svelte';
  import BottomTools from '$lib/components/BottomTools.svelte';
  import StatusBar from '$lib/components/StatusBar.svelte';
  import SourceGlyph from '$lib/components/SourceGlyph.svelte';
  import StatusDot from '$lib/components/StatusDot.svelte';
  import AiPanelContent from '$lib/components/AiPanelContent.svelte';
  import SettingsPanelContent from '$lib/components/SettingsPanelContent.svelte';
  import SourceExplorer from '$lib/components/SourceExplorer.svelte';
  import TimelineList from '$lib/components/TimelineList.svelte';
  import GroupTabs from '$lib/components/GroupTabs.svelte';
  import ReaderView from '$lib/components/ReaderView.svelte';
  import Modal from '$lib/components/Modal.svelte';
  import { Accordion } from 'bits-ui';

  let activeGroup  = $derived(timelineFilter.groupId ?? 'all');
  let activeSource = $derived(timelineFilter.feedId ?? null);

  // Resizable pane widths (clamped in the template)
  let leftRailWidth   = $state(232);
  let leftRailPrevW   = 232;
  let timelineWidth   = $state(460);
  let dragging        = $state<string | null>(null);
  let dragStartX      = 0;
  let dragStartW      = 0;
  let leftRailCollapsed = $state(false);

  function onDragStart(edge: string, e: MouseEvent) {
    dragging = edge;
    dragStartX = e.clientX;
    dragStartW = edge === 'left' ? leftRailWidth : timelineWidth;
    e.preventDefault();
  }

  function onMouseMove(e: MouseEvent) {
    if (!dragging) return;
    const delta = e.clientX - dragStartX;
    if (dragging === 'left') {
      if (leftRailCollapsed) return;
      leftRailWidth = Math.max(160, Math.min(360, dragStartW + delta));
    } else {
      timelineWidth = Math.max(280, Math.min(720, dragStartW + delta));
    }
  }

  function toggleLeftRail() {
    if (leftRailCollapsed) {
      leftRailWidth = leftRailPrevW;
      leftRailCollapsed = false;
    } else {
      leftRailPrevW = leftRailWidth;
      leftRailWidth = 32;
      leftRailCollapsed = true;
    }
  }

  function stopDrag() { dragging = null; }

  let openId       = $state('');
  let signalActive  = $state(false);
  const desktopFilter = $derived<'all'|'unread'|'saved'|'signal'>(
    signalActive ? 'signal' :
    timelineFilter.isRead === false ? 'unread' :
    timelineFilter.isSaved === true ? 'saved' : 'all'
  );
  let activeTag    = $derived(timelineFilter.tag ?? null);
  const density    = $derived(settings.density);
  let searchQuery  = $state('');
  let ftsResults   = $state<import('$lib/types').FeedItem[] | null>(null);
  let searchInputEl: HTMLInputElement | null = $state(null);
  let showSettings  = $state(false);
  let showAI        = $state(false);
  let showSources   = $state(false);
  let accValue = $state<string[]>(['sources']);
  let showSourcesAccordion = $derived(accValue.includes('sources'));
  let popoverOpen   = $state(false);
  let showCheatsheet = $state(false);

  const IS_TAURI = typeof window !== 'undefined' && '__TAURI__' in window;

  // FTS backend search — debounced 300ms, only in Tauri context.
  $effect(() => {
    const q = searchQuery.trim();
    if (!IS_TAURI || !q) { ftsResults = null; return; }
    let cancelled = false;
    const timer = setTimeout(async () => {
      if (cancelled) return;
      try { ftsResults = await searchItems(q, 50); } catch { /* ignore */ }
    }, 300);
    return () => { cancelled = true; clearTimeout(timer); };
  });

  const activeGroupLabel = $derived(groups.find(g => g.id === activeGroup)?.name ?? activeGroup);

  const displayItems = $derived.by(() => {
    // When Tauri FTS results are ready, use them directly (full-DB search).
    if (IS_TAURI && ftsResults !== null && searchQuery.trim()) return ftsResults;

    let list = items as typeof items;
    if (!IS_TAURI && searchQuery.trim()) {
      // Client-side fallback for browser dev mode only.
      const q = searchQuery.toLowerCase();
      list = list.filter(i =>
        i.title.toLowerCase().includes(q) || (i.snippet?.toLowerCase().includes(q) ?? false)
      );
    }
    // Signal filtering remains client-side (TODO: move to backend)
    if (signalActive) list = list.filter(i => i.aiScore >= settings.confidenceThreshold);
    return list;
  });

  const openItem    = $derived(items.find(i => i.id === openId));
  const openSource  = $derived(openItem ? sources.find(s => s.id === openItem.src) : undefined);
  const groupSources = $derived(activeGroup === 'all' ? sources : sources.filter(s=>s.group===activeGroup));
  const unreadCount = $derived(pageCounts.unread);
  const taggedCount = $derived(dbStats.tagCount);

  // Top 5 tags from global AI stats.
  const topTags = $derived(
    aiStats.tagCounts.slice(0, 5).map(([tag]) => tag)
  );

  function selectGroup(id: string) { searchQuery = ''; setGroupFilter(id === 'all' ? null : id); }
  function setActiveTag(tag: string) {
    const next = activeTag === tag ? null : tag;
    setTagFilter(next);
    showAI = false;
  }

  function handleFilterChange(filter: string) {
    signalActive = false;
    switch (filter) {
      case 'all':    setReadFilter(null); setSavedFilter(null); break;
      case 'unread': setReadFilter(false); setSavedFilter(null); break;
      case 'saved':  setReadFilter(null); setSavedFilter(true); break;
      case 'signal': setReadFilter(null); setSavedFilter(null); signalActive = true; break;
    }
  }

  function openItemAndRead(id: string) {
    openId = id;
    if (settings.markReadOn === 'open') markRead(id);
  }

  async function doSync() {
    await storeSync();
  }

  // Keyboard shortcuts
  $effect(() => {
    function onKey(e: KeyboardEvent) {
      const target = e.target as HTMLElement;
      const inInput = target instanceof HTMLInputElement || target instanceof HTMLTextAreaElement;

      if (e.key === 'Escape') {
        if (showCheatsheet) { showCheatsheet = false; return; }
        if (searchQuery)  { searchQuery = ''; return; }
        searchInputEl?.blur();
        return;
      }
      if (e.key === '?' && !inInput) {
        showCheatsheet = !showCheatsheet;
        return;
      }
      if (e.key === 'a' && !inInput) {
        showAI = !showAI;
        if (showAI) showSettings = false;
        return;
      }
      if (e.key === 'r' && !inInput) {
        showSources = !showSources;
        return;
      }
      if (e.key === '/' && !inInput) {
        e.preventDefault();
        searchInputEl?.focus();
        return;
      }
      if (inInput) return;
      if (popoverOpen) return;

      switch (e.key) {
        case 'j': case 'ArrowDown': {
          const cur = displayItems.findIndex(i => i.id === openId);
          const next = displayItems[Math.min(cur + 1, displayItems.length - 1)];
          if (next) openItemAndRead(next.id);
          break;
        }
        case 'k': case 'ArrowUp': {
          const cur = displayItems.findIndex(i => i.id === openId);
          if (cur <= 0) break;
          const prev = displayItems[cur - 1];
          if (prev) openItemAndRead(prev.id);
          break;
        }
        case 'm':
          if (openItem) markRead(openItem.id, !openItem.read);
          break;
        case 's':
          if (openItem) toggleSaved(openItem.id);
          break;
        case 'o':
          if (openItem?.domain) openExternal(openItem.url ?? `https://${openItem.domain}`);
          break;
        case 'h':
        case 'x': {
          if (!openItem) break;
          const cur = displayItems.findIndex(i => i.id === openItem!.id);
          const fallback = displayItems[Math.max(0, cur - 1)];
          hideItem(openItem.id);
          openId = (fallback && fallback.id !== openItem!.id) ? fallback.id : '';
          break;
        }
      }
    }
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  });


</script>

{#if storeReady.error}
  <div style="display:flex;align-items:center;justify-content:center;width:100%;height:100%;background:{T.bg0};font:11px/1.6 {T.mono};color:{T.red};">
    failed to load data — check console for details
  </div>
{:else}
<div style="display:flex;flex-direction:column;width:100%;height:100%;background:{T.bg0};color:{T.ink0};overflow:hidden;">

  <!-- Window chrome -->
  <div style="height:32px;display:flex;align-items:center;padding:0 8px 0 12px;background:{T.bg0};border-bottom:1px solid {T.bd0};flex-shrink:0;gap:10px;">
    <div style="display:flex;gap:7px;align-items:center;">
      <span style="width:11px;height:11px;border-radius:11px;background:#e26b6b;display:block;"></span>
      <span style="width:11px;height:11px;border-radius:11px;background:#e6b450;display:block;"></span>
      <span style="width:11px;height:11px;border-radius:11px;background:#6bd896;display:block;"></span>
    </div>
    <div style="width:18px;"></div>
    <span style="font:600 11px/1 {T.mono};color:{T.ink0};letter-spacing:1px;">PULSE<span style="color:{T.cyan};">.</span></span>
    <span style="font:11px/1 {T.mono};color:{T.ink3};">—</span>
    <span style="font:11px/1 {T.mono};color:{T.ink2};">{activeGroupLabel} · {displayItems.length} items</span>
    <span style="flex:1;"></span>
    <button onclick={() => searchInputEl?.focus()} aria-label="Search" style="width:22px;height:22px;display:inline-flex;align-items:center;justify-content:center;background:transparent;border:none;cursor:pointer;border-radius:3px;" title="Search (/)">
      <Icon name="search" size={13} color={T.ink1} />
    </button>
    <button onclick={doSync} aria-label="Sync feeds" style="width:22px;height:22px;display:inline-flex;align-items:center;justify-content:center;background:transparent;border:none;cursor:pointer;border-radius:3px;" title="Sync">
      <span class={syncState.syncing ? 'syncing' : ''}><Icon name="sync" size={13} color={syncState.syncing ? T.cyan : T.ink1} /></span>
    </button>
    <button onclick={() => showCheatsheet = !showCheatsheet} aria-label="Keyboard shortcuts (?)" style="background:transparent;border:none;cursor:pointer;font:10px/1 {T.mono};color:{T.ink3};padding:0 4px;" title="Keyboard shortcuts (?)">?</button>
  </div>

  <!-- Main body -->
  <div
    style="flex:1;display:flex;overflow:hidden;position:relative;"
    onmousemove={onMouseMove}
    onmouseup={stopDrag}
    onmouseleave={stopDrag}
  >

    <!-- Left rail -->
    {#if leftRailCollapsed}
      <div style="width:32px;flex-shrink:0;background:{T.bg1};border-right:1px solid {T.bd0};display:flex;flex-direction:column;align-items:center;padding-top:4px;gap:6px;overflow:hidden;">
        <button onclick={toggleLeftRail} aria-label="Expand sidebar" title="Expand sidebar" style="width:24px;height:24px;display:flex;align-items:center;justify-content:center;background:transparent;border:none;cursor:pointer;border-radius:3px;" onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.background = T.bg2; }} onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.background = 'transparent'; }}>
          <Icon name="chev-r" size={13} color={T.ink2} />
        </button>
        {#each groups as g}
          <button
            onclick={() => selectGroup(g.id)}
            onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.background = T.bg2; }}
            onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.background = 'transparent'; }}
            title={`${g.name} (${g.n})`}
            aria-label={`${g.name} group, ${g.n} items`}
            style="width:24px;height:24px;display:flex;align-items:center;justify-content:center;background:{g.id===activeGroup?'rgba(78,205,214,0.12)':'transparent'};border:none;border-radius:3px;cursor:pointer;position:relative;"
          >
            <span style="font:700 10px/1 {T.mono};color:{g.id===activeGroup ? T.cyan : T.ink2};">{g.name.slice(0, 2).toUpperCase()}</span>
            {#if g.n > 0}<span style="position:absolute;top:0;right:0;width:6px;height:6px;border-radius:50%;background:{T.cyan};"></span>{/if}
          </button>
        {/each}
        <div style="flex:1;"></div>
        <button onclick={() => { showAI = !showAI; }} aria-label="AI Signal" title="AI Signal" style="width:24px;height:24px;display:flex;align-items:center;justify-content:center;background:transparent;border:none;cursor:pointer;border-radius:3px;" onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.background = T.bg2; }} onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.background = 'transparent'; }}>
          <Icon name="cpu" size={13} color={showAI ? T.cyan : T.ink2} />
        </button>
        <button onclick={() => { showSources = !showSources; }} aria-label="Sources" title="Sources" style="width:24px;height:24px;display:flex;align-items:center;justify-content:center;background:transparent;border:none;cursor:pointer;border-radius:3px;" onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.background = T.bg2; }} onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.background = 'transparent'; }}>
          <Icon name="rss" size={13} color={showSources ? T.cyan : T.ink2} />
        </button>
        <button onclick={() => { showSettings = !showSettings; }} aria-label="Settings" title="Settings" style="width:24px;height:24px;display:flex;align-items:center;justify-content:center;background:transparent;border:none;cursor:pointer;border-radius:3px;margin-bottom:6px;" onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.background = T.bg2; }} onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.background = 'transparent'; }}>
          <Icon name="cog" size={13} color={showSettings ? T.cyan : T.ink2} />
        </button>
      </div>
    {:else}
    <div style="width:{leftRailWidth}px;flex-shrink:0;background:{T.bg1};border-right:1px solid {T.bd0};display:flex;flex-direction:column;overflow:hidden;">
      <!-- Collapse toggle -->
      <div style="display:flex;justify-content:flex-end;padding:2px 4px 0;">
        <button onclick={toggleLeftRail} aria-label="Collapse sidebar" title="Collapse sidebar" style="width:22px;height:22px;display:flex;align-items:center;justify-content:center;background:transparent;border:none;cursor:pointer;border-radius:3px;" onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.background = T.bg2; }} onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.background = 'transparent'; }}>
          <Icon name="chev-l" size={12} color={T.ink3} />
        </button>
      </div>
      <!-- Search -->
      <div style="padding:8px 8px 4px;">
        <div style="display:flex;align-items:center;gap:6px;padding:5px 8px;background:{T.bg0};border:1px solid {T.bd1};border-radius:3px;">
          <Icon name="search" size={11} color={T.ink3} />
          <input
            bind:this={searchInputEl}
            bind:value={searchQuery}
            placeholder="search {dbStats.totalItems} items"
            style="flex:1;font:11px/1 {T.mono};"
            aria-label="Search items"
          />
          <KeyCap k="/" dim />
        </div>
      </div>

      <!-- Filter pills -->
      <FilterPills active={desktopFilter} onChange={handleFilterChange} />

      <!-- Groups (vertical) -->
      <div style="padding:2px 0 4px;border-bottom:1px solid {T.bd0};">
        <div style="padding:6px 12px 2px;font:10px/1 {T.mono};color:{T.ink3};letter-spacing:0.6px;text-transform:uppercase;">groups</div>
        <GroupTabs {groups} active={activeGroup} onSelect={(id) => selectGroup(id)} orientation="vertical" />
      </div>

      <!-- Sources accordion -->
      <Accordion.Root type="multiple" bind:value={accValue} style="border-bottom:1px solid {T.bd0};">
        <Accordion.Item value="sources">
          <Accordion.Header>
            <Accordion.Trigger
              onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.background = T.bg2; }}
              onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.background = 'transparent'; }}
              style="display:flex;align-items:center;gap:4px;padding:6px 12px;width:100%;background:transparent;border:none;cursor:pointer;text-align:left;"
              aria-label={`Sources for ${activeGroupLabel}`}
            >
              <span style="font:10px/1 {T.mono};color:{T.ink3};letter-spacing:0.6px;text-transform:uppercase;flex:1;">sources</span>
              <span style="font:10px/1 {T.mono};color:{T.ink2};">{groupSources.length}</span>
              <Icon name={showSourcesAccordion ? 'chev-dn' : 'chev-r'} size={10} color={T.ink3} />
            </Accordion.Trigger>
          </Accordion.Header>
          <Accordion.Content style="padding:0 8px 6px;">
            {#each groupSources as s}
              <button
                onclick={() => { const next = activeSource === s.id ? null : s.id; setFeedFilter(next); }}
                oncontextmenu={(e) => { e.preventDefault(); showSources = !showSources; }}
                onmouseenter={(e) => { if (activeSource!==s.id)(e.currentTarget as HTMLElement).style.background = T.bg2; }}
                onmouseleave={(e) => { if (activeSource!==s.id)(e.currentTarget as HTMLElement).style.background = 'transparent'; }}
                onfocus={(e) => { (e.currentTarget as HTMLElement).style.outline = `1px solid ${T.cyan}`; }}
                onblur={(e) => { (e.currentTarget as HTMLElement).style.outline = 'none'; }}
                style="display:flex;align-items:center;gap:6px;padding:4px 6px;width:100%;background:{activeSource===s.id ? 'rgba(78,205,214,0.06)' : 'transparent'};border:none;border-left:2px solid {activeSource===s.id ? T.cyan : 'transparent'};cursor:pointer;text-align:left;"
                aria-current={activeSource===s.id ? 'true' : undefined}
              >
                <StatusDot status={s.status} size={5} />
                <span style="font:11px/1.2 {T.mono};color:{activeSource===s.id ? T.ink0 : T.ink1};overflow:hidden;text-overflow:ellipsis;white-space:nowrap;flex:1;">{s.name}</span>
                {#if s.unread > 0}<span style="font:10px/1 {T.mono};color:{T.cyan};">{s.unread}</span>{/if}
              </button>
            {/each}
            <button
              onclick={() => { showSources = !showSources; }}
              onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.background = T.bg2; }}
              onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.background = 'transparent'; }}
              style="display:flex;align-items:center;gap:4px;padding:4px 6px;width:100%;background:transparent;border:none;color:{T.ink3};font:10px/1 {T.mono};cursor:pointer;text-align:left;margin-top:2px;"
            >
              <Icon name="plus" size={10} color={T.ink3} />
              <span>Manage Sources</span>
            </button>
          </Accordion.Content>
        </Accordion.Item>
      </Accordion.Root>

      <!-- Spacer -->
      <div style="flex:1;"></div>

      <!-- Bottom utilities -->
      <BottomTools
        {showAI} {showSources} {showSettings}
        syncing={syncState.syncing}
        {syncState} {taggingProgress}
        onToggleAI={() => { showAI = !showAI; if (showAI) showSettings = false; }}
        onToggleSources={() => { showSources = !showSources; }}
        onToggleSettings={() => { showSettings = !showSettings; if (showSettings) showAI = false; }}
      />
    </div>
    {/if}

    <!-- Drag handle: left rail ↔ timeline -->
    <DragHandle edge="left" {onDragStart} {dragging} />

    <!-- Timeline pane -->
    <div style="width:{timelineWidth}px;flex-shrink:0;display:flex;flex-direction:column;border-right:1px solid {T.bd0};overflow:hidden;background:{T.bg0};">
      <div style="display:flex;flex-direction:column;border-bottom:1px solid {T.bd0};background:{T.bg1};flex-shrink:0;">
        <div style="display:flex;align-items:center;gap:10px;padding:6px 10px;font:10px/1 {T.mono};color:{T.ink2};">
          <span style="color:{T.ink0};">{activeSource ? (sources.find(s => s.id === activeSource)?.name ?? activeGroupLabel) : activeGroupLabel}</span>
          <span style="color:{T.ink3};">·</span>
          <span><span style="color:{T.cyan};">{unreadCount}</span><span style="color:{T.ink3};"> unread</span></span>
          {#if searchQuery}<span style="color:{T.ink3};">·</span><span style="color:{T.amber};">"{searchQuery}"</span>{/if}
          <span style="flex:1;"></span>
          {#if unreadCount > 0}
            <button onclick={() => markAllRead(displayItems.map(i => i.id))} style="background:transparent;border:none;cursor:pointer;font:10px/1 {T.mono};color:{T.ink2};">mark all read</button>
          {/if}
        </div>
        {#if activeTag || topTags.length > 0}
          <div style="display:flex;align-items:center;gap:6px;padding:0 8px 6px;overflow-x:auto;scrollbar-width:none;flex-wrap:nowrap;">
            {#if activeTag}
              {@const tc = TAG_COLORS[activeTag] ?? { fg: T.cyan, bg: 'rgba(78,205,214,0.10)', bd: 'rgba(78,205,214,0.30)' }}
              <button onclick={() => setTagFilter(null)} style="flex-shrink:0;display:inline-flex;align-items:center;gap:4px;padding:2px 7px;background:{tc.bg};border:1px solid {tc.bd};border-radius:2px;font:10px/1 {T.mono};color:{tc.fg};cursor:pointer;letter-spacing:0.2px;white-space:nowrap;">
                <span style="color:{T.ink3};">tag:</span>{activeTag} ×
              </button>
              {#if topTags.length > 0}<span style="flex-shrink:0;color:{T.ink3};font:10px/1 {T.mono};">·</span>{/if}
            {/if}
            {#each topTags as tag}
              {#if tag !== activeTag}
                {@const tc = TAG_COLORS[tag] ?? { fg: T.ink2, bg: 'transparent', bd: T.bd1 }}
                <button onclick={() => setActiveTag(tag)} style="flex-shrink:0;display:inline-flex;align-items:center;padding:2px 7px;background:transparent;border:1px solid {T.bd1};border-radius:2px;font:10px/1 {T.mono};color:{tc.fg};cursor:pointer;white-space:nowrap;">{tag}</button>
              {/if}
            {/each}
          </div>
        {/if}
      </div>

      <TimelineList
        items={displayItems}
        {searchQuery}
        {openId}
        onItemClick={(id) => openItemAndRead(id)}
        onTagClick={setActiveTag}
      />
    </div>

    <!-- Drag handle: timeline ↔ reader -->
    <DragHandle edge="timeline" {onDragStart} {dragging} />

    <!-- Detail pane -->
    {#if openItem}
      <div style="flex:1;min-width:0;display:flex;flex-direction:column;background:{T.bg0};overflow:hidden;">
        <div style="padding:6px 14px;border-bottom:1px solid {T.bd0};font:10px/1 {T.mono};color:{T.ink2};background:{T.bg1};display:flex;align-items:center;gap:8px;flex-shrink:0;">
          {#if openSource}
            <SourceGlyph kind={openSource.kind} />
            <span style="color:{T.ink1};">{openSource.name}</span>
            <span style="color:{T.ink3};">·</span>
          {/if}
          <span>{openItem.author}</span>
          <span style="color:{T.ink3};">·</span>
          <span>{openItem.age}</span>
          {#if openItem.score > 0}<span style="color:{T.ink3};">·</span><span style="color:{T.amber};">▲{openItem.score}</span>{/if}
          {#if openItem.n > 0}<span style="color:{T.ink3};">·</span><span style="color:{T.ink2};">{openItem.n}c</span>{/if}
        </div>

        <ReaderView itemId={openItem.id} noteMode="inline" onTagClick={setActiveTag} onPopoverChange={(open) => { popoverOpen = open; }} showMetadata={false} isDesktop={true} />

        <div style="display:flex;border-top:1px solid {T.bd1};background:{T.bg1};flex-shrink:0;">
          <button
            onclick={() => markRead(openItem!.id, !openItem!.read)}
            style="flex:1;display:flex;flex-direction:column;align-items:center;gap:3px;padding:8px 0;background:transparent;border:none;color:{openItem.read ? T.green : T.ink2};cursor:pointer;font:10px/1 {T.mono};letter-spacing:0.3px;"
          >
            <div style="display:flex;align-items:center;gap:4px;">
              <Icon name="check" size={14} color={openItem.read ? T.green : T.ink1} />
              <KeyCap k="m" dim />
            </div>
            <span style="text-transform:uppercase;">{openItem.read ? 'unread' : 'read'}</span>
          </button>
          <button
            onclick={() => toggleSaved(openItem!.id)}
            style="flex:1;display:flex;flex-direction:column;align-items:center;gap:3px;padding:8px 0;background:transparent;border:none;color:{openItem.saved ? T.amber : T.ink2};cursor:pointer;font:10px/1 {T.mono};letter-spacing:0.3px;"
          >
            <div style="display:flex;align-items:center;gap:4px;">
              <Icon name="bookmark" size={14} color={openItem.saved ? T.amber : T.ink1} />
              <KeyCap k="s" dim />
            </div>
            <span style="text-transform:uppercase;">{openItem.saved ? 'saved' : 'save'}</span>
          </button>
          {#if openItem.url}
            <button
              onclick={() => openExternal(openItem!.url!)}
              style="flex:1;display:flex;flex-direction:column;align-items:center;gap:3px;padding:8px 0;background:transparent;border:none;color:{T.ink2};cursor:pointer;font:10px/1 {T.mono};letter-spacing:0.3px;"
            >
              <div style="display:flex;align-items:center;gap:4px;">
                <Icon name="ext" size={14} color={T.ink1} />
                <KeyCap k="o" dim />
              </div>
              <span style="text-transform:uppercase;">open</span>
            </button>
          {/if}
          <button
            onclick={() => shareItem(openItem!.title, openItem!.url ?? openItem!.externalUrl)}
            style="flex:1;display:flex;flex-direction:column;align-items:center;gap:3px;padding:8px 0;background:transparent;border:none;color:{T.ink2};cursor:pointer;font:10px/1 {T.mono};letter-spacing:0.3px;"
          >
            <div style="display:flex;align-items:center;gap:4px;">
              <Icon name="share" size={14} color={T.ink1} />
            </div>
            <span style="text-transform:uppercase;">share</span>
          </button>
          <button
            onclick={() => { const cur = displayItems.findIndex(i => i.id === openItem!.id); const fallback = displayItems[Math.max(0, cur - 1)]; hideItem(openItem!.id); openId = (fallback && fallback.id !== openItem!.id) ? fallback.id : ''; }}
            style="flex:1;display:flex;flex-direction:column;align-items:center;gap:3px;padding:8px 0;background:transparent;border:none;color:{T.red};cursor:pointer;font:10px/1 {T.mono};letter-spacing:0.3px;"
          >
            <div style="display:flex;align-items:center;gap:4px;">
              <Icon name="eye-off" size={14} color={T.red} />
              <KeyCap k="x" dim />
            </div>
            <span style="text-transform:uppercase;">hide</span>
          </button>
        </div>
      </div>
    {:else}
      <div style="flex:1;display:flex;align-items:center;justify-content:center;flex-direction:column;gap:8px;color:{T.ink3};font:11px/1 {T.mono};">
        <span>select an item</span>
        <span style="font:10px/1 {T.mono};color:{T.ink4};">j/k to navigate · / to search</span>
      </div>
    {/if}

    <!-- AI signal modal -->
    <Modal open={showAI} title="AI Signal" onClose={() => { showAI = false; }} width="480px">
      {#if openItem && openItem.tags.length > 0}
        <div style="padding:10px;background:{T.bg1};border:1px solid {T.bd0};border-radius:4px;margin-bottom:14px;">
          <div style="font:10px/1 {T.mono};color:{T.ink3};letter-spacing:0.6px;text-transform:uppercase;margin-bottom:8px;">current item</div>
          <div style="display:flex;align-items:center;gap:8px;margin-bottom:8px;">
            <span style="font:10px/1 {T.mono};color:{T.ink2};">signal</span>
            <div style="flex:1;height:3px;background:{T.bg3};border-radius:2px;overflow:hidden;">
              <div style="height:100%;width:{openItem.aiScore * 100}%;background:{T.cyan};border-radius:2px;"></div>
            </div>
            <span style="font:10px/1 {T.mono};color:{T.amber};font-variant-numeric:tabular-nums;">{openItem.aiScore.toFixed(2)}</span>
          </div>
          <div style="display:flex;flex-wrap:wrap;gap:5px;">
            {#each openItem.tags as tag}
              {@const tc = TAG_COLORS[tag] ?? { fg: T.ink2, bg: 'transparent', bd: T.bd1 }}
              <span style="font:10px/1 {T.mono};color:{tc.fg};background:{tc.bg};border:1px solid {tc.bd};border-radius:3px;padding:3px 7px;">{tag}</span>
            {/each}
          </div>
        </div>
      {/if}
      <AiPanelContent compact onTagFilter={setActiveTag} onItemClick={(id) => { openItemAndRead(id); }} onSourceFilter={(id) => { const next = activeSource === id ? null : id; setFeedFilter(next); showAI = false; }} />
    </Modal>

    <!-- Settings modal -->
    <Modal open={showSettings} title="Settings" onClose={() => { showSettings = false; }} width="420px">
      <SettingsPanelContent showShortcuts />
    </Modal>

    <!-- Sources modal -->
    <Modal open={showSources} title="Sources" onClose={() => { showSources = false; }} width="480px">
      <SourceExplorer onSourceSelect={(id) => { setFeedFilter(id); showSources = false; }} compact={true} isDesktop={true} />
    </Modal>

    <!-- Keyboard shortcut cheatsheet -->
    <Modal open={showCheatsheet} title="Keyboard Shortcuts" onClose={() => showCheatsheet = false} width="440px">
      <div style="display:grid;grid-template-columns:1fr 1fr;gap:0 24px;">
        {#each [
          { k: '/',       desc: 'focus search'      },
          { k: 'j / ↓',  desc: 'next item'         },
          { k: 'k / ↑',  desc: 'prev item'         },
          { k: 'm',       desc: 'toggle read'       },
          { k: 's',       desc: 'save / unsave'     },
          { k: 'o',       desc: 'open in browser'   },
          { k: 'x',       desc: 'hide item'         },
          { k: 'a',       desc: 'AI signal'         },
          { k: 'r',       desc: 'sources'           },
          { k: 'Esc',     desc: 'clear / close'     },
        ] as sc}
          <div style="display:flex;align-items:center;gap:10px;padding:6px 0;border-bottom:1px solid {T.bd0};">
            <KeyCap k={sc.k} />
            <span style="font:11px/1 {T.mono};color:{T.ink2};">{sc.desc}</span>
          </div>
        {/each}
      </div>
      <div style="margin-top:12px;font:10px/1 {T.mono};color:{T.ink3};text-align:center;">press ? or Esc to close</div>
    </Modal>
  </div>

  <!-- Status bar -->
  <StatusBar
    density={settings.density}
    aiTagging={settings.aiTagging}
    {activeGroupLabel} {activeSource}
    activeSourceName={activeSource ? sources.find(s => s.id === activeSource)?.name : undefined}
    itemCount={displayItems.length}
    totalCount={pageCounts.total}
    {unreadCount} {taggedCount}
    {searchQuery} syncing={syncState.syncing} {syncState}
    onToggleCheatsheet={() => showCheatsheet = !showCheatsheet}
  />
</div>
{/if}
