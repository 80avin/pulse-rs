<script lang="ts">
  import { T } from '$lib/tokens';
  import { sources } from '$lib/stores/data.svelte';
  import { settings } from '$lib/settings.svelte';
  import { searchItems } from '$lib/stores/search.svelte';
  import Icon from './Icon.svelte';
  import ItemRow from './ItemRow.svelte';
  import type { FeedItem } from '$lib/types';

  let {
    onItemOpen,
  }: {
    onItemOpen?: (id: string, ids: string[]) => void;
  } = $props();

  let query = $state('');
  let results = $state<FeedItem[]>([]);
  let searching = $state(false);
  let searchInputEl: HTMLInputElement | null = $state(null);

  $effect(() => {
    const q = query.trim();
    if (q.length < 2) { results = []; searching = false; return; }
    searching = true;
    const timer = setTimeout(async () => { results = await searchItems(q); searching = false; }, 250);
    return () => clearTimeout(timer);
  });
</script>

<div class="flex flex-col h-full bg-bg-0 text-ink-0">
  <div class="shrink-0 border-b border-bd-0 bg-bg-1 p-2 px-2.5">
    <div class="flex items-center gap-2 bg-bg-0 border border-bd-1 rounded p-2 px-2.5">
      <Icon name="search" size={15} color={T.ink3} />
      <input bind:this={searchInputEl} bind:value={query} placeholder="search all items…" autofocus class="flex-1 bg-transparent border-none outline-none text-ink-0 text-[13px] leading-none font-mono" />
      {#if searching}<span class="text-ink-3 text-[10px] leading-none font-mono">…</span>
      {:else if query}<button onclick={() => { query = ''; results = []; searchInputEl?.focus(); }} class="bg-transparent border-none cursor-pointer flex p-0.5"><Icon name="x" size={14} color={T.ink3} /></button>{/if}
    </div>
  </div>
  <div class="flex-1 overflow-y-auto">
    {#if query.trim().length < 2}
      <div class="text-center text-ink-3 py-10 px-5 text-[11px] leading-[1.6] font-mono"><div>search titles, body text, and tags</div><div class="mt-1.5 text-ink-4 text-[10px] leading-none font-mono">type at least 2 characters</div></div>
    {:else if searching}
      <div class="text-center text-ink-3 py-10 px-5 text-[11px] leading-[1.6] font-mono">searching…</div>
    {:else if results.length === 0}
      <div class="text-center text-ink-3 py-10 px-5 text-[11px] leading-[1.6] font-mono">no results for "<span class="text-ink-2">{query}</span>"</div>
    {:else}
      <div class="text-ink-3 border-b border-bd-0 p-1.5 px-3 tracking-[0.6px] text-[9px] leading-none font-mono">{results.length} result{results.length === 1 ? '' : 's'}</div>
      {#each results as item}
        {@const source = sources.find(s => s.id === item.src)}
        <ItemRow {item} {source} isFocused={false} density={settings.density} onclick={() => onItemOpen?.(item.id, results.map(i => i.id))} />
      {/each}
    {/if}
  </div>
</div>
