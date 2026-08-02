<script lang="ts">
  import { onMount } from 'svelte';
  import { T } from '$lib/tokens';
  import { sources, toggleSaved, tauriInvoke, adaptItem } from '$lib/stores/data.svelte';
  import ItemRow from '$lib/components/ItemRow.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import type { FeedItem } from '$lib/types';

  interface BackendItem {
    id: string; sourceId: string; sourceName: string; title: string; url: string;
    body: string; bodyHtml: string | null; externalUrl: string | null; author: string | null;
    publishedAt: string; read: boolean; saved: boolean; hidden: boolean;
    score: number | null; n: number; tags: string[]; signal: number;
    ogImage: string | null; note: string | null; userTags: string[];
  }

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
  let scrollEl: HTMLElement | null = $state(null);

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

  function onScroll() {
    const el = scrollEl;
    if (!el || !cursor || loading) return;
    if (el.scrollHeight - el.scrollTop - el.clientHeight < 300) fetchMore();
  }

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

  <!-- Saved items -->
  <div bind:this={scrollEl} onscroll={onScroll} class="flex-1 overflow-y-auto">
    {#if saved.length === 0}
      <div class="text-center text-ink-3 py-14 px-6 text-[11px] leading-[1.6] font-mono">
        <div>no saved items yet</div>
        <div class="mt-1.5 text-ink-4 text-[10px] leading-none font-mono">tap the bookmark on any item to save it for later</div>
      </div>
    {:else}
      {#each saved as item}
        {@const source = sources.find(s => s.id === item.src)}
        <div class="relative">
          <ItemRow
            {item}
            {source}
            isFocused={false}
            density="normal"
            onclick={() => onOpen(item.id, saved.map(i => i.id))}
          />
          <button
            onclick={() => onUnsave(item)}
            title="Remove from saved"
            aria-label="Remove from saved"
            class="absolute top-1 right-1 bg-transparent border-none cursor-pointer p-1 opacity-70 hover:opacity-100"
          >
            <Icon name="x" size={12} color={T.amber} />
          </button>
        </div>
      {/each}
      {#if cursor}
        <div class="h-9 flex items-center justify-center text-[10px] leading-none font-mono text-ink-3">
          {loading ? 'loading…' : ''}
        </div>
      {/if}
    {/if}
  </div>
</div>
