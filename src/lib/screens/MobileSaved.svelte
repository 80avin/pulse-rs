<script lang="ts">
  import { onMount } from 'svelte';
  import { T } from '$lib/tokens';
  import { toggleSaved, tauriInvoke, adaptItem } from '$lib/stores/data.svelte';
  import TimelineList from '$lib/components/TimelineList.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import type { FeedItem, BackendItem } from '$lib/types';

  
  interface BackendPage {
    items: BackendItem[];
    nextCursor: { publishedAt: number; itemId: string } | null;
    counts: { total: number; unread: number; saved: number; signal: number };
  }

  // The bottom nav is rendered by AppShell; this pane only provides content.
  let { onOpen }: {
    onOpen: (id: string, ids: string[]) => void;
  } = $props();

  let saved = $state<FeedItem[]>([]);
  let cursor = $state<{ publishedAt: number; itemId: string } | null>(null);
  let loading = $state(false);

  async function fetchMore() {
    // Terminal guard: once we've loaded a page and there's no further cursor,
    // stop. (A null cursor means "no more items".)
    if (loading || (!cursor && saved.length > 0)) return;
    loading = true;
    try {
      const page = await tauriInvoke<BackendPage>('get_items_page', {
        groupId: null, feedId: null, tag: null, isRead: null, isSaved: true,
        limit: 100,
        cursor: cursor ? { publishedAt: cursor.publishedAt, itemId: cursor.itemId } : null,
      });
      saved.push(...page.items.map(adaptItem));
      cursor = page.nextCursor ?? null;
    } catch {
      /* ignore transient errors */
    } finally {
      loading = false;
    }
  }

  // Load the first page exactly once on mount. (An $effect here would re-run on
  // every cursor/saved change and re-fetch from the start — the infinite loop.)
  onMount(() => { fetchMore(); });

  async function onUnsave(item: FeedItem) {
    await toggleSaved(item.id);
    saved = saved.filter(i => i.id !== item.id);
  }
</script>

<div class="flex flex-col h-full bg-bg-0 text-ink-0">
  <!-- Header -->
  <div class="shrink-0 border-b border-bd-0 bg-bg-1 px-3.5 flex items-center gap-2.5" style="height:44px;">
    <Icon name="bookmark" size={15} color={T.amber} />
    <span class="text-[12px] leading-none font-mono text-ink-0 tracking-[0.5px]">saved</span>
    <span class="flex-1"></span>
    <span class="text-[10px] leading-none font-mono text-ink-3">{saved.length} saved</span>
  </div>

  <!-- Saved items (virtualized list with scroll-to-load-more; unsave overlay per row) -->
  <TimelineList
    items={saved}
    hasMore={!!cursor}
    onLoadMore={fetchMore}
    emptyMessage={'no saved items yet\ntap the bookmark on any item to save it for later'}
    onItemClick={(id, allIds) => onOpen(id, allIds)}
  >
    {#snippet renderAction(item)}
      <button
        onclick={() => onUnsave(item)}
        title="Remove from saved"
        aria-label="Remove from saved"
        class="absolute top-1 right-1 bg-transparent border-none cursor-pointer p-1 opacity-70 hover:opacity-100"
      >
        <Icon name="x" size={12} color={T.amber} />
      </button>
    {/snippet}
  </TimelineList>
</div>
