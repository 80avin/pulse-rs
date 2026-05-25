<script lang="ts">
  import { T, TAG_COLORS } from '$lib/tokens';
  import { items, sources, groups, storeReady, markRead, toggleSaved, markAllRead, hideItem, doSync as storeSync, createGroup, syncState, aiStatus, taggingProgress, loadingMore, loadMoreItems, searchItems, hasPrecedingItems } from '$lib/store.svelte';
  import { settings } from '$lib/settings.svelte';
  import { openExternal, sanitizeHtml } from '$lib/utils';
  import Icon from '$lib/components/Icon.svelte';
  import KeyCap from '$lib/components/KeyCap.svelte';
  import TagChip from '$lib/components/TagChip.svelte';
  import ScoreBar from '$lib/components/ScoreBar.svelte';
  import SourceGlyph from '$lib/components/SourceGlyph.svelte';
  import StatusDot from '$lib/components/StatusDot.svelte';
  import ItemRow from '$lib/components/ItemRow.svelte';
  import AiPanelContent from '$lib/components/AiPanelContent.svelte';
  import SettingsPanelContent from '$lib/components/SettingsPanelContent.svelte';
  import { createVirtualizer } from '@tanstack/svelte-virtual';
  import { get } from 'svelte/store';

  let activeGroup  = $state('all');
  let activeSource = $state<string | null>(null);

  // Resizable pane widths (clamped in the template)
  let leftRailWidth  = $state(232);
  let timelineWidth  = $state(460);
  let dragging       = $state<'left' | 'timeline' | null>(null);
  let dragStartX     = 0;
  let dragStartW     = 0;

  function startDrag(which: 'left' | 'timeline', e: MouseEvent) {
    dragging = which;
    dragStartX = e.clientX;
    dragStartW = which === 'left' ? leftRailWidth : timelineWidth;
    e.preventDefault();
  }

  function onMouseMove(e: MouseEvent) {
    if (!dragging) return;
    const delta = e.clientX - dragStartX;
    if (dragging === 'left') leftRailWidth = Math.max(160, Math.min(360, dragStartW + delta));
    else timelineWidth = Math.max(280, Math.min(720, dragStartW + delta));
  }

  function stopDrag() { dragging = null; }

  let openId       = $state('');
  let desktopFilter = $state<'all'|'unread'|'saved'|'signal'>('all');
  let activeTag    = $state<string | null>(null);
  const density    = $derived(settings.density);
  let searchQuery  = $state('');
  let ftsResults   = $state<import('$lib/types').FeedItem[] | null>(null);
  let syncing      = $state(false);
  let searchInputEl: HTMLInputElement | null = $state(null);
  let showSettings  = $state(false);
  let showAI        = $state(false);
  let showAddGroup  = $state(false);
  let newGroupName  = $state('');
  let addGroupInputEl: HTMLInputElement | null = $state(null);
  let showCheatsheet = $state(false);

  // Context menu state
  let contextMenu = $state<{ x: number; y: number; item: import('$lib/types').FeedItem } | null>(null);

  // Virtual list — renders only visible rows, saving ~80% of DOM nodes at scale.
  let listScrollEl: HTMLElement | null = $state(null);
  const listVirtualizer = createVirtualizer({
    count: 0,
    getScrollElement: () => listScrollEl,
    estimateSize: () => (settings.density === 'dense' ? 52 : 82),
    overscan: 10,
  });
  $effect(() => {
    get(listVirtualizer).setOptions({ count: filteredItems.length });
  });

  // Action: called when each virtual item wrapper mounts; reports actual height to the
  // virtualizer so it corrects positions for variable-height rows (tags, snippet, etc.).
  function measureItem(el: HTMLElement) {
    get(listVirtualizer).measureElement(el);
  }

  const IS_TAURI = typeof window !== 'undefined' && '__TAURI__' in window;
  async function tauriInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<T>(cmd, args);
  }

  $effect(() => { activeGroup; activeSource = null; });

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
  });;

  const activeGroupLabel = $derived(groups.find(g => g.id === activeGroup)?.name ?? activeGroup);

  const railSources = $derived.by(() =>
    activeGroup === 'all' ? sources : sources.filter(s => s.group === activeGroup)
  );

  const filteredItems = $derived.by(() => {
    // When Tauri FTS results are ready, use them directly (full-DB search).
    if (IS_TAURI && ftsResults !== null && searchQuery.trim()) return ftsResults;

    let list = items as typeof items;
    if (activeGroup !== 'all') {
      const ids = new Set(sources.filter(s => s.group === activeGroup).map(s => s.id));
      list = list.filter(i => ids.has(i.src));
    }
    if (activeSource) list = list.filter(i => i.src === activeSource);
    if (!IS_TAURI && searchQuery.trim()) {
      // Client-side fallback for browser dev mode only.
      const q = searchQuery.toLowerCase();
      list = list.filter(i =>
        i.title.toLowerCase().includes(q) || (i.snippet?.toLowerCase().includes(q) ?? false)
      );
    }
    if (desktopFilter === 'unread') list = list.filter(i => !i.read);
    else if (desktopFilter === 'saved') list = list.filter(i => i.saved);
    else if (desktopFilter === 'signal') list = list.filter(i => i.aiScore >= settings.confidenceThreshold);
    if (activeTag) list = list.filter(i => i.tags.includes(activeTag!));
    return list;
  });

  const openItem    = $derived(items.find(i => i.id === openId));
  const openSource  = $derived(openItem ? sources.find(s => s.id === openItem.src) : undefined);
  const unreadCount = $derived(filteredItems.filter(i => !i.read).length);
  const taggedCount = $derived(filteredItems.filter(i => i.tags.length > 0).length);

  // Top 5 tags by frequency across the current filtered list
  const topTags = $derived.by(() => {
    const tagCounts: Record<string, number> = {};
    for (const item of filteredItems) {
      for (const t of item.tags) tagCounts[t] = (tagCounts[t] ?? 0) + 1;
    }
    return Object.entries(tagCounts).sort((a, b) => b[1] - a[1]).slice(0, 5).map(([t]) => t);
  });

  function selectGroup(id: string) { activeGroup = id; searchQuery = ''; activeTag = null; }
  function setActiveTag(tag: string) { activeTag = activeTag === tag ? null : tag; showAI = false; }

  function openItemAndRead(id: string) {
    openId = id;
    if (settings.markReadOn === 'open') markRead(id);
  }

  async function doSync() {
    if (syncing) return;
    syncing = true;
    await storeSync();
    syncing = false;
  }

  function commitAddGroup() {
    const name = newGroupName.trim();
    if (!name) return;
    createGroup(name);
    newGroupName = '';
    showAddGroup = false;
  }

  $effect(() => {
    if (showAddGroup) {
      // focus input after it mounts
      setTimeout(() => addGroupInputEl?.focus(), 10);
    }
  });

  // Keyboard shortcuts
  $effect(() => {
    function onKey(e: KeyboardEvent) {
      const target = e.target as HTMLElement;
      const inInput = target instanceof HTMLInputElement || target instanceof HTMLTextAreaElement;

      if (e.key === 'Escape') {
        if (contextMenu)  { contextMenu = null; return; }
        if (showCheatsheet) { showCheatsheet = false; return; }
        if (showAI)       { showAI = false; return; }
        if (showSettings) { showSettings = false; return; }
        if (showAddGroup) { showAddGroup = false; return; }
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
      if (e.key === '/' && !inInput) {
        e.preventDefault();
        searchInputEl?.focus();
        return;
      }
      if (inInput) return;

      switch (e.key) {
        case 'j': case 'ArrowDown': {
          const cur = filteredItems.findIndex(i => i.id === openId);
          const next = filteredItems[Math.min(cur + 1, filteredItems.length - 1)];
          if (next) openItemAndRead(next.id);
          break;
        }
        case 'k': case 'ArrowUp': {
          const cur = filteredItems.findIndex(i => i.id === openId);
          if (cur <= 0) break;
          const prev = filteredItems[cur - 1];
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
          const cur = filteredItems.findIndex(i => i.id === openItem!.id);
          const fallback = filteredItems[Math.max(0, cur - 1)];
          hideItem(openItem.id);
          openId = (fallback && fallback.id !== openItem!.id) ? fallback.id : '';
          break;
        }
      }
    }
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  });

  // Dismiss context menu on outside click
  $effect(() => {
    if (!contextMenu) return;
    function dismiss(e: MouseEvent) {
      const target = e.target as HTMLElement;
      if (!target.closest('[data-ctx-menu]')) contextMenu = null;
    }
    window.addEventListener('mousedown', dismiss);
    return () => window.removeEventListener('mousedown', dismiss);
  });

  function ctxCopyUrl(url: string) {
    navigator.clipboard.writeText(url).catch(() => {});
    contextMenu = null;
  }
  function ctxCopyTitle(title: string) {
    navigator.clipboard.writeText(title).catch(() => {});
    contextMenu = null;
  }
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
    <span style="font:11px/1 {T.mono};color:{T.ink2};">{activeGroupLabel} · {filteredItems.length} items</span>
    <span style="flex:1;"></span>
    <button onclick={() => searchInputEl?.focus()} style="width:22px;height:22px;display:inline-flex;align-items:center;justify-content:center;background:transparent;border:none;cursor:pointer;border-radius:3px;" title="Search (/)">
      <Icon name="search" size={13} color={T.ink1} />
    </button>
    <button onclick={doSync} style="width:22px;height:22px;display:inline-flex;align-items:center;justify-content:center;background:transparent;border:none;cursor:pointer;border-radius:3px;" title="Sync">
      <span class={syncing ? 'syncing' : ''}><Icon name="sync" size={13} color={syncing ? T.cyan : T.ink1} /></span>
    </button>
    <button
      onclick={() => { showAI = !showAI; if (showAI) showSettings = false; }}
      style="width:22px;height:22px;display:inline-flex;align-items:center;justify-content:center;background:{showAI ? 'rgba(78,205,214,0.08)' : 'transparent'};border:none;cursor:pointer;border-radius:3px;position:relative;"
      title={taggingProgress.active ? `tagging ${taggingProgress.tagged}/${taggingProgress.total}…` : 'AI Signal (a)'}
    >
      <Icon name="cpu" size={13} color={taggingProgress.active ? T.amber : (showAI ? T.cyan : T.ink1)} />
      {#if taggingProgress.active}
        <span style="position:absolute;top:1px;right:1px;width:5px;height:5px;border-radius:50%;background:{T.amber};"></span>
      {/if}
    </button>
    <button
      onclick={() => { showSettings = !showSettings; if (showSettings) showAI = false; }}
      style="width:22px;height:22px;display:inline-flex;align-items:center;justify-content:center;background:{showSettings ? 'rgba(78,205,214,0.08)' : 'transparent'};border:none;cursor:pointer;border-radius:3px;"
      title="Settings"
    >
      <Icon name="cog" size={13} color={showSettings ? T.cyan : T.ink1} />
    </button>
  </div>

  <!-- Main body -->
  <!-- svelte-ignore event_directive_deprecated -->
  <div
    style="flex:1;display:flex;overflow:hidden;position:relative;cursor:{dragging ? 'col-resize' : 'default'};"
    onmousemove={onMouseMove}
    onmouseup={stopDrag}
    onmouseleave={stopDrag}
  >

    <!-- Left rail -->
    <div style="width:{leftRailWidth}px;flex-shrink:0;background:{T.bg1};border-right:1px solid {T.bd0};display:flex;flex-direction:column;overflow:hidden;">
      <div style="padding:8px;border-bottom:1px solid {T.bd0};">
        <div style="display:flex;align-items:center;gap:6px;padding:5px 8px;background:{T.bg0};border:1px solid {T.bd1};border-radius:3px;">
          <Icon name="search" size={11} color={T.ink3} />
          <input
            bind:this={searchInputEl}
            bind:value={searchQuery}
            placeholder="search {items.length} items"
            style="flex:1;font:11px/1 {T.mono};"
          />
          <KeyCap k="/" dim />
        </div>
      </div>

      <div style="padding:6px 8px 4px;font:9px/1 {T.mono};color:{T.ink3};letter-spacing:0.6px;text-transform:uppercase;display:flex;justify-content:space-between;align-items:center;">
        <span>groups</span>
        <button
          onclick={() => { showAddGroup = !showAddGroup; newGroupName = ''; }}
          style="width:20px;height:20px;display:inline-flex;align-items:center;justify-content:center;background:{showAddGroup ? 'rgba(78,205,214,0.08)' : 'transparent'};border:1px solid {showAddGroup ? T.cyan : T.bd1};border-radius:3px;cursor:pointer;"
          title="Add group"
        >
          <Icon name="plus" size={10} color={showAddGroup ? T.cyan : T.ink2} />
        </button>
      </div>
      {#each groups as g}
        <button
          onclick={() => selectGroup(g.id)}
          style="display:flex;align-items:center;gap:8px;padding:5px 12px;background:{g.id === activeGroup ? 'rgba(78,205,214,0.06)' : 'transparent'};border:none;border-left:2px solid {g.id === activeGroup ? T.cyan : 'transparent'};color:{g.id === activeGroup ? T.ink0 : T.ink1};font:{g.id === activeGroup ? '600' : '400'} 12px/1.2 {T.mono};cursor:pointer;text-align:left;width:100%;"
        >
          <span style="flex:1;">{g.name}</span>
          <span style="font:10px/1 {T.mono};color:{g.id === activeGroup ? T.cyan : T.ink3};font-variant-numeric:tabular-nums;">{g.n}</span>
        </button>
      {/each}
      {#if showAddGroup}
        <div style="padding:6px 8px;border-top:1px solid {T.bd0};background:{T.bg0};">
          <div style="display:flex;gap:4px;">
            <input
              bind:this={addGroupInputEl}
              bind:value={newGroupName}
              placeholder="group name"
              onkeydown={(e) => { if (e.key === 'Enter') commitAddGroup(); if (e.key === 'Escape') { showAddGroup = false; } }}
              style="flex:1;padding:5px 8px;font:11px/1 {T.mono};background:{T.bg1};border:1px solid {T.bd1};border-radius:3px;color:{T.ink0};"
            />
            <button
              onclick={commitAddGroup}
              style="padding:0 10px;background:{T.cyan};color:{T.bg0};border:none;border-radius:3px;font:600 10px/1 {T.mono};cursor:pointer;white-space:nowrap;"
            >add</button>
          </div>
        </div>
      {/if}

      <div style="padding:12px 8px 4px;font:9px/1 {T.mono};color:{T.ink3};letter-spacing:0.6px;text-transform:uppercase;display:flex;justify-content:space-between;align-items:center;border-top:1px solid {T.bd0};margin-top:6px;">
        <span>sources</span>
        <span style="color:{T.ink2};font-variant-numeric:tabular-nums;">{railSources.length}</span>
      </div>
      <div style="flex:1;overflow-y:auto;">
        {#each railSources as s}
          <button
            onclick={() => { activeSource = activeSource === s.id ? null : s.id; }}
            style="display:grid;grid-template-columns:auto 1fr auto;gap:8px;align-items:center;padding:4px 12px;width:100%;background:{activeSource === s.id ? 'rgba(78,205,214,0.06)' : 'transparent'};border:none;border-left:2px solid {activeSource === s.id ? T.cyan : 'transparent'};font:11px/1.2 {T.mono};cursor:pointer;text-align:left;"
            title="{s.name}{s.status === 'error' ? ' — sync error' : s.status === 'stale' ? ' — stale' : ''}{s.failureStreak > 0 ? ` (${s.failureStreak} consecutive failures)` : ''}"
          >
            <StatusDot status={s.status} size={5} />
            <span style="display:flex;align-items:center;gap:4px;overflow:hidden;">
              <span style="color:{s.status === 'error' ? T.red : s.status === 'stale' ? T.amber : (activeSource === s.id ? T.ink0 : T.ink1)};overflow:hidden;text-overflow:ellipsis;white-space:nowrap;flex:1;">{s.name}</span>
              {#if s.failureStreak >= 2}
                <span style="font:9px/1 {T.mono};color:{T.red};flex-shrink:0;">({s.failureStreak}×)</span>
              {/if}
            </span>
            <span style="color:{s.unread > 0 ? T.cyan : T.ink3};font-variant-numeric:tabular-nums;">{s.unread}</span>
          </button>
        {/each}
      </div>

      <div style="padding:8px;border-top:1px solid {T.bd0};display:flex;gap:4px;background:{T.bg1};">
        {#each (['dense', 'normal', 'roomy'] as const) as d}
          <button
            onclick={() => { settings.density = d; }}
            style="flex:1;background:{density === d ? T.bg3 : 'transparent'};color:{density === d ? T.cyan : T.ink2};border:1px solid {density === d ? T.bd2 : T.bd1};border-radius:2px;padding:4px 6px;font:9px/1 {T.mono};letter-spacing:0.4px;cursor:pointer;text-transform:uppercase;"
          >{d}</button>
        {/each}
      </div>
    </div>

    <!-- Timeline pane -->
    <div style="width:460px;flex-shrink:0;display:flex;flex-direction:column;border-right:1px solid {T.bd0};overflow:hidden;background:{T.bg0};">
      <div style="display:flex;flex-direction:column;border-bottom:1px solid {T.bd0};background:{T.bg1};flex-shrink:0;">
        <div style="display:flex;align-items:center;gap:10px;padding:6px 10px;font:10px/1 {T.mono};color:{T.ink2};">
          <span style="color:{T.ink0};">{activeSource ? (sources.find(s => s.id === activeSource)?.name ?? activeGroupLabel) : activeGroupLabel}</span>
          <span style="color:{T.ink3};">·</span>
          <span><span style="color:{T.cyan};">{unreadCount}</span><span style="color:{T.ink3};"> unread</span></span>
          {#if searchQuery}<span style="color:{T.ink3};">·</span><span style="color:{T.amber};">"{searchQuery}"</span>{/if}
          <span style="flex:1;"></span>
          {#if unreadCount > 0}
            <button
              onclick={() => markAllRead(filteredItems.map(i => i.id))}
              style="background:transparent;border:none;cursor:pointer;font:9px/1 {T.mono};color:{T.ink2};letter-spacing:0.3px;"
              title="Mark all read in current view"
            >mark all read</button>
          {/if}
        </div>
        <!-- Filter strip -->
        <div style="display:flex;gap:2px;padding:0 6px 6px;">
          {#each ([['all','all'],['unread','unread'],['saved','saved'],['signal','signal']] as const) as [id, label]}
            <button
              onclick={() => { desktopFilter = id; }}
              style="
                padding:3px 10px;border-radius:2px;border:none;cursor:pointer;
                font:9px/1 {T.mono};letter-spacing:0.4px;text-transform:uppercase;
                background:{desktopFilter === id ? T.bg3 : 'transparent'};
                color:{desktopFilter === id ? T.cyan : T.ink3};
              "
            >{label}</button>
          {/each}
        </div>

        <!-- Tag filter row -->
        {#if activeTag || topTags.length > 0}
          <div style="display:flex;align-items:center;gap:6px;padding:0 8px 7px;overflow-x:auto;scrollbar-width:none;flex-wrap:nowrap;">
            {#if activeTag}
              {@const tc = TAG_COLORS[activeTag] ?? { fg: T.cyan, bg: 'rgba(78,205,214,0.10)', bd: 'rgba(78,205,214,0.30)' }}
              <button
                onclick={() => { activeTag = null; }}
                style="flex-shrink:0;display:inline-flex;align-items:center;gap:4px;padding:2px 7px;background:{tc.bg};border:1px solid {tc.bd};border-radius:2px;font:9px/1 {T.mono};color:{tc.fg};cursor:pointer;letter-spacing:0.2px;white-space:nowrap;"
              >
                <span style="color:{T.ink3};">tag:</span>{activeTag} ×
              </button>
              {#if topTags.length > 0}
                <span style="flex-shrink:0;color:{T.ink3};font:9px/1 {T.mono};">·</span>
                {#each topTags as tag}
                  {@const tc2 = TAG_COLORS[tag] ?? { fg: T.ink2, bg: 'transparent', bd: T.bd1 }}
                  <button
                    onclick={() => setActiveTag(tag)}
                    style="flex-shrink:0;display:inline-flex;align-items:center;padding:2px 7px;background:transparent;border:1px solid {T.bd1};border-radius:2px;font:9px/1 {T.mono};color:{tag === activeTag ? tc2.fg : T.ink3};cursor:pointer;white-space:nowrap;opacity:{tag === activeTag ? 1 : 0.6};"
                  >{tag}</button>
                {/each}
              {/if}
            {:else}
              <span style="flex-shrink:0;font:9px/1 {T.mono};color:{T.ink3};letter-spacing:0.3px;">top:</span>
              {#each topTags as tag}
                {@const tc = TAG_COLORS[tag] ?? { fg: T.ink2, bg: 'transparent', bd: T.bd1 }}
                <button
                  onclick={() => setActiveTag(tag)}
                  style="flex-shrink:0;display:inline-flex;align-items:center;padding:2px 7px;background:{tc.bg};border:1px solid {tc.bd};border-radius:2px;font:9px/1 {T.mono};color:{tc.fg};cursor:pointer;white-space:nowrap;"
                >{tag}</button>
              {/each}
            {/if}
          </div>
        {/if}
      </div>

      <div bind:this={listScrollEl} style="flex:1;overflow-y:auto;position:relative;">
        {#if filteredItems.length === 0 && !storeReady.loading}
          <div style="padding:32px;text-align:center;font:11px/1.6 {T.mono};color:{T.ink3};">
            {searchQuery ? `no results for "${searchQuery}"` : 'no items in this view'}
          </div>
        {:else}
          <div style="height:{$listVirtualizer.getTotalSize()}px;position:relative;">
            {#each $listVirtualizer.getVirtualItems() as vItem (vItem.key)}
              {@const item = filteredItems[vItem.index]}
              {#if item}
                {@const source = sources.find(s => s.id === item.src)}
                <div
                  data-index={vItem.index}
                  use:measureItem
                  style="position:absolute;top:0;left:0;width:100%;transform:translateY({vItem.start}px);"
                  oncontextmenu={(e) => { e.preventDefault(); contextMenu = { x: e.clientX, y: e.clientY, item }; }}
                >
                  <ItemRow
                    {item}
                    {source}
                    isFocused={item.id === openId}
                    {density}
                    onclick={() => openItemAndRead(item.id)}
                    onTagClick={setActiveTag}
                  />
                </div>
              {/if}
            {/each}
          </div>
          {#if hasPrecedingItems.value}
            <div style="padding:6px 12px;font:9px/1 {T.mono};color:{T.ink3};text-align:center;border-top:1px solid {T.bd0};">
              older items evicted · use search to find them
            </div>
          {/if}
          {#if loadingMore.cursor}
            <div style="padding:10px 12px 14px;display:flex;justify-content:center;">
              <button
                onclick={() => loadMoreItems(activeGroup !== 'all' ? activeGroup : undefined)}
                disabled={loadingMore.active}
                style="
                  padding:6px 20px;
                  background:{T.bg1};
                  border:1px solid {T.bd1};
                  border-radius:3px;
                  font:10px/1 {T.mono};
                  color:{loadingMore.active ? T.ink3 : T.ink1};
                  cursor:{loadingMore.active ? 'default' : 'pointer'};
                  letter-spacing:0.3px;
                "
              >{loadingMore.active ? 'loading…' : 'load more'}</button>
            </div>
          {/if}
        {/if}
      </div>
    </div>

    <!-- Detail pane -->
    {#if openItem}
      <div style="flex:1;min-width:0;display:flex;flex-direction:column;background:{T.bg0};overflow:hidden;">
        <div style="padding:6px 14px;border-bottom:1px solid {T.bd0};font:10px/1 {T.mono};color:{T.ink2};background:{T.bg1};display:flex;align-items:center;gap:8px;flex-shrink:0;flex-wrap:wrap;">
          {#if openSource}
            <SourceGlyph kind={openSource.kind} />
            <span style="color:{T.ink1};">{openSource.name}</span>
            <span style="color:{T.ink3};">·</span>
          {/if}
          <span>{openItem.author}</span>
          <span style="color:{T.ink3};">·</span>
          <span>{openItem.age}</span>
          <span style="flex:1;"></span>
          <button onclick={() => markRead(openItem!.id, !openItem!.read)} style="background:transparent;border:none;cursor:pointer;display:flex;align-items:center;gap:4px;font:10px/1 {T.mono};color:{openItem.read ? T.green : T.ink2};">
            <KeyCap k="m" dim /><span>{openItem.read ? 'read' : 'unread'}</span>
          </button>
          <button onclick={() => toggleSaved(openItem!.id)} style="background:transparent;border:none;cursor:pointer;display:flex;align-items:center;gap:4px;font:10px/1 {T.mono};color:{openItem.saved ? T.amber : T.ink2};">
            <KeyCap k="s" dim /><span>{openItem.saved ? 'saved' : 'save'}</span>
          </button>
          {#if openItem.url}
            <button
              onclick={() => openExternal(openItem!.url!)}
              style="background:transparent;border:none;cursor:pointer;display:flex;align-items:center;gap:4px;font:10px/1 {T.mono};color:{T.ink2};"
              title="Open {openItem.url}"
            >
              <KeyCap k="o" dim /><span>open</span>
            </button>
          {/if}
        </div>

        <div style="flex:1;overflow-y:auto;padding:20px 28px 32px;">
          <h1 style="margin:0;font:600 22px/1.25 {T.sans};color:{T.ink0};letter-spacing:-0.3px;max-width:720px;">{openItem.title}</h1>

          <!-- Primary link (Reddit permalink / HN / RSS) -->
          {#if openItem.url}
            {@const primaryDomain = new URL(openItem.url).hostname.replace(/^www\./, '')}
            <button
              onclick={() => openExternal(openItem!.url!)}
              style="margin-top:8px;display:inline-flex;align-items:center;gap:6px;background:transparent;border:none;cursor:pointer;padding:0;font:11px/1 {T.mono};color:{T.ink2};"
            >
              <Icon name="ext" size={11} color={T.ink3} />
              <span style="text-decoration:underline;text-underline-offset:2px;text-decoration-color:{T.bd2};">{primaryDomain}</span>
            </button>
          {/if}

          <!-- External link for Reddit link posts -->
          {#if openItem.externalUrl}
            {@const extDomain = (() => { try { return new URL(openItem.externalUrl).hostname.replace(/^www\./, ''); } catch { return openItem.externalUrl; } })()}
            <button
              onclick={() => openExternal(openItem!.externalUrl!)}
              style="margin-top:4px;display:block;font:11px/1.4 {T.mono};color:{T.cyan};background:transparent;border:none;cursor:pointer;padding:0;text-align:left;"
            >
              <Icon name="ext" size={11} color={T.cyan} />
              <span style="margin-left:4px;text-decoration:underline;text-underline-offset:2px;">{openItem.externalUrl}</span>
            </button>
          {/if}

          <div style="margin-top:9px;display:flex;align-items:center;gap:8px;flex-wrap:wrap;">
            {#each openItem.tags as tag}<TagChip {tag} size={10} onclick={() => setActiveTag(tag)} />{/each}
            <span style="flex:1;min-width:12px;"></span>
            <span style="font:10px/1 {T.mono};color:{T.ink2};">signal</span>
            <ScoreBar value={openItem.aiScore} w={36} />
            {#if openItem.score > 0}<span style="font:11px/1 {T.mono};color:{T.amber};">▲ {openItem.score}</span>{/if}
            {#if openItem.n > 0}<span style="font:11px/1 {T.mono};color:{T.ink1};">{openItem.n} comments</span>{/if}
          </div>

          <div style="margin-top:22px;font:14.5px/1.65 {T.sans};color:{T.ink0};max-width:720px;" class="item-body">
            {#if openItem.bodyHtml}
              {@html sanitizeHtml(openItem.bodyHtml)}
            {:else if openItem.body}
              <p style="margin:0;white-space:pre-line;">{openItem.body}</p>
            {/if}
            {#if openItem.url}
              <div style="margin-top:24px;padding-top:16px;border-top:1px solid {T.bd0};display:flex;gap:8px;flex-wrap:wrap;">
                <button
                  onclick={() => openExternal(openItem!.url!)}
                  style="display:inline-flex;align-items:center;gap:8px;background:{T.bg1};border:1px solid {T.bd1};border-radius:3px;padding:10px 16px;cursor:pointer;font:12px/1 {T.mono};color:{T.cyan};"
                >
                  <Icon name="ext" size={13} color={T.cyan} />
                  <span>open post</span>
                </button>
                {#if openItem.externalUrl}
                  <button
                    onclick={() => openExternal(openItem!.externalUrl!)}
                    style="display:inline-flex;align-items:center;gap:8px;background:{T.bg1};border:1px solid {T.bd1};border-radius:3px;padding:10px 16px;cursor:pointer;font:12px/1 {T.mono};color:{T.ink1};"
                  >
                    <Icon name="ext" size={13} color={T.ink2} />
                    <span>open link</span>
                  </button>
                {/if}
              </div>
            {/if}
          </div>
        </div>

        <div style="padding:4px 14px;border-top:1px solid {T.bd0};background:{T.bg1};font:10px/1 {T.mono};color:{T.ink3};display:flex;align-items:center;gap:12px;flex-shrink:0;">
          <span>~{Math.max(1, Math.round((openItem.body || '').split(/\s+/).filter(Boolean).length / 238))}min read</span>
          <span style="flex:1;"></span>
          <span style="color:{T.green};">● readable view</span>
        </div>
      </div>
    {:else}
      <div style="flex:1;display:flex;align-items:center;justify-content:center;flex-direction:column;gap:8px;color:{T.ink3};font:11px/1 {T.mono};">
        <span>select an item</span>
        <span style="font:10px/1 {T.mono};color:{T.ink4};">j/k to navigate · / to search</span>
      </div>
    {/if}

    <!-- AI signal panel -->
    {#if showAI}
      <div style="position:absolute;top:0;right:0;bottom:0;width:300px;background:{T.bg1};border-left:1px solid {T.bd1};z-index:50;display:flex;flex-direction:column;box-shadow:-6px 0 28px rgba(0,0,0,0.45);">
        <div style="padding:10px 14px;border-bottom:1px solid {T.bd0};display:flex;align-items:center;gap:8px;flex-shrink:0;">
          <Icon name="cpu" size={13} color={T.cyan} />
          <span style="font:600 11px/1 {T.mono};color:{T.ink0};letter-spacing:0.8px;text-transform:uppercase;flex:1;">ai signal</span>
          <span style="font:10px/1 {T.mono};color:{T.ink3};">a</span>
          <button onclick={() => { showAI = false; }} style="background:transparent;border:none;cursor:pointer;display:flex;align-items:center;padding:4px;margin-left:4px;">
            <Icon name="x" size={14} color={T.ink1} />
          </button>
        </div>
        <div style="flex:1;overflow-y:auto;padding:14px;display:flex;flex-direction:column;gap:14px;">

          <!-- Current item analysis (desktop-specific: shows open item context) -->
          {#if openItem && openItem.tags.length > 0}
            <div style="padding:10px;background:{T.bg1};border:1px solid {T.bd0};border-radius:4px;">
              <div style="font:9px/1 {T.mono};color:{T.ink3};letter-spacing:0.6px;text-transform:uppercase;margin-bottom:8px;">current item</div>
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

          <!-- Shared AI content (model status, download, tag distribution) -->
          <AiPanelContent compact onTagFilter={setActiveTag} />

        </div>
      </div>
    {/if}

    <!-- Settings panel (slides in from right over detail pane) -->
    {#if showSettings}
      <div style="position:absolute;top:0;right:0;bottom:0;width:300px;background:{T.bg1};border-left:1px solid {T.bd1};z-index:50;display:flex;flex-direction:column;box-shadow:-6px 0 28px rgba(0,0,0,0.45);">
        <div style="padding:10px 14px;border-bottom:1px solid {T.bd0};display:flex;align-items:center;justify-content:space-between;flex-shrink:0;">
          <span style="font:600 11px/1 {T.mono};color:{T.ink0};letter-spacing:0.8px;text-transform:uppercase;">settings</span>
          <button onclick={() => { showSettings = false; }} style="background:transparent;border:none;cursor:pointer;display:flex;align-items:center;padding:4px;">
            <Icon name="x" size={14} color={T.ink1} />
          </button>
        </div>
        <div style="flex:1;overflow-y:auto;padding:14px;display:flex;flex-direction:column;gap:12px;">
          <SettingsPanelContent showShortcuts />
        </div>
      </div>
    {/if}

    <!-- Context menu -->
    {#if contextMenu}
      <div
        data-ctx-menu
        style="
          position:fixed;
          top:{Math.min(contextMenu.y, (typeof window !== 'undefined' ? window.innerHeight : 600) - 280)}px;
          left:{Math.min(contextMenu.x, (typeof window !== 'undefined' ? window.innerWidth : 1000) - 212)}px;
          width:200px;
          background:{T.bg1};
          border:1px solid {T.bd1};
          border-radius:4px;
          box-shadow:0 8px 32px rgba(0,0,0,0.6);
          z-index:200;
          overflow:hidden;
          font:11px/1 {T.mono};
        "
      >
        {#each [contextMenu.item] as ci}
        {@const isHnSelf = ci.url?.includes('news.ycombinator.com/item') ?? false}
        {#if ci.url && !isHnSelf}
          <button
            onclick={() => { openExternal(ci.url!); contextMenu = null; }}
            style="display:flex;align-items:center;gap:10px;width:100%;padding:9px 12px;background:transparent;border:none;border-bottom:1px solid {T.bd0};color:{T.ink0};cursor:pointer;text-align:left;"
            onmouseenter={(e) => (e.currentTarget as HTMLElement).style.background = T.bg2}
            onmouseleave={(e) => (e.currentTarget as HTMLElement).style.background = 'transparent'}
          >
            <Icon name="ext" size={11} color={T.ink2} />
            <span>Open in browser</span>
          </button>
        {/if}
        {#if ci.url}
          <button
            onclick={() => ctxCopyUrl(ci.url!)}
            style="display:flex;align-items:center;gap:10px;width:100%;padding:9px 12px;background:transparent;border:none;border-bottom:1px solid {T.bd0};color:{T.ink0};cursor:pointer;text-align:left;"
            onmouseenter={(e) => (e.currentTarget as HTMLElement).style.background = T.bg2}
            onmouseleave={(e) => (e.currentTarget as HTMLElement).style.background = 'transparent'}
          >
            <Icon name="link" size={11} color={T.ink2} />
            <span>Copy URL</span>
          </button>
        {/if}
        <button
          onclick={() => ctxCopyTitle(ci.title)}
          style="display:flex;align-items:center;gap:10px;width:100%;padding:9px 12px;background:transparent;border:none;border-bottom:1px solid {T.bd1};color:{T.ink0};cursor:pointer;text-align:left;"
          onmouseenter={(e) => (e.currentTarget as HTMLElement).style.background = T.bg2}
          onmouseleave={(e) => (e.currentTarget as HTMLElement).style.background = 'transparent'}
        >
          <Icon name="edit" size={11} color={T.ink2} />
          <span>Copy title</span>
        </button>
        <div style="height:1px;background:{T.bd0};margin:2px 0;"></div>
        <button
          onclick={() => { markRead(ci.id, !ci.read); contextMenu = null; }}
          style="display:flex;align-items:center;gap:10px;width:100%;padding:9px 12px;background:transparent;border:none;border-bottom:1px solid {T.bd0};color:{ci.read ? T.ink1 : T.cyan};cursor:pointer;text-align:left;"
          onmouseenter={(e) => (e.currentTarget as HTMLElement).style.background = T.bg2}
          onmouseleave={(e) => (e.currentTarget as HTMLElement).style.background = 'transparent'}
        >
          <Icon name="check" size={11} color={ci.read ? T.ink2 : T.cyan} />
          <span>{ci.read ? 'Mark as unread' : 'Mark as read'}</span>
        </button>
        <button
          onclick={() => { toggleSaved(ci.id); contextMenu = null; }}
          style="display:flex;align-items:center;gap:10px;width:100%;padding:9px 12px;background:transparent;border:none;border-bottom:1px solid {T.bd0};color:{ci.saved ? T.amber : T.ink1};cursor:pointer;text-align:left;"
          onmouseenter={(e) => (e.currentTarget as HTMLElement).style.background = T.bg2}
          onmouseleave={(e) => (e.currentTarget as HTMLElement).style.background = 'transparent'}
        >
          <Icon name="star" size={11} color={ci.saved ? T.amber : T.ink2} />
          <span>{ci.saved ? 'Unsave' : 'Save'}</span>
        </button>
        <button
          onclick={() => { const cur = filteredItems.findIndex(i => i.id === ci.id); const fallback = filteredItems[Math.max(0, cur - 1)]; hideItem(ci.id); openId = (fallback && fallback.id !== ci.id) ? fallback.id : ''; contextMenu = null; }}
          style="display:flex;align-items:center;gap:10px;width:100%;padding:9px 12px;background:transparent;border:none;border-bottom:1px solid {T.bd1};color:{T.red};cursor:pointer;text-align:left;"
          onmouseenter={(e) => (e.currentTarget as HTMLElement).style.background = T.bg2}
          onmouseleave={(e) => (e.currentTarget as HTMLElement).style.background = 'transparent'}
        >
          <Icon name="eye-off" size={11} color={T.red} />
          <span>Hide</span>
        </button>
        <div style="height:1px;background:{T.bd0};margin:2px 0;"></div>
        <button
          onclick={() => { openItemAndRead(ci.id); contextMenu = null; }}
          style="display:flex;align-items:center;gap:10px;width:100%;padding:9px 12px;background:transparent;border:none;color:{T.ink1};cursor:pointer;text-align:left;"
          onmouseenter={(e) => (e.currentTarget as HTMLElement).style.background = T.bg2}
          onmouseleave={(e) => (e.currentTarget as HTMLElement).style.background = 'transparent'}
        >
          <Icon name="cpu" size={11} color={T.ink2} />
          <span>Tag info</span>
        </button>
        {/each}
      </div>
    {/if}

    <!-- Keyboard shortcut cheatsheet -->
    {#if showCheatsheet}
      <div
        role="dialog"
        style="position:absolute;inset:0;display:flex;align-items:center;justify-content:center;z-index:300;pointer-events:none;"
      >
        <div
          role="button"
          tabindex="-1"
          onkeydown={(e) => { if (e.key === 'Escape') showCheatsheet = false; }}
          onclick={() => showCheatsheet = false}
          style="position:absolute;inset:0;background:rgba(0,0,0,0.5);pointer-events:all;"
        ></div>
        <div
          style="
            position:relative;
            background:{T.bg1};
            border:1px solid {T.bd1};
            border-radius:6px;
            padding:20px 24px;
            width:440px;
            box-shadow:0 16px 48px rgba(0,0,0,0.7);
            pointer-events:all;
            z-index:1;
          "
        >
          <div style="display:flex;align-items:center;justify-content:space-between;margin-bottom:16px;">
            <span style="font:600 11px/1 {T.mono};color:{T.ink0};letter-spacing:0.8px;text-transform:uppercase;">keyboard shortcuts</span>
            <button onclick={() => showCheatsheet = false} style="background:transparent;border:none;cursor:pointer;padding:2px;">
              <Icon name="x" size={13} color={T.ink2} />
            </button>
          </div>
          <div style="display:grid;grid-template-columns:1fr 1fr;gap:0 24px;">
            {#each [
              { k: '/',       desc: 'focus search'      },
              { k: 'j / ↓',  desc: 'next item'         },
              { k: 'k / ↑',  desc: 'prev item'         },
              { k: 'm',       desc: 'toggle read'       },
              { k: 's',       desc: 'save / unsave'     },
              { k: 'o',       desc: 'open in browser'   },
              { k: 'x',       desc: 'hide item'         },
              { k: 'a',       desc: 'AI signal panel'   },
              { k: '?',       desc: 'this cheatsheet'   },
              { k: 'Esc',     desc: 'clear / back'      },
            ] as sc, i}
              <div style="display:flex;align-items:center;gap:10px;padding:6px 0;border-bottom:1px solid {T.bd0};">
                <KeyCap k={sc.k} />
                <span style="font:11px/1 {T.mono};color:{T.ink2};">{sc.desc}</span>
              </div>
            {/each}
          </div>
          <div style="margin-top:12px;font:10px/1 {T.mono};color:{T.ink3};text-align:center;">press ? or Esc to close</div>
        </div>
      </div>
    {/if}
  </div>

  <!-- Status bar -->
  <div style="height:24px;display:flex;align-items:center;padding:0 10px;gap:14px;border-top:1px solid {T.bd0};background:{T.bg1};flex-shrink:0;font:10px/1 {T.mono};color:{T.ink2};">
    <span style="color:{T.bg0};background:{T.cyan};padding:3px 6px;border-radius:2px;font:600 9px/1 {T.mono};letter-spacing:0.6px;">NORMAL</span>
    <span><span style="color:{T.ink3};">group:</span> {activeGroupLabel}</span>
    {#if activeSource}<span style="color:{T.ink4};">·</span><span><span style="color:{T.ink3};">src:</span> {sources.find(s => s.id === activeSource)?.name}</span>{/if}
    <span style="color:{T.ink4};">·</span>
    <span title="{filteredItems.length} items in view · {items.length} total loaded"><span style="color:{T.ink3};">items:</span> <span style="color:{T.ink0};">{filteredItems.length}</span> / {items.length}</span>
    <span style="color:{T.ink4};">·</span>
    <span title="{unreadCount} unread in current view"><span style="color:{T.ink3};">unread:</span> <span style="color:{unreadCount > 0 ? T.cyan : T.ink3};">{unreadCount}</span></span>
    <span style="color:{T.ink4};">·</span>
    <span title="{taggedCount} items with at least one AI tag in current view"><span style="color:{T.ink3};">tagged:</span> <span style="color:{T.amber};">{taggedCount}</span></span>
    {#if searchQuery}<span style="color:{T.ink4};">·</span><span style="color:{T.amber};">"{searchQuery}"</span>{/if}
    <span style="flex:1;"></span>
    <span title="last sync: {syncState.lastSyncAt}{syncState.lastNewCount > 0 ? ` · +${syncState.lastNewCount} new` : ''}">
      <span style="color:{syncing ? T.cyan : T.green};">●</span>
      <span style="color:{T.ink3};"> sync</span>
      <span style="color:{T.ink1};"> {syncState.lastSyncAt}</span>
      {#if syncState.lastNewCount > 0}<span style="color:{T.cyan};"> +{syncState.lastNewCount}</span>{/if}
    </span>
    <span style="color:{T.ink4};">·</span>
    <span><span style="color:{T.ink3};">ai</span> <span style="color:{settings.aiTagging ? T.amber : T.ink3};">{settings.aiTagging ? 'on' : 'off'}</span></span>
    <span style="color:{T.ink4};">·</span>
    <button onclick={() => showCheatsheet = !showCheatsheet} style="background:transparent;border:none;cursor:pointer;font:10px/1 {T.mono};color:{T.ink3};padding:0;" title="keyboard shortcuts (?)">?</button>
  </div>
</div>
{/if}
