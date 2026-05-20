<script lang="ts">
  import { T } from '$lib/tokens';
  import { sources, searchItems } from '$lib/store.svelte';
  import PulseBottomNav from '$lib/components/PulseBottomNav.svelte';
  import ItemRow from '$lib/components/ItemRow.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import type { FeedItem } from '$lib/types';

  let { tab, onTabChange, onOpen }: {
    tab: string;
    onTabChange: (id: string) => void;
    onOpen: (id: string, ids: string[]) => void;
  } = $props();

  let query = $state('');
  let results = $state<FeedItem[]>([]);
  let searching = $state(false);
  let searchInputEl: HTMLInputElement | null = $state(null);

  $effect(() => {
    const q = query.trim();
    if (q.length < 2) {
      results = [];
      searching = false;
      return;
    }
    searching = true;
    const timer = setTimeout(async () => {
      results = await searchItems(q);
      searching = false;
    }, 250);
    return () => clearTimeout(timer);
  });
</script>

<div style="display:flex;flex-direction:column;height:100%;background:{T.bg0};color:{T.ink0};">
  <!-- Search bar -->
  <div style="padding:8px 10px;border-bottom:1px solid {T.bd0};background:{T.bg1};flex-shrink:0;">
    <div style="display:flex;align-items:center;gap:8px;padding:8px 10px;background:{T.bg0};border:1px solid {T.bd1};border-radius:4px;">
      <Icon name="search" size={15} color={T.ink3} />
      <input
        bind:this={searchInputEl}
        bind:value={query}
        placeholder="search all items…"
        autofocus
        style="flex:1;font:13px/1 {T.mono};background:transparent;border:none;outline:none;color:{T.ink0};"
      />
      {#if searching}
        <span style="font:10px/1 {T.mono};color:{T.ink3};">…</span>
      {:else if query}
        <button onclick={() => { query = ''; results = []; searchInputEl?.focus(); }} style="background:transparent;border:none;cursor:pointer;display:flex;padding:2px;">
          <Icon name="x" size={14} color={T.ink3} />
        </button>
      {/if}
    </div>
  </div>

  <!-- Results -->
  <div style="flex:1;overflow-y:auto;">
    {#if query.trim().length < 2}
      <div style="padding:40px 20px;text-align:center;font:11px/1.6 {T.mono};color:{T.ink3};">
        <div>search titles, body text, and tags</div>
        <div style="margin-top:6px;font:10px/1 {T.mono};color:{T.ink4};">type at least 2 characters</div>
      </div>
    {:else if searching}
      <div style="padding:40px 20px;text-align:center;font:11px/1.6 {T.mono};color:{T.ink3};">searching…</div>
    {:else if results.length === 0}
      <div style="padding:40px 20px;text-align:center;font:11px/1.6 {T.mono};color:{T.ink3};">
        no results for "<span style="color:{T.ink2};">{query}</span>"
      </div>
    {:else}
      <div style="padding:6px 12px;font:9px/1 {T.mono};color:{T.ink3};letter-spacing:0.6px;border-bottom:1px solid {T.bd0};">
        {results.length} result{results.length === 1 ? '' : 's'}
      </div>
      {#each results as item}
        {@const source = sources.find(s => s.id === item.src)}
        <ItemRow
          {item}
          {source}
          isFocused={false}
          density="normal"
          onclick={() => onOpen(item.id, results.map(i => i.id))}
        />
      {/each}
    {/if}
  </div>

  <PulseBottomNav active={tab} onChange={onTabChange} />
</div>
