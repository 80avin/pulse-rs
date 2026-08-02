<script lang="ts">
  import { T, TAG_COLORS } from '$lib/tokens';
  import { items, sources, groups, storeReady, markRead, toggleSaved, markAllRead, hideItem, dbStats } from '$lib/stores/data.svelte';
  import { doSync as storeSync, syncState } from '$lib/stores/sync.svelte';
  import { searchItems } from '$lib/stores/search.svelte';
  import { timelineFilter, applyFilter, setFeedFilter, setGroupFilter, setTagFilter, pageCounts } from '$lib/stores/timeline.svelte';
  import { settings } from '$lib/settings.svelte';
  import { openExternal } from '$lib/utils';
  import Icon from '$lib/components/Icon.svelte';
  import KeyCap from '$lib/components/KeyCap.svelte';
  import DragHandle from '$lib/components/DragHandle.svelte';
  import FilterPills from '$lib/components/FilterPills.svelte';
  import BottomTools from '$lib/components/BottomTools.svelte';
  import StatusBar from '$lib/components/StatusBar.svelte';
  import StatusDot from '$lib/components/StatusDot.svelte';
  import SettingsPanelContent from '$lib/components/SettingsPanelContent.svelte';
  import SourceExplorer from '$lib/components/SourceExplorer.svelte';
  import TimelinePane from '$lib/components/TimelinePane.svelte';
  import ReaderPane from '$lib/components/ReaderPane.svelte';
  import GroupTabs from '$lib/components/GroupTabs.svelte';
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
  const desktopFilter = $derived<'all'|'unread'|'saved'>(
    timelineFilter.isRead === false ? 'unread' :
    timelineFilter.isSaved === true ? 'saved' : 'all'
  );
  let activeTag    = $derived(timelineFilter.tag ?? null);
  const density    = $derived(settings.density);
  let searchQuery  = $state('');
  let ftsResults   = $state<import('$lib/types').FeedItem[] | null>(null);
  let searchInputEl: HTMLInputElement | null = $state(null);
  let showSettings  = $state(false);
  let showSources   = $state(false);
  let accValue = $state<string[]>(['sources']);
  let showSourcesAccordion = $derived(accValue.includes('sources'));
  let popoverOpen   = $state(false);
  let showCheatsheet = $state(false);

  const IS_TAURI = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

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
    return list;
  });

  const openItem    = $derived(items.find(i => i.id === openId));
  const openSource  = $derived(openItem ? sources.find(s => s.id === openItem.src) : undefined);
  const groupSources = $derived(activeGroup === 'all' ? sources : sources.filter(s=>s.group===activeGroup));
  const unreadCount = $derived(pageCounts.unread);
  const taggedCount = $derived(dbStats.tagCount);

  function selectGroup(id: string) { searchQuery = ''; setGroupFilter(id === 'all' ? null : id); }
  function setActiveTag(tag: string) {
    const next = activeTag === tag ? null : tag;
    setTagFilter(next);
  }

  function handleFilterChange(filter: string) {
    switch (filter) {
      case 'all':    applyFilter({ isRead: null, isSaved: null }); break;
      case 'unread': applyFilter({ isRead: false, isSaved: null }); break;
      case 'saved':  applyFilter({ isRead: null, isSaved: true }); break;
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
  <div class="flex items-center justify-center w-full h-full bg-bg-0 text-red text-[11px] leading-[1.6] font-mono">
    failed to load data — check console for details
  </div>
{:else}
<div class="flex flex-col w-full h-full bg-bg-0 text-ink-0 overflow-hidden">

  <!-- App toolbar (window controls are handled by the native OS title bar) -->
  <div class="h-8 flex items-center pl-3 pr-2 bg-bg-0 border-b border-bd-0 shrink-0 gap-2.5">
    <span class="text-ink-2 text-[11px] leading-none font-mono">{activeGroupLabel} · {displayItems.length} items</span>
    <span class="flex-1"></span>
    <button onclick={() => searchInputEl?.focus()} aria-label="Search" class="inline-flex items-center justify-center bg-transparent border-none cursor-pointer rounded w-5.5 h-5.5" title="Search (/)">
      <Icon name="search" size={13} color={T.ink1} />
    </button>
    <button onclick={doSync} aria-label="Sync feeds" class="inline-flex items-center justify-center bg-transparent border-none cursor-pointer rounded w-5.5 h-5.5" title="Sync">
      <span class={syncState.syncing ? 'syncing' : ''}><Icon name="sync" size={13} color={syncState.syncing ? T.cyan : T.ink1} /></span>
    </button>
    <button onclick={() => showCheatsheet = !showCheatsheet} aria-label="Keyboard shortcuts (?)" class="bg-transparent border-none cursor-pointer text-ink-3 px-1 text-[10px] leading-none font-mono" title="Keyboard shortcuts (?)">?</button>
  </div>

  <!-- Main body -->
  <div
    class="flex-1 flex overflow-hidden relative"
    onmousemove={onMouseMove}
    onmouseup={stopDrag}
    onmouseleave={stopDrag}
  >

    <!-- Left rail -->
    {#if leftRailCollapsed}
      <div class="w-8 shrink-0 bg-bg-1 border-r border-bd-0 flex flex-col items-center pt-1 overflow-hidden gap-1.5">
        <button onclick={toggleLeftRail} aria-label="Expand sidebar" title="Expand sidebar" class="w-6 h-6 flex items-center justify-center bg-transparent border-none cursor-pointer rounded" onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.background = T.bg2; }} onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.background = 'transparent'; }}>
          <Icon name="chev-r" size={13} color={T.ink2} />
        </button>
        {#each groups as g}
          <button
            onclick={() => selectGroup(g.id)}
            onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.background = T.bg2; }}
            onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.background = 'transparent'; }}
            title={`${g.name} (${g.n})`}
            aria-label={`${g.name} group, ${g.n} items`}
            class="w-6 h-6 flex items-center justify-center border-none cursor-pointer relative rounded-[3px]"
            style="background:{g.id===activeGroup?'rgba(78,205,214,0.12)':'transparent'}"
          >
            <span class="font-bold text-[10px] leading-none font-mono text-cyan={g.id===activeGroup} text-ink-2={g.id!==activeGroup}">{g.name.slice(0, 2).toUpperCase()}</span>
            {#if g.n > 0}<span class="absolute top-0 right-0 w-1.5 h-1.5 rounded-full bg-cyan"></span>{/if}
          </button>
        {/each}
        <div class="flex-1"></div>
        <button onclick={() => { showSources = !showSources; }} aria-label="Sources" title="Sources" class="w-6 h-6 flex items-center justify-center bg-transparent border-none cursor-pointer rounded" onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.background = T.bg2; }} onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.background = 'transparent'; }}>
          <Icon name="rss" size={13} color={showSources ? T.cyan : T.ink2} />
        </button>
        <button onclick={() => { showSettings = !showSettings; }} aria-label="Settings" title="Settings" class="w-6 h-6 flex items-center justify-center bg-transparent border-none cursor-pointer mb-1.5 rounded" onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.background = T.bg2; }} onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.background = 'transparent'; }}>
          <Icon name="cog" size={13} color={showSettings ? T.cyan : T.ink2} />
        </button>
      </div>
    {:else}
    <div class="shrink-0 bg-bg-1 border-r border-bd-0 flex flex-col overflow-hidden" style="width:{leftRailWidth}px">
      <!-- Collapse toggle -->
      <div class="flex justify-end pt-0.5 px-1">
        <button onclick={toggleLeftRail} aria-label="Collapse sidebar" title="Collapse sidebar" class="flex items-center justify-center bg-transparent border-none cursor-pointer rounded w-5.5 h-5.5" onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.background = T.bg2; }} onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.background = 'transparent'; }}>
          <Icon name="chev-l" size={12} color={T.ink3} />
        </button>
      </div>
      <!-- Search -->
      <div class="px-2 pt-2 pb-1">
        <div class="flex items-center gap-1.5 bg-bg-0 border border-bd-1 p-1.25 px-2 rounded">
          <Icon name="search" size={11} color={T.ink3} />
          <input
            bind:this={searchInputEl}
            bind:value={searchQuery}
            placeholder="search {dbStats.totalItems} items"
            class="flex-1 text-[11px] leading-none font-mono"
            aria-label="Search items"
          />
          <KeyCap k="/" dim />
        </div>
      </div>

      <!-- Filter pills -->
      <FilterPills active={desktopFilter} onChange={handleFilterChange} />

      <!-- Groups (vertical) -->
      <div class="border-b border-bd-0 pt-0.5 pb-1">
        <div class="uppercase text-ink-3 px-3 pt-1.5 pb-0.5 tracking-[0.6px] text-[10px] leading-none font-mono">groups</div>
        <GroupTabs {groups} active={activeGroup} onSelect={(id) => selectGroup(id)} orientation="vertical" />
      </div>

      <!-- Sources accordion -->
      <Accordion.Root type="multiple" bind:value={accValue} class="border-b border-bd-0">
        <Accordion.Item value="sources">
          <Accordion.Header>
            <Accordion.Trigger
              onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.background = T.bg2; }}
              onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.background = 'transparent'; }}
              class="flex items-center gap-1 w-full bg-transparent border-none cursor-pointer text-left p-1.5 px-3"
              aria-label={`Sources for ${activeGroupLabel}`}
            >
              <span class="uppercase flex-1 text-ink-3 tracking-[0.6px] text-[10px] leading-none font-mono">sources</span>
              <span class="text-ink-2 text-[10px] leading-none font-mono">{groupSources.length}</span>
              <Icon name={showSourcesAccordion ? 'chev-dn' : 'chev-r'} size={10} color={T.ink3} />
            </Accordion.Trigger>
          </Accordion.Header>
          <Accordion.Content class="p-[0_8px_6px]">
            {#each groupSources as s}
              <button
                onclick={() => { const next = activeSource === s.id ? null : s.id; setFeedFilter(next); }}
                oncontextmenu={(e) => { e.preventDefault(); showSources = !showSources; }}
                onmouseenter={(e) => { if (activeSource!==s.id)(e.currentTarget as HTMLElement).style.background = T.bg2; }}
                onmouseleave={(e) => { if (activeSource!==s.id)(e.currentTarget as HTMLElement).style.background = 'transparent'; }}
                onfocus={(e) => { (e.currentTarget as HTMLElement).style.outline = `1px solid ${T.cyan}`; }}
                onblur={(e) => { (e.currentTarget as HTMLElement).style.outline = 'none'; }}
                class="flex items-center gap-1.5 w-full border-none cursor-pointer text-left px-1.5 py-1"
                style="background:{activeSource===s.id ? 'rgba(78,205,214,0.06)' : 'transparent'};border-left:2px solid {activeSource===s.id ? T.cyan : 'transparent'}"
                aria-current={activeSource===s.id ? 'true' : undefined}
              >
                <StatusDot status={s.status} size={5} />
                <span class="overflow-hidden text-ellipsis whitespace-nowrap flex-1 text-[11px] leading-[1.2] font-mono" style="color:{activeSource===s.id ? T.ink0 : T.ink1};">{s.name}</span>
                {#if s.unread > 0}<span class="text-cyan text-[10px] leading-none font-mono">{s.unread}</span>{/if}
              </button>
            {/each}
            <button
              onclick={() => { showSources = !showSources; }}
              onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.background = T.bg2; }}
              onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.background = 'transparent'; }}
              class="flex items-center gap-1 w-full bg-transparent border-none text-ink-3 cursor-pointer text-left px-1.5 py-1 mt-0.5 text-[10px] leading-none font-mono"
            >
              <Icon name="plus" size={10} color={T.ink3} />
              <span>Manage Sources</span>
            </button>
          </Accordion.Content>
        </Accordion.Item>
      </Accordion.Root>

      <!-- Spacer -->
      <div class="flex-1"></div>

      <!-- Bottom utilities -->
      <BottomTools
        {showSources} {showSettings}
        syncing={syncState.syncing}
        {syncState}
        onToggleSources={() => { showSources = !showSources; }}
        onToggleSettings={() => { showSettings = !showSettings; }}
      />
    </div>
    {/if}

    <!-- Drag handle: left rail ↔ timeline -->
    <DragHandle edge="left" {onDragStart} {dragging} />

    <!-- Timeline pane -->
    <div class="shrink-0 flex flex-col border-r border-bd-0 overflow-hidden bg-bg-0" style="width:{timelineWidth}px">
      <TimelinePane mode="wide" items={displayItems} {searchQuery} {openId} onOpen={openItemAndRead} />
    </div>

    <!-- Drag handle: timeline ↔ reader -->
    <DragHandle edge="timeline" {onDragStart} {dragging} />

    <!-- Detail pane -->
    {#if openItem}
      <div class="flex-1 min-w-0 flex flex-col bg-bg-0 overflow-hidden">
        <ReaderPane mode="wide" itemId={openItem.id} allIds={displayItems.map(i => i.id)} onBack={() => { openId = ''; }} onNavigate={openItemAndRead} />
      </div>
    {/if}

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
      <div class="grid grid-cols-2 gap-x-6">
        {#each [
          { k: '/',       desc: 'focus search'      },
          { k: 'j / ↓',  desc: 'next item'         },
          { k: 'k / ↑',  desc: 'prev item'         },
          { k: 'm',       desc: 'toggle read'       },
          { k: 's',       desc: 'save / unsave'     },
          { k: 'o',       desc: 'open in browser'   },
          { k: 'x',       desc: 'hide item'         },
          { k: 'r',       desc: 'sources'           },
          { k: 'Esc',     desc: 'clear / close'     },
        ] as sc}
          <div class="flex items-center gap-2.5 border-b border-bd-0 py-1.5">
            <KeyCap k={sc.k} />
            <span class="text-ink-2 text-[11px] leading-none font-mono">{sc.desc}</span>
          </div>
        {/each}
      </div>
      <div class="mt-3 text-ink-3 text-center text-[10px] leading-none font-mono">press ? or Esc to close</div>
    </Modal>
  </div>

  <!-- Status bar -->
  <StatusBar
    density={settings.density}
    
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
