<script lang="ts">
  import { onMount } from 'svelte';
  import { T } from '$lib/tokens';
  import { toggleSaved, tauriInvoke, adaptItem, sources, ageLabel, rememberItem } from '$lib/stores/data.svelte';
  import { settings } from '$lib/settings.svelte';
  import ItemRow from '$lib/components/ItemRow.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import GhostButton from '$lib/components/shared/GhostButton.svelte';
  import type { FeedItem, BackendItem } from '$lib/types';

  interface BackendPage {
    items: BackendItem[];
    nextCursor: { publishedAt: number; itemId: string } | null;
    counts: { total: number; unread: number; saved: number };
  }

  // The bottom nav is rendered by AppShell; this pane only provides content.
  let { onOpen }: {
    onOpen: (id: string, ids: string[], list: FeedItem[]) => void;
  } = $props();

  let saved = $state<FeedItem[]>([]);
  let cursor = $state<{ publishedAt: number; itemId: string } | null>(null);
  let loading = $state(false);

  async function fetchMore() {
    // Stop once we've loaded a page and the cursor is null ("no more items")
    if (loading || (!cursor && saved.length > 0)) return;
    loading = true;
    try {
      const page = await tauriInvoke<BackendPage>('get_items_page', {
        groupId: null, feedId: null, tag: null, isRead: null, isSaved: true,
        limit: 100,
        cursor: cursor ? { publishedAt: cursor.publishedAt, itemId: cursor.itemId } : null,
      });
      const adapted = page.items.map(adaptItem);
      for (const it of adapted) rememberItem(it);
      saved.push(...adapted);
      cursor = page.nextCursor ?? null;
    } catch {
      /* ignore transient errors */
    } finally {
      loading = false;
    }
  }

  // Group by saved-month (backend orders saved_at DESC). An $effect here would
  // re-run on every cursor/saved change and re-fetch from the start.
  onMount(() => { fetchMore(); });

  async function onUnsave(item: FeedItem) {
    await toggleSaved(item.id);
    saved = saved.filter(i => i.id !== item.id);
  }

  const allSavedIds = $derived(saved.map(i => i.id));

  // Group by month SAVED (backend orders saved_at DESC → newest-first groups)
  const groups = $derived.by(() => {
    const out: { key: string; label: string; items: FeedItem[] }[] = [];
    for (const item of saved) {
      const d = item.savedAt ? new Date(item.savedAt) : null;
      const key = d ? `${d.getFullYear()}-${d.getMonth()}` : 'unsaved';
      const label = d
        ? d.toLocaleDateString(undefined, { month: 'long', year: 'numeric' })
        : 'unsaved';
      const last = out[out.length - 1];
      if (last && last.key === key) last.items.push(item);
      else out.push({ key, label, items: [item] });
    }
    return out;
  });

  const density = $derived(settings.density);
</script>

<div class="flex flex-col h-full bg-bg-0 text-ink-0">
  <div class="shrink-0 border-b border-bd-0 bg-bg-1 px-3.5 flex items-center gap-2.5" style="height:44px;">
    <Icon name="bookmark" size={15} color={T.amber} />
    <span class="text-[12px] leading-none font-mono text-ink-0 tracking-[0.5px]">saved</span>
    <span class="flex-1"></span>
    <span class="text-[10px] leading-none font-mono text-ink-3">{saved.length} saved</span>
  </div>

  <div class="flex-1 overflow-y-auto">
    {#if saved.length === 0 && !loading}
      <div class="p-8 text-center whitespace-pre-line text-[11px] leading-[1.6] font-mono text-ink-3">
        no saved items yet
        tap the bookmark on any item to save it for later
      </div>
    {:else}
      {#each groups as g}
        <div class="sticky top-0 z-10 bg-bg-0 border-b border-bd-0 px-3.5 py-1.5">
          <span class="uppercase tracking-[0.6px] text-[10px] leading-none font-mono text-ink-2">{g.label}</span>
        </div>
        {#each g.items as item}
          <div class="relative">
            <ItemRow
              {item}
              source={sources.find(s => s.id === item.src)}
              {density}
              onclick={() => onOpen(item.id, allSavedIds, saved)}
            />
            <span class="absolute right-8 top-1 text-ink-4 text-[8px] leading-none font-mono" style="pointer-events:none;">{item.savedAt ? `saved ${ageLabel(item.savedAt)} ago` : ''}</span>
            <GhostButton
              onclick={() => onUnsave(item)}
              title="Remove from saved"
              ariaLabel="Remove from saved"
              class="absolute top-1 right-1 p-1 opacity-70 hover:opacity-100"
            >
              <Icon name="x" size={12} color={T.amber} />
            </GhostButton>
          </div>
        {/each}
      {/each}
      <div class="h-9 flex items-center justify-center text-[10px] leading-none font-mono text-ink-3">
        {#if loading}
          loading…
        {:else if cursor}
          <GhostButton onclick={fetchMore} class="text-ink-3 text-[10px] leading-none">load more</GhostButton>
        {/if}
      </div>
    {/if}
  </div>
</div>
