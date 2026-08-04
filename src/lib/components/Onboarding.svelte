<script lang="ts">
  import { onMount } from 'svelte';
  import { T, SOURCE_KIND } from '$lib/tokens';
  import Icon from '$lib/components/Icon.svelte';
  import GhostButton from '$lib/components/shared/GhostButton.svelte';
  import { tauriInvoke, reloadSources, reloadGroups, reloadDbStats } from '$lib/stores/data.svelte';

  interface PopularFeedDto { name: string; url: string; kind: string; }
  interface PopularCategoryDto { category: string; experimental?: boolean; feeds: PopularFeedDto[]; }
  interface OnboardSelectionDto { name: string; url: string; kind: string; category: string; }

  let { onDone }: { onDone: () => void } = $props();

  let cats = $state<PopularCategoryDto[]>([]);
  let loading = $state(true);
  let error = $state(false);
  let selected = $state<Record<string, boolean>>({});
  let adding = $state(false);
  let addError = $state(false);

  async function loadCatalog() {
    loading = true;
    error = false;
    try {
      cats = await tauriInvoke<PopularCategoryDto[]>('get_popular_feeds');
    } catch {
      error = true;
    } finally {
      loading = false;
    }
  }

  onMount(loadCatalog);

  const selectedCount = $derived(Object.values(selected).filter(Boolean).length);

  function toggle(url: string) { selected[url] = !selected[url]; }

  function catAllSelected(cat: PopularCategoryDto): boolean {
    return cat.feeds.length > 0 && cat.feeds.every(f => selected[f.url]);
  }
  function toggleCategory(cat: PopularCategoryDto) {
    const all = catAllSelected(cat);
    for (const f of cat.feeds) selected[f.url] = !all;
  }

  async function handleAdd() {
    if (adding || selectedCount === 0) return;
    adding = true;
    addError = false;
    try {
      const selections: OnboardSelectionDto[] = [];
      for (const cat of cats) {
        for (const f of cat.feeds) {
          if (selected[f.url]) selections.push({ name: f.name, url: f.url, kind: f.kind, category: cat.category });
        }
      }
      await tauriInvoke<number>('add_onboard_feeds', { selections });
      await Promise.all([reloadSources(), reloadGroups(), reloadDbStats()]);
      onDone();
    } catch {
      addError = true;
    } finally {
      adding = false;
    }
  }
</script>

<div role="dialog" aria-modal="true" aria-label="Discover feeds" class="absolute inset-0 z-50 bg-bg-0 flex flex-col text-ink-0">
  <!-- Header -->
  <div class="shrink-0 px-4 pt-4 pb-2.5 border-b border-bd-0">
    <div class="text-[13px] leading-none font-mono font-semibold tracking-[0.3px] text-ink-0">welcome to <span class="text-cyan">pulse</span></div>
    <div class="mt-2 text-[10px] leading-[1.5] font-mono text-ink-3">a curated, dev-focused feed reader. pick the feeds you care about — we'll group them into categories for you.</div>
  </div>

  {#if loading}
    <div class="flex-1 flex items-center justify-center text-ink-3 text-[11px] leading-none font-mono">loading catalog…</div>
  {:else if error}
    <div class="flex-1 flex flex-col items-center justify-center gap-3 p-6">
      <div class="text-red text-[11px] leading-[1.6] font-mono text-center">couldn't load the feed catalog.<br />check your connection and try again.</div>
      <button onclick={loadCatalog} class="bg-bg-1 border border-bd-1 text-ink-1 cursor-pointer rounded p-2.5 px-4 text-[11px] leading-none font-mono">retry</button>
      <GhostButton onclick={onDone} class="text-ink-3 text-[10px] leading-none">skip for now</GhostButton>
    </div>
  {:else}
    <!-- Categories -->
    <div class="flex-1 overflow-y-auto">
      {#each cats as cat}
        <section class="border-b border-bd-0">
          <div class="flex items-center gap-2 px-4 py-2 sticky top-0 bg-bg-0">
            <span class="flex-1 min-w-0 flex items-center gap-2">
              <span class="uppercase tracking-[0.6px] text-[10px] leading-none font-mono text-ink-3 truncate">{cat.category}</span>
              {#if cat.experimental}
                <span class="shrink-0 inline-flex items-center rounded-sm border border-dashed border-bd-1 px-[4px] py-[2px] text-[9px] leading-none font-mono uppercase tracking-[0.5px] text-ink-2" title="Experimental category — feed quality may vary">experimental</span>
              {/if}
            </span>
            <GhostButton
              onclick={() => toggleCategory(cat)}
              class="text-[9px] leading-none uppercase tracking-[0.5px]"
              style="color:{catAllSelected(cat) ? T.cyan : T.ink3};"
            >{catAllSelected(cat) ? 'clear' : 'select all'}</GhostButton>
          </div>
          <div class="grid grid-cols-1 gap-1.5 px-3 pb-3 pt-1 sm:grid-cols-2">
            {#each cat.feeds as f}
              <button
                onclick={() => toggle(f.url)}
                aria-pressed={selected[f.url]}
                class="flex items-center gap-2 text-left cursor-pointer rounded-sm px-2.5 py-2"
                style="background:{selected[f.url] ? 'rgba(78,205,214,0.06)' : T.bg1};border:1px solid {selected[f.url] ? T.cyan : T.bd1};"
              >
                <span class="shrink-0 w-3.5 h-3.5 flex items-center justify-center rounded-sm" style="background:{selected[f.url] ? T.cyan : 'transparent'};border:1px solid {selected[f.url] ? T.cyan : T.bd1};">
                  {#if selected[f.url]}<Icon name="check" size={9} color={T.bg0} />{/if}
                </span>
                <span class="flex-1 min-w-0 text-[11px] leading-none font-mono text-ink-0 truncate">{f.name}</span>
                <span class="shrink-0 text-[9px] leading-none font-mono uppercase tracking-[0.5px]" style="color:{SOURCE_KIND[f.kind]?.color ?? T.ink3};">{f.kind}</span>
              </button>
            {/each}
          </div>
        </section>
      {/each}
    </div>

    <!-- Footer -->
    <div class="shrink-0 border-t border-bd-0 px-4 flex items-center gap-2" style="padding-top:12px;padding-bottom:max(12px,env(safe-area-inset-bottom));">
      {#if addError}
        <span class="text-red text-[10px] leading-none font-mono">couldn't add feeds — try again.</span>
      {/if}
      <button
        onclick={onDone}
        class="bg-transparent border border-bd-1 text-ink-2 cursor-pointer rounded px-4 py-3 text-[12px] leading-none font-mono"
      >skip</button>
      <button
        onclick={handleAdd}
        disabled={adding || selectedCount === 0}
        class="flex-1 bg-cyan border-none text-bg-0 font-semibold cursor-pointer rounded px-4 py-3 text-[12px] leading-none font-mono"
        style="opacity:{adding || selectedCount === 0 ? '0.5' : '1'};"
      >{adding ? 'adding…' : `add ${selectedCount} feed${selectedCount === 1 ? '' : 's'}`}</button>
    </div>
  {/if}
</div>
