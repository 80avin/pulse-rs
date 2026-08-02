<script lang="ts">
  import { onMount } from 'svelte';
  import { T } from '$lib/tokens';
  import { storeReady } from '$lib/stores/data.svelte';
  import { setFeedFilter, setTagFilter } from '$lib/stores/timeline.svelte';
  import MobileTimeline from './MobileTimeline.svelte';
  import MobileReader from './MobileReader.svelte';
  import MobileSources from './MobileSources.svelte';
  import MobileSearch from './MobileSearch.svelte';
  import MobileSaved from './MobileSaved.svelte';
  import MobileSettings from './MobileSettings.svelte';

  let tab = $state('timeline');
  let openItemId = $state<string | null>(null);
  let timelineIds = $state<string[]>([]);

  // ── History-based back navigation ──────────────────────────────────────────
  // Each navigation pushes a browser history entry so the Android system back
  // button unwinds in-app navigation instead of immediately exiting the app.

  type NavState = { tab: string; openItemId: string | null };

  onMount(() => {
    // Seed the initial history entry so back-from-home exits correctly.
    history.replaceState({ tab: 'timeline', openItemId: null } satisfies NavState, '');

    function handlePop(e: PopStateEvent) {
      const s = e.state as NavState | null;
      tab        = s?.tab        ?? 'timeline';
      openItemId = s?.openItemId ?? null;
    }

    window.addEventListener('popstate', handlePop);
    return () => window.removeEventListener('popstate', handlePop);
  });

  function changeTab(newTab: string) {
    if (newTab === tab && !openItemId) return;
    history.pushState({ tab: newTab, openItemId: null } satisfies NavState, '');
    tab = newTab;
    openItemId = null;
  }

  function handleTagFilter(tag: string) {
    setTagFilter(tag);
    // Navigate to timeline to show the filtered results
    history.pushState({ tab: 'timeline', openItemId: null } satisfies NavState, '');
    tab = 'timeline';
    openItemId = null;
  }

  function openItem(id: string, ids: string[]) {
    history.pushState({ tab, openItemId: id } satisfies NavState, '');
    openItemId = id;
    timelineIds = ids;
  }

  function openSourceFeed(sourceId: string) {
    setFeedFilter(sourceId);
    history.pushState({ tab: 'timeline', openItemId: null } satisfies NavState, '');
    tab = 'timeline';
  }

  // Immediate visual update then sync history — popstate will confirm the state
  function goBack() {
    openItemId = null;
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
<div class="flex flex-col w-full h-full bg-bg-1 overflow-hidden">
  <div class="shrink-0 h-[var(--sat)]"></div>

  <div class="flex-1 overflow-hidden flex flex-col relative">
    {#if storeReady.error}
      <div class="flex-1 flex items-center justify-center text-red text-[11px] leading-none font-mono">
        failed to load data — restart the app
      </div>
    {:else}
      <!-- Tab content — always rendered when its tab is active, stays mounted behind reader for scroll preservation -->
      {#if tab === 'timeline'}
        <MobileTimeline
          {tab}
          onTabChange={changeTab}
          onOpen={openItem}
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
      {:else if tab === 'saved'}
        <MobileSaved {tab} onTabChange={changeTab} onOpen={openItem} />
      {:else if tab === 'settings'}
        <MobileSettings {tab} onTabChange={changeTab} />
      {:else}
        <div class="flex-1 flex items-center justify-center text-ink-3 text-[11px] leading-none font-mono">
          {tab}
        </div>
      {/if}

      <!-- Reader overlays tab content via absolute positioning — tab underneath stays alive -->
      {#if openItemId}
        <div class="absolute inset-0 z-10 flex flex-col">
          <MobileReader
            itemId={openItemId}
            allIds={timelineIds}
            onBack={goBack}
            onNavigate={(id) => { openItemId = id; }}
          />
        </div>
      {/if}
    {/if}
  </div>
</div>
