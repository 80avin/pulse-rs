<script lang="ts">
  import { onMount } from 'svelte';
  import { T } from '$lib/tokens';
  import { storeReady } from '$lib/store.svelte';
  import MobileTimeline from './MobileTimeline.svelte';
  import MobileReader from './MobileReader.svelte';
  import MobileSources from './MobileSources.svelte';
  import MobileSearch from './MobileSearch.svelte';
  import MobileSettings from './MobileSettings.svelte';
  import MobileAI from './MobileAI.svelte';

  let tab = $state('timeline');
  let openItemId = $state<string | null>(null);
  let filterSource = $state<string | null>(null);
  let timelineIds = $state<string[]>([]);
  let activeTag = $state<string | null>(null);

  // ── History-based back navigation ──────────────────────────────────────────
  // Each navigation pushes a browser history entry so the Android system back
  // button unwinds in-app navigation instead of immediately exiting the app.

  type NavState = { tab: string; openItemId: string | null; filterSource: string | null };

  onMount(() => {
    // Seed the initial history entry so back-from-home exits correctly.
    history.replaceState({ tab: 'timeline', openItemId: null, filterSource: null } satisfies NavState, '');

    function handlePop(e: PopStateEvent) {
      const s = e.state as NavState | null;
      tab          = s?.tab          ?? 'timeline';
      openItemId   = s?.openItemId   ?? null;
      filterSource = s?.filterSource ?? null;
    }

    window.addEventListener('popstate', handlePop);
    return () => window.removeEventListener('popstate', handlePop);
  });

  function changeTab(newTab: string) {
    if (newTab === tab && !openItemId && !filterSource) return;
    history.pushState({ tab: newTab, openItemId: null, filterSource: null } satisfies NavState, '');
    tab = newTab;
    openItemId = null;
    filterSource = null;
  }

  function handleTagFilter(tag: string) {
    activeTag = activeTag === tag ? null : tag;
    // Navigate to timeline to show the filtered results
    history.pushState({ tab: 'timeline', openItemId: null, filterSource: null } satisfies NavState, '');
    tab = 'timeline';
    openItemId = null;
    filterSource = null;
  }

  function openItem(id: string, ids: string[]) {
    history.pushState({ tab, openItemId: id, filterSource } satisfies NavState, '');
    openItemId = id;
    timelineIds = ids;
  }

  function openSourceFeed(sourceId: string) {
    history.pushState({ tab: 'timeline', openItemId: null, filterSource: sourceId } satisfies NavState, '');
    filterSource = sourceId;
    tab = 'timeline';
  }

  // Immediate visual update then sync history — popstate will confirm the state
  function goBack() {
    openItemId = null;
    history.back();
  }

  // Clear source filter by unwinding to the screen that set it (usually Sources)
  function clearSourceFilter() {
    filterSource = null;
    history.back();
  }
</script>

<!--
  Two-layer wrapper:
    1. Outer (bg1): fills entire viewport; the bg1 colour shows through the
       translucent Android status bar so it blends with the app chrome.
    2. Status-bar spacer: exactly env(safe-area-inset-top) tall — no content
       drawn here, so nothing is hidden under the system status bar.
    3. Inner (flex:1): every mobile screen fills this region.
-->
<div style="display:flex;flex-direction:column;width:100%;height:100%;background:{T.bg1};overflow:hidden;">
  <div style="height:var(--sat);flex-shrink:0;"></div>

  <div style="flex:1;overflow:hidden;display:flex;flex-direction:column;">
    {#if storeReady.error}
      <div style="flex:1;display:flex;align-items:center;justify-content:center;font:11px/1 {T.mono};color:{T.red};">
        failed to load data — restart the app
      </div>
    {:else if openItemId}
      <MobileReader
        itemId={openItemId}
        allIds={timelineIds}
        onBack={goBack}
        onNavigate={(id) => { openItemId = id; }}
      />
    {:else if tab === 'timeline'}
      <MobileTimeline
        {tab}
        {filterSource}
        {activeTag}
        onTabChange={changeTab}
        onOpen={openItem}
        onClearSourceFilter={clearSourceFilter}
        onClearTagFilter={() => { activeTag = null; }}
        onTagFilter={handleTagFilter}
      />
    {:else if tab === 'sources'}
      <MobileSources
        {tab}
        onTabChange={changeTab}
        onSourceSelect={openSourceFeed}
      />
    {:else if tab === 'search'}
      <MobileSearch
        {tab}
        onTabChange={changeTab}
        onOpen={openItem}
      />
    {:else if tab === 'ai'}
      <MobileAI {tab} onTabChange={changeTab} onTagFilter={handleTagFilter} />
    {:else if tab === 'settings'}
      <MobileSettings {tab} onTabChange={changeTab} />
    {:else}
      <div style="flex:1;display:flex;align-items:center;justify-content:center;color:{T.ink3};font:11px/1 {T.mono};">
        {tab}
      </div>
    {/if}
  </div>
</div>
