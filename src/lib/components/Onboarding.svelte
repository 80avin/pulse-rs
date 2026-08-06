<script lang="ts">
  import { onMount } from 'svelte';
  import { T, SOURCE_KIND } from '$lib/tokens';
  import Icon from '$lib/components/Icon.svelte';
  import GhostButton from '$lib/components/shared/GhostButton.svelte';
  import { tauriInvoke, reloadSources, reloadGroups, reloadDbStats, ageLabel, domainOf } from '$lib/stores/data.svelte';
  import { openExternal } from '$lib/utils';
  import { isDesktop } from '$lib/use-is-desktop.svelte';

  interface PopularFeedDto { name: string; url: string; kind: string; }
  interface PopularCategoryDto { category: string; experimental?: boolean; feeds: PopularFeedDto[]; }
  interface OnboardSelectionDto { name: string; url: string; kind: string; category: string; }
  interface PreviewItemDto { id: string; title: string; url: string | null; author: string | null; publishedAt: string | null; }

  let { onDone, previewUrl, onOpenPreview, onExitPreview }: { onDone: () => void; previewUrl: string | null; onOpenPreview: (url: string) => void; onExitPreview: () => void } = $props();

  let cats = $state<PopularCategoryDto[]>([]);
  let loading = $state(true);
  let error = $state(false);
  let selected = $state<Record<string, boolean>>({});
  let adding = $state(false);
  let addError = $state(false);
  let previewItems = $state<PreviewItemDto[]>([]);
  let previewLoading = $state(false);
  let previewError = $state('');
  const previewFeed = $derived(previewUrl ? cats.flatMap(c => c.feeds).find(f => f.url === previewUrl) ?? null : null);

  $effect(() => {
    if (!previewUrl) {
      previewItems = [];
      previewLoading = false;
      previewError = '';
      return;
    }
    const url = previewUrl;
    previewItems = [];
    previewLoading = true;
    previewError = '';
    const kind = previewFeed?.kind ?? 'rss';
    tauriInvoke<PreviewItemDto[]>('preview_feed', { url, kind, limit: 30 })
      .then((items) => {
        if (previewUrl !== url) return;
        previewItems = items;
        previewLoading = false;
      })
      .catch((e) => {
        if (previewUrl !== url) return;
        previewError = typeof e === 'string' ? e : 'preview request failed';
        previewLoading = false;
      });
  });

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

<div role="dialog" aria-modal="true" aria-label="Discover feeds" class="fixed inset-0 z-50 text-ink-0 {isDesktop() ? 'bg-black/50 flex items-center justify-center p-4' : 'bg-bg-0 flex flex-col'}">
  <div class={isDesktop() ? 'bg-bg-0 border border-bd-1 rounded-lg shadow-[0_16px_48px_rgba(0,0,0,0.7)] w-[760px] max-w-[94vw] h-[85vh] flex flex-col overflow-hidden' : 'flex-1 flex flex-col min-h-0'}>
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
  {:else if previewFeed}
    <div class="flex-1 flex flex-col min-h-0">
      <div class="shrink-0 px-4 pt-3 pb-2.5 border-b border-bd-0">
        <div class="flex items-center gap-2">
          <div class="flex-1 min-w-0">
            <div class="text-[11px] leading-none font-mono text-ink-0 truncate">previewing <span class="text-cyan">{previewFeed.name}</span></div>
            <div class="mt-1.5 text-[9px] leading-none font-mono text-ink-3 truncate">{previewFeed.url}</div>
          </div>
          <GhostButton
            onclick={onExitPreview}
            class="shrink-0 text-[9px] leading-none uppercase tracking-[0.5px]"
            style="color:{T.cyan};"
          >exit preview</GhostButton>
        </div>
        {#if previewLoading}
          <div class="mt-2.5 text-[10px] leading-none font-mono text-ink-3">loading items…</div>
        {:else if previewError}
          <div class="mt-2.5 text-[10px] leading-[1.5] font-mono text-red">couldn't load items — {previewError}</div>
        {/if}
      </div>
      <div class="flex-1 overflow-y-auto">
        {#if !previewLoading && !previewError}
          {#if previewItems.length === 0}
            <div class="px-4 py-4 text-[11px] leading-none font-mono text-ink-3">no items yet</div>
          {:else}
            {#each previewItems as item}
              <button
                onclick={() => item.url && openExternal(item.url)}
                class="block w-full text-left cursor-pointer bg-transparent border-none px-4 py-2.5"
                style="border-bottom:1px solid {T.bd0};"
              >
                <div class="text-[11px] leading-[1.45] font-mono text-ink-0 truncate">{item.title}</div>
                <div class="mt-1.5 flex items-center gap-2 text-[10px] leading-none font-mono text-ink-3">
                  <span class="min-w-0 truncate">{item.author ?? 'unknown'}</span>
                  {#if item.publishedAt}
                    <span class="shrink-0">{ageLabel(item.publishedAt)} ago</span>
                  {/if}
                </div>
                {#if item.url}
                  <div class="mt-1 text-[9px] leading-none font-mono text-ink-2 truncate">{domainOf(item.url)}</div>
                {/if}
              </button>
            {/each}
          {/if}
        {/if}
      </div>
    </div>
  {:else}
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
              <div
                class="flex items-center gap-2 rounded-sm px-2.5 py-2"
                style="background:{selected[f.url] ? 'rgba(78,205,214,0.06)' : T.bg1};border:1px solid {selected[f.url] ? T.cyan : T.bd1};"
              >
                <button
                  onclick={() => toggle(f.url)}
                  aria-pressed={selected[f.url]}
                  class="flex items-center gap-2 flex-1 min-w-0 text-left cursor-pointer rounded-sm bg-transparent border-none p-0 font-mono"
                >
                  <span class="shrink-0 w-3.5 h-3.5 flex items-center justify-center rounded-sm" style="background:{selected[f.url] ? T.cyan : 'transparent'};border:1px solid {selected[f.url] ? T.cyan : T.bd1};">
                    {#if selected[f.url]}<Icon name="check" size={9} color={T.bg0} />{/if}
                  </span>
                  <span class="flex-1 min-w-0 text-[11px] leading-none font-mono text-ink-0 truncate">{f.name}</span>
                </button>
                <span class="shrink-0 text-[9px] leading-none font-mono uppercase tracking-[0.5px]" style="color:{SOURCE_KIND[f.kind]?.color ?? T.ink3};">{f.kind}</span>
                <button
                  onclick={(e) => { e.stopPropagation(); onOpenPreview(f.url); }}
                  title={`preview ${f.name}`}
                  aria-label={`preview ${f.name}`}
                  class="shrink-0 flex items-center justify-center rounded-sm bg-transparent border-none cursor-pointer p-[3px]"
                  style="color:{T.ink2};"
                >
                  <Icon name="eye" size={12} />
                </button>
              </div>
            {/each}
          </div>
        </section>
      {/each}
    </div>

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
</div>
