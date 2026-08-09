<script lang="ts">
  import { onMount } from 'svelte';
  import { T } from '$lib/tokens';
  import { tauriInvoke, ageLabel, adaptItem, sources } from '$lib/stores/data.svelte';
  import { searchItems } from '$lib/stores/search.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import GhostButton from '$lib/components/shared/GhostButton.svelte';
  import type { BackendItem, FeedItem } from '$lib/types';

  interface OverviewGroup {
    groupId: string;
    groupName: string;
    totalItems: number;
    unreadCount: number;
    items: BackendItem[];
  }

  let { onOpenItem, onOpenGroup, onExit }: {
    onOpenItem: (id: string, ids: string[], list: FeedItem[]) => void;
    onOpenGroup: (groupId: string) => void;
    onExit?: () => void;
  } = $props();

  type SearchSort = 'relevance' | 'newest' | 'oldest';
  const SORTS = ['relevance', 'newest', 'oldest'] as const;

  let query = $state('');
  let results = $state<FeedItem[]>([]);
  let searching = $state(false);
  let sort = $state<SearchSort>('relevance');
  let searchInputEl: HTMLInputElement | null = $state(null);

  let groups = $state<OverviewGroup[]>([]);
  let loading = $state(true);
  let error = $state(false);

  $effect(() => {
    const q = query.trim();
    if (q.length < 2) { results = []; searching = false; return; }
    searching = true;
    const currentSort = sort; // reading sort synchronously to trigger svelte dep tracking
    const timer = setTimeout(async () => { results = await searchItems(q, 100, currentSort); searching = false; }, 250);
    return () => clearTimeout(timer);
  });

  async function load() {
    loading = true;
    error = false;
    try {
      groups = await tauriInvoke<OverviewGroup[]>('get_overview', { limit: 8 });
    } catch {
      error = true;
    } finally {
      loading = false;
    }
  }

  onMount(load);
</script>

<div class="flex flex-col w-full h-full bg-bg-0 text-ink-0 overflow-hidden">
  <div class="shrink-0 border-b border-bd-0 bg-bg-1 p-2 px-2.5">
    <div class="flex items-center gap-2 bg-bg-0 border border-bd-1 rounded p-2 px-2.5">
      <Icon name="search" size={15} color={T.ink3} />
      <input bind:this={searchInputEl} bind:value={query} placeholder="search all items…" aria-label="Search items" class="flex-1 bg-transparent border-none outline-none text-ink-0 text-[13px] leading-none font-mono" />
      {#if searching}<span class="text-ink-3 text-[10px] leading-none font-mono">…</span>
      {:else if query}<GhostButton onclick={() => { query = ''; results = []; searchInputEl?.focus(); }} ariaLabel="Clear search" class="flex p-0.5"><Icon name="x" size={14} color={T.ink3} /></GhostButton>{/if}
    </div>
  </div>

  <div class="flex-1 overflow-y-auto">
    {#if query.trim().length >= 2}
      {#if searching}
        <div class="text-center text-ink-3 py-10 px-5 text-[11px] leading-[1.6] font-mono">searching…</div>
      {:else if results.length === 0}
        <div class="text-center text-ink-3 py-10 px-5 text-[11px] leading-[1.6] font-mono">no results for "<span class="text-ink-2">{query}</span>"</div>
      {:else}
        <div class="flex items-center gap-2 border-b border-bd-0 p-1.5 px-3">
          <span class="flex-1 text-ink-3 tracking-[0.6px] text-[9px] leading-none font-mono">{results.length} result{results.length === 1 ? '' : 's'}</span>
          <div class="flex items-center gap-0.75" role="group" aria-label="Search sort order">
            {#each SORTS as opt}
              {@const isActive = sort === opt}
              <button
                onclick={() => { sort = opt; }}
                aria-pressed={isActive}
                class={'cursor-pointer bg-transparent p-[3px_8px] rounded-[3px] text-[10px] leading-none font-mono tracking-[0.3px] ' + (isActive ? 'text-cyan' : 'text-ink-2')}
                style="border:1px solid {isActive ? T.cyan : T.bd1};background:{isActive ? 'rgba(78,205,214,0.10)' : 'transparent'};"
              >{opt}</button>
            {/each}
          </div>
        </div>
        {#each results as item}
          {@const source = sources.find(s => s.id === item.src)}
          <button
            onclick={() => onOpenItem(item.id, results.map(i => i.id), results)}
            class="flex items-center gap-1.5 w-full text-left cursor-pointer border-0 border-b border-bd-0 bg-transparent px-3 py-2 hover:bg-bg-2"
            style="border-left:2px solid {item.read ? 'transparent' : T.cyanDim};"
          >
            <span class="flex-1 min-w-0 truncate text-[12px] leading-[1.4] font-mono" style="color:{item.read ? T.ink2 : T.ink0};">{item.title}</span>
            <span class="shrink-0 text-[10px] leading-none font-mono truncate" style="max-width:90px;color:{T.ink3};">{source?.name ?? ''}</span>
            <span class="shrink-0 text-[10px] leading-none font-mono" style="color:{T.ink2};">{item.age}</span>
          </button>
        {/each}
      {/if}
    {:else}
      {#if onExit}
        <div class="shrink-0 flex items-center gap-2.5 px-3 py-2 border-b border-bd-0 bg-bg-1 text-[14px] leading-none font-mono">
          <span class="uppercase tracking-[0.6px] text-ink-3">overview</span>
          <span class="flex-1"></span>
          <GhostButton onclick={onExit} class="shrink-0 text-[12px] leading-none uppercase tracking-[0.5px]" style="color:{T.cyan};">timeline</GhostButton>
        </div>
      {/if}

      {#if loading}
        <div class="flex-1 flex items-center justify-center text-ink-3 text-[11px] leading-none font-mono">loading overview…</div>
      {:else if error}
        <div class="flex-1 flex flex-col items-center justify-center gap-3 p-6">
          <div class="text-red text-[11px] leading-[1.6] font-mono text-center">couldn't load the overview.<br />check your connection and try again.</div>
          <button onclick={load} class="bg-bg-1 border border-bd-1 text-ink-1 cursor-pointer rounded p-2.5 px-4 text-[11px] leading-none font-mono">retry</button>
        </div>
      {:else if groups.length === 0}
        <div class="flex-1 flex items-center justify-center text-ink-3 text-[11px] leading-[1.6] font-mono text-center px-6">no feeds with items yet — add a feed or sync</div>
      {:else}
        <div class="mx-auto w-full max-w-[1440px] px-6 py-5">
          <div class="grid gap-4 grid-cols-1 md:grid-cols-2 xl:grid-cols-3 content-start">
            {#each groups as g (g.groupId)}
              {@const safe = g.items.filter(i => i && i.id && i.title)}
              {@const list = safe.map(adaptItem)}
              {@const ids = list.map(i => i.id)}
              <div class="flex flex-col border rounded-md overflow-hidden" style="background:{T.bg2};border:1px solid {T.bd1};box-shadow:0 2px 8px rgba(0,0,0,0.35);min-height:160px;">
                <div class="flex items-center gap-2 px-3 pt-2.5 pb-2 border-b" style="background:rgba(78,205,214,0.08);border-color:{T.bd1};">
                  <button
                    onclick={() => onOpenGroup(g.groupId)}
                    class="flex-1 min-w-0 text-left cursor-pointer bg-transparent border-none p-0 font-mono text-[13px] leading-none font-semibold truncate uppercase tracking-[0.5px]"
                    style="color:{T.ink0};"
                  >{g.groupName}</button>
                  <span class="shrink-0 text-[10px] leading-none font-mono" style="color:{T.ink3};">{g.totalItems} total · {g.unreadCount} unread</span>
                </div>
                <div class="flex-1 min-h-0">
                  {#each safe as b, i (b.id)}
                    <button
                      onclick={() => onOpenItem(b.id, ids, list)}
                      class="flex items-center gap-1.5 w-full text-left cursor-pointer border-0 border-b border-bd-0 bg-transparent px-3 py-2 hover:bg-bg-3"
                      style="border-left:2px solid {b.read ? 'transparent' : T.cyan};"
                    >
                      <span class="flex-1 min-w-0 truncate text-[12px] leading-[1.4] font-mono" style="color:{b.read ? T.ink2 : T.ink0};">{b.title}</span>
                      <span class="shrink-0 text-[10px] leading-none font-mono truncate" style="max-width:90px;color:{T.ink3};">{b.sourceName}</span>
                      <span class="shrink-0 text-[10px] leading-none font-mono" style="color:{T.ink2};">{ageLabel(b.publishedAt)}</span>
                    </button>
                  {/each}
                </div>
                <div class="shrink-0 px-1.5 py-1">
                  <GhostButton onclick={() => onOpenGroup(g.groupId)} class="w-full flex items-center justify-center text-[12px] leading-none" style="border:1px solid {T.bd1};border-radius:4px;padding:4px 0;color:{T.ink1};">more →</GhostButton>
                </div>
              </div>
            {/each}
          </div>
        </div>
      {/if}
    {/if}
  </div>
</div>
