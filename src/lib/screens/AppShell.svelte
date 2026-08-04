<script lang="ts">
  import { onMount } from 'svelte';
  import { T } from '$lib/tokens';
  import { items, sources, groups, storeReady, markRead, toggleSaved, hideItem, dbStats, markSourceRead, removeSource, syncSource as storeSyncSource } from '$lib/stores/data.svelte';
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
  import Onboarding from '$lib/components/Onboarding.svelte';
  import ItemActionsMenu from '$lib/components/ItemActionsMenu.svelte';
  import PulseBottomNav from '$lib/components/PulseBottomNav.svelte';
  import MobileSaved from './MobileSaved.svelte';
  import SearchView from '$lib/components/SearchView.svelte';
  import ContextMenu from '$lib/components/shared/ContextMenu.svelte';
  import { SOURCE_ACTIONS, type SourceActionKind } from '$lib/source-actions';
  import { anyOverlayOpen } from '$lib/stores/overlays.svelte';
  import { isDesktop } from '$lib/use-is-desktop.svelte';
  import { TABS, isTabId, type TabId } from '$lib/nav';

  // ── Shared navigation state (one shell, both breakpoints) ───────────────
  let openId = $state('');
  let timelineIds = $state<string[]>([]);
  let activeTab = $state<TabId>('feed');

  const IS_TAURI = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

  const isWide = $derived(isDesktop());

  function openItem(id: string, ids: string[]) {
    openId = id;
    timelineIds = ids;
    // Push a history entry so Android's system back closes the reader instead
    // of exiting the app (popstate restores the previous state).
    history.pushState({ tab: activeTab, openItemId: id } satisfies NavState, '');
  }
  function openItemAndRead(id: string, ids: string[]) {
    openId = id;
    timelineIds = ids;
    if (settings.markReadOn === 'open') markRead(id);
  }
  function closeReader() { openId = ''; }

  // Narrow navigation (history-backed so Android back unwinds in-app)
  type NavState = { tab: string; openItemId: string | null };
  function changeTab(newTab: string) {
    if (!isTabId(newTab)) { activeTab = 'feed'; newTab = 'feed'; }
    if (newTab === activeTab && !openId) return;
    history.pushState({ tab: newTab, openItemId: null } satisfies NavState, '');
    activeTab = newTab as TabId;
    openId = '';
  }
  function goBack() { openId = ''; history.back(); }
  function goToSearch() { changeTab('search'); }
  function openSourceFeed(sourceId: string) { setFeedFilter(sourceId); changeTab('feed'); }
  function handleTagFilter(tag: string) { setTagFilter(tag); changeTab('feed'); }

  onMount(() => {
    history.replaceState({ tab: 'feed', openItemId: null } satisfies NavState, '');
    function handlePop(e: PopStateEvent) {
      const s = e.state as NavState | null;
      activeTab = (s?.tab && isTabId(s.tab)) ? s.tab : 'feed';
      openId = s?.openItemId ?? '';
    }
    window.addEventListener('popstate', handlePop);
    return () => window.removeEventListener('popstate', handlePop);
  });

  // ── Wide-only state (rail, panes, search, modals) ────────────────────────
  let activeGroup  = $derived(timelineFilter.groupId ?? 'all');
  let activeSource = $derived(timelineFilter.feedId ?? null);

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

  const desktopFilter = $derived<'all'|'unread'|'saved'>(
    timelineFilter.isRead === false ? 'unread' :
    timelineFilter.isSaved === true ? 'saved' : 'all'
  );
  let activeTag    = $derived(timelineFilter.tag ?? null);
  let searchQuery  = $state('');
  let ftsResults   = $state<import('$lib/types').FeedItem[] | null>(null);
  let searchInputEl: HTMLInputElement | null = $state(null);
  let showSettings  = $state(false);
  let showSources   = $state(false);
  let showSourcesAccordion = $state(true);
  let showCheatsheet = $state(false);
  let showOnboarding = $state(false);
  const ONBOARDING_DONE_KEY = 'pulse:onboarding-done';

  // First-run onboarding: auto-show once cold-start finishes with no sources,
  // and only if it hasn't been dismissed before.
  $effect(() => {
    if (!storeReady.loading && sources.length === 0 && !localStorage.getItem(ONBOARDING_DONE_KEY)) {
      showOnboarding = true;
    }
  });
  function finishOnboarding() {
    localStorage.setItem(ONBOARDING_DONE_KEY, '1');
    showOnboarding = false;
  }

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
    if (IS_TAURI && ftsResults !== null && searchQuery.trim()) return ftsResults;
    let list = items as typeof items;
    if (!IS_TAURI && searchQuery.trim()) {
      const q = searchQuery.toLowerCase();
      list = list.filter(i =>
        i.title.toLowerCase().includes(q) || (i.snippet?.toLowerCase().includes(q) ?? false)
      );
    }
    return list;
  });
  const openItemObj = $derived(items.find(i => i.id === openId));
  const groupSources = $derived(activeGroup === 'all' ? sources : sources.filter(s=>s.group===activeGroup));
  const unreadCount = $derived(pageCounts.unread);
  const taggedCount = $derived(dbStats.tagCount);

  function selectGroup(id: string) { searchQuery = ''; setGroupFilter(id === 'all' ? null : id); }
  function setActiveTag(tag: string) { setTagFilter(activeTag === tag ? null : tag); }
  function handleFilterChange(filter: string) {
    switch (filter) {
      case 'all':    applyFilter({ isRead: null, isSaved: null }); break;
      case 'unread': applyFilter({ isRead: false, isSaved: null }); break;
      case 'saved':  applyFilter({ isRead: null, isSaved: true }); break;
    }
  }
  async function doSync() { await storeSync(); }

  // Right-click source context menu
  let sourceMenu = $state<{ source: import('$lib/types').Source; x: number; y: number } | null>(null);
  function openSourceMenu(e: MouseEvent, s: import('$lib/types').Source) {
    e.preventDefault();
    sourceMenu = { source: s, x: e.clientX, y: e.clientY };
  }
  function closeSourceMenu() { sourceMenu = null; }
  function sourceMenuView(id: string) {
    setFeedFilter(activeSource === id ? null : id);
    closeSourceMenu();
  }
  async function sourceMenuRefresh(id: string) { await storeSyncSource(id); closeSourceMenu(); }
  async function sourceMenuMarkRead(id: string) { await markSourceRead(id); closeSourceMenu(); }
  async function sourceMenuRemove(id: string) { await removeSource(id); closeSourceMenu(); }
  function sourceMenuEdit() { closeSourceMenu(); showSources = true; }
  function runSourceAction(kind: SourceActionKind) {
    if (kind === 'edit') { sourceMenuEdit(); return; }
    const id = sourceMenu?.source.id;
    if (!id) return;
    switch (kind) {
      case 'view':      sourceMenuView(id); break;
      case 'refresh':   sourceMenuRefresh(id); break;
      case 'mark-read': sourceMenuMarkRead(id); break;
      case 'remove':    sourceMenuRemove(id); break;
      default: break;
    }
  }

  // Wide keyboard shortcuts
  $effect(() => {
    function onKey(e: KeyboardEvent) {
      if (!isWide) return;
      if (showOnboarding) { if (e.key === 'Escape') showOnboarding = false; return; }
      if (anyOverlayOpen()) return;
      const target = e.target as HTMLElement;
      const inInput = target instanceof HTMLInputElement || target instanceof HTMLTextAreaElement;

      if (e.key === 'Escape') {
        if (showCheatsheet) { showCheatsheet = false; return; }
        if (searchQuery)  { searchQuery = ''; return; }
        searchInputEl?.blur();
        return;
      }
      if (e.key === '?' && !inInput) { showCheatsheet = !showCheatsheet; return; }
      if (e.key === 'r' && !inInput) { showSources = !showSources; return; }
      if (e.key === '/' && !inInput) { e.preventDefault(); searchInputEl?.focus(); return; }
      if (inInput) return;

      switch (e.key) {
        case 'j': case 'ArrowDown': {
          const cur = displayItems.findIndex(i => i.id === openId);
          const next = displayItems[Math.min(cur + 1, displayItems.length - 1)];
          if (next) openItemAndRead(next.id, displayItems.map(i => i.id));
          break;
        }
        case 'k': case 'ArrowUp': {
          const cur = displayItems.findIndex(i => i.id === openId);
          if (cur <= 0) break;
          const prev = displayItems[cur - 1];
          if (prev) openItemAndRead(prev.id, displayItems.map(i => i.id));
          break;
        }
        case 'm': if (openItemObj) markRead(openItemObj.id, !openItemObj.read); break;
        case 's': if (openItemObj) toggleSaved(openItemObj.id); break;
        case 'o': if (openItemObj?.domain) openExternal(openItemObj.url ?? `https://${openItemObj.domain}`); break;
        case 'h':
        case 'x': {
          if (!openItemObj) break;
          const cur = displayItems.findIndex(i => i.id === openItemObj!.id);
          const fallback = displayItems[Math.max(0, cur - 1)];
          hideItem(openItemObj.id);
          openId = (fallback && fallback.id !== openItemObj!.id) ? fallback.id : '';
          break;
        }
      }
    }
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  });
</script>

<div class="relative w-full h-full overflow-hidden">
  {#if storeReady.error}
    <div class="flex items-center justify-center w-full h-full bg-bg-0 text-red text-[11px] leading-[1.6] font-mono">
      failed to load data — check console for details
    </div>
  {:else if isWide}
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
      <div class="flex-1 flex overflow-hidden relative" role="presentation" onmousemove={onMouseMove} onmouseup={stopDrag} onmouseleave={stopDrag}>
        {#if leftRailCollapsed}
          <div class="w-8 shrink-0 bg-bg-1 border-r border-bd-0 flex flex-col items-center pt-1 overflow-hidden gap-1.5">
            <button onclick={toggleLeftRail} aria-label="Expand sidebar" title="Expand sidebar" class="w-6 h-6 flex items-center justify-center bg-transparent border-none cursor-pointer rounded">
              <Icon name="chev-r" size={13} color={T.ink2} />
            </button>
            {#each groups as g}
              <button onclick={() => selectGroup(g.id)} title={`${g.name} (${g.n})`} aria-label={`${g.name} group, ${g.n} unread`} class="w-6 h-6 flex items-center justify-center border-none cursor-pointer relative rounded-[3px] hover:bg-bg-2" style={g.id===activeGroup?'background:rgba(78,205,214,0.12)':undefined}>
                <span class="font-bold text-[10px] leading-none font-mono text-cyan={g.id===activeGroup} text-ink-2={g.id!==activeGroup}">{g.name.slice(0, 2).toUpperCase()}</span>
                {#if g.n > 0}<span class="absolute top-0 right-0 w-1.5 h-1.5 rounded-full bg-cyan"></span>{/if}
              </button>
            {/each}
            <div class="flex-1"></div>
            <button onclick={() => { showSources = !showSources; }} aria-label="Sources" title="Sources" class="w-6 h-6 flex items-center justify-center bg-transparent border-none cursor-pointer rounded">
              <Icon name="rss" size={13} color={showSources ? T.cyan : T.ink2} />
            </button>
            <button onclick={() => { showSettings = !showSettings; }} aria-label="Settings" title="Settings" class="w-6 h-6 flex items-center justify-center bg-transparent border-none cursor-pointer mb-1.5 rounded">
              <Icon name="cog" size={13} color={showSettings ? T.cyan : T.ink2} />
            </button>
          </div>
        {:else}
          <div class="shrink-0 bg-bg-1 border-r border-bd-0 flex flex-col overflow-hidden" style="width:{leftRailWidth}px">
            <div class="flex justify-end pt-0.5 px-1">
              <button onclick={toggleLeftRail} aria-label="Collapse sidebar" title="Collapse sidebar" class="flex items-center justify-center bg-transparent border-none cursor-pointer rounded w-5.5 h-5.5">
                <Icon name="chev-l" size={12} color={T.ink3} />
              </button>
            </div>
            <div class="px-2 pt-2 pb-1">
              <div class="flex items-center gap-1.5 bg-bg-0 border border-bd-1 p-1.25 px-2 rounded">
                <Icon name="search" size={11} color={T.ink3} />
                <input bind:this={searchInputEl} bind:value={searchQuery} placeholder="search {dbStats.totalItems} items" class="flex-1 text-[11px] leading-none font-mono" aria-label="Search items" />
                <KeyCap k="/" dim />
              </div>
            </div>
            <FilterPills active={desktopFilter} onChange={handleFilterChange} />
            <div class="border-b border-bd-0 pt-0.5 pb-1 shrink-0">
              <div class="uppercase text-ink-2 px-3 pt-1.5 pb-0.5 tracking-[0.6px] text-[10px] leading-none font-mono">groups</div>
              <div class="max-h-40 overflow-y-auto">
                <GroupTabs {groups} active={activeGroup} onSelect={(id) => selectGroup(id)} orientation="vertical" />
              </div>
            </div>
            <div class="flex-1 min-h-0 flex flex-col border-b border-bd-0">
              <button onclick={() => showSourcesAccordion = !showSourcesAccordion} aria-label={`Sources for ${activeGroupLabel}`} aria-expanded={showSourcesAccordion} class="flex items-center gap-1 w-full bg-transparent border-none cursor-pointer text-left p-1.5 px-3 pt-2.5 shrink-0">
                <span class="uppercase flex-1 text-ink-2 tracking-[0.6px] text-[10px] leading-none font-mono">sources</span>
                <span class="text-ink-2 text-[10px] leading-none font-mono">{groupSources.length}</span>
                <Icon name={showSourcesAccordion ? 'chev-dn' : 'chev-r'} size={10} color={T.ink3} />
              </button>
              {#if showSourcesAccordion}
                <div class="flex-1 min-h-0 overflow-y-auto px-2">
                  {#each groupSources as s}
                    <button onclick={() => { const next = activeSource === s.id ? null : s.id; setFeedFilter(next); }} oncontextmenu={(e) => openSourceMenu(e, s)} class="flex items-center gap-1.5 w-full border-none cursor-pointer text-left px-1.5 py-1 hover:bg-bg-2 focus-visible:outline-1 focus-visible:outline-[var(--color-cyan)]" style={activeSource===s.id ? `background:rgba(78,205,214,0.06);border-left:2px solid ${T.cyan}` : 'border-left:2px solid transparent'} aria-current={activeSource===s.id ? 'true' : undefined}>
                      <StatusDot status={s.status} size={5} />
                      <span class="overflow-hidden text-ellipsis whitespace-nowrap flex-1 text-[12px] leading-[1.2] font-mono" style="color:{activeSource===s.id ? T.ink0 : T.ink1};">{s.name}</span>
                      {#if s.unread > 0}<span class="text-cyan text-[10px] leading-none font-mono">{s.unread}</span>{/if}
                    </button>
                  {/each}
                </div>
                <div class="shrink-0 flex items-center gap-1 px-2 pb-1.5 pt-1">
                  <button onclick={() => { showSources = !showSources; }} class="flex items-center gap-1 flex-1 min-w-0 bg-transparent border-none text-ink-3 cursor-pointer text-left px-1.5 py-1 text-[10px] leading-none font-mono">
                    <Icon name="plus" size={10} color={T.ink3} />
                    <span>Manage Sources</span>
                  </button>
                  <button onclick={() => { showOnboarding = true; }} class="flex items-center gap-1 flex-1 min-w-0 bg-transparent border-none text-ink-3 cursor-pointer justify-center px-1.5 py-1 text-[10px] leading-none font-mono" title="Discover feeds" aria-label="Discover feeds">
                    <Icon name="list" size={10} color={T.ink3} />
                    <span>discover</span>
                  </button>
                </div>
              {/if}
            </div>
            <BottomTools {showSources} {showSettings} syncing={syncState.syncing} {syncState} onToggleSources={() => { showSources = !showSources; }} onToggleSettings={() => { showSettings = !showSettings; }} />
          </div>
        {/if}

        <DragHandle edge="left" {onDragStart} {dragging} />

        <!-- Timeline pane -->
        <div class="shrink-0 flex flex-col border-r border-bd-0 overflow-hidden bg-bg-0" style="width:{timelineWidth}px">
          <TimelinePane mode="wide" items={displayItems} {searchQuery} {openId} onOpen={(id, ids) => openItemAndRead(id, ids)} />
        </div>

        <DragHandle edge="timeline" {onDragStart} {dragging} />

        <!-- Reader pane -->
        {#if openItemObj}
          <div class="flex-1 min-w-0 flex flex-col bg-bg-0 overflow-hidden">
            <ReaderPane mode="wide" itemId={openItemObj.id} allIds={timelineIds} onBack={closeReader} onNavigate={(id) => openItemAndRead(id, timelineIds)} />
          </div>
        {/if}

        <!-- Settings modal -->
        <Modal open={showSettings} title="Settings" onClose={() => { showSettings = false; }} width="560px">
          <SettingsPanelContent showShortcuts />
        </Modal>

        <!-- Sources modal -->
        <Modal open={showSources} title="Sources" onClose={() => { showSources = false; }} width="640px">
          <SourceExplorer onSourceSelect={(id) => { setFeedFilter(id); showSources = false; }} compact={true} />
        </Modal>

        <!-- Keyboard shortcut cheatsheet -->
        <Modal open={showCheatsheet} title="Keyboard Shortcuts" onClose={() => showCheatsheet = false} width="520px">
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
  {:else}
    <div class="flex flex-col w-full h-full bg-bg-1 overflow-hidden">
      <div class="shrink-0 h-[var(--sat)]"></div>
      <div class="flex-1 overflow-hidden flex flex-col relative">
        {#if activeTab === 'feed'}
          <TimelinePane mode="narrow" items={items} openId={openId ?? ''} onOpen={openItem} onSearch={goToSearch} />
        {:else if activeTab === 'sources'}
          <div class="flex flex-col h-full bg-bg-0 text-ink-0">
            <div class="h-[44px] flex items-center px-[10px] border-b border-b-bd-0 bg-bg-1 shrink-0 gap-2">
              <span class="text-[12px] leading-none font-mono text-ink-0 tracking-[0.5px] flex-1">sources <span class="text-ink-3">· {sources.length}</span></span>
              <button onclick={doSync} aria-label="Sync feeds" class="inline-flex items-center justify-center bg-transparent border-none cursor-pointer rounded min-h-11 min-w-11">
                <span class={syncState.syncing ? 'syncing' : ''}><Icon name="sync" size={16} color={syncState.syncing ? T.cyan : T.ink1} /></span>
              </button>
              <button onclick={() => { document.querySelector('.add-source-target')?.scrollIntoView({ behavior: 'smooth', block: 'center' }); }} class="inline-flex items-center justify-center bg-transparent border-none cursor-pointer rounded min-h-11 min-w-11" title="Add source" aria-label="Add source">
                <Icon name="plus" size={16} color={T.cyan} />
              </button>
              <button onclick={() => { showOnboarding = true; }} class="bg-transparent border-none cursor-pointer text-ink-2 min-h-11 px-1 text-[10px] leading-none font-mono tracking-[0.4px]" title="Discover feeds" aria-label="Discover feeds">discover</button>
            </div>
            <div class="flex-1 overflow-auto">
              <SourceExplorer onSourceSelect={openSourceFeed} onSync={doSync} />
            </div>
          </div>
        {:else if activeTab === 'search'}
          <SearchView onItemOpen={(id, ids) => openItem(id, ids)} />
        {:else if activeTab === 'saved'}
          <MobileSaved onOpen={openItem} />
        {:else if activeTab === 'settings'}
          <div class="flex flex-col h-full bg-bg-0 text-ink-0">
            <div class="h-[44px] flex items-center px-3.5 border-b border-b-bd-0 bg-bg-1 shrink-0">
              <span class="text-[12px] leading-none font-mono text-ink-0 tracking-[0.5px]">settings</span>
            </div>
            <div class="flex-1 overflow-y-auto flex flex-col gap-2.5 p-3 px-2.5">
              <SettingsPanelContent showShortcuts={false} />
              <div class="h-3"></div>
            </div>
          </div>
        {:else}
          <div class="flex-1 flex items-center justify-center text-ink-3 text-[11px] leading-none font-mono">{activeTab}</div>
        {/if}

        {#if openId}
          <div class="absolute inset-0 z-10 flex flex-col">
            <ReaderPane mode="narrow" itemId={openId} allIds={timelineIds} onBack={goBack} onNavigate={(id) => { openId = id; }} />
          </div>
        {/if}
      </div>
      <PulseBottomNav active={activeTab} onChange={changeTab} />
    </div>
  {/if}

  {#if showOnboarding}
    <Onboarding onDone={finishOnboarding} />
  {/if}

  <ContextMenu open={sourceMenu !== null} mode="popup" x={sourceMenu?.x ?? 0} y={sourceMenu?.y ?? 0} onClose={closeSourceMenu} class="w-48">
    {#each SOURCE_ACTIONS as act}
      <button class="menu-row text-[12px] leading-none" style="color:{act.color};" onclick={() => runSourceAction(act.kind)}>
        <Icon name={act.icon} size={12} color={act.color} />
        <span>{act.label}</span>
      </button>
    {/each}
  </ContextMenu>

  <ItemActionsMenu />
</div>
