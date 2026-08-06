<script lang="ts">
  import { T } from '$lib/tokens';
  import { items, sources, markRead, toggleSaved, saveWithNote, hideItem, getItem, knownItems } from '$lib/stores/data.svelte';
  import { settings } from '$lib/settings.svelte';
  import { openExternal, shareItem } from '$lib/utils';
  import { untrack } from 'svelte';
  import { swipeable } from './swipe.svelte';
  import SaveButton from './SaveButton.svelte';
  import NoteSheet from './NoteSheet.svelte';
  import KeyCap from './KeyCap.svelte';
  import Icon from './Icon.svelte';
  import SourceGlyph from './SourceGlyph.svelte';
  import GhostButton from './shared/GhostButton.svelte';
  import IconBtn from './shared/IconBtn.svelte';
  import ReaderView from './ReaderView.svelte';

  // One reader pane for both breakpoints. `mode` switches the chrome (metadata
  // bar + no-nav on wide; top bar + swipe + back on narrow); ReaderView, the
  // action bar, and the note flow are shared.
  let { mode, itemId, list, onBack, onNavigate }: {
    mode: 'wide' | 'narrow';
    itemId: string;
    list: import('$lib/types').FeedItem[];
    onBack?: () => void;
    onNavigate?: (id: string) => void;
  } = $props();

  // The opener hands us the exact list the item was clicked from, so the
  // reader never guesses which cache an item lives in. `allIds` drives
  // prev/next across that same list.
  const allIds = $derived(list.map(i => i.id));
  const storeItem = $derived(list.find(i => i.id === itemId) ?? items.find(i => i.id === itemId) ?? knownItems[itemId]);
  let cachedItem = $state<import('$lib/types').FeedItem | null>(null);
  $effect(() => {
    if (storeItem) cachedItem = storeItem;
    // Never let a stale cached item (from a previously-opened row) stand in
    // for the current itemId: that both hides the real item and blocks the
    // fetch-by-ID fallback below.
    else if (cachedItem && cachedItem.id !== itemId) cachedItem = null;
  });
  // Fetch-by-ID fallback: the reader only sees the paginated `items` cache, so
  // items that aren't in it (e.g. old saved items) would otherwise show
  // "item not found" even though they exist in the DB.
  let fetchedItem = $state<import('$lib/types').FeedItem | null>(null);
  let fetchState = $state<'idle' | 'loading' | 'missing'>('idle');
  $effect(() => {
    if (!itemId) { fetchedItem = null; fetchState = 'idle'; return; }
    if (storeItem || cachedItem) { fetchedItem = null; fetchState = 'idle'; return; }
    let cancelled = false;
    fetchedItem = null;
    fetchState = 'loading';
    getItem(itemId).then((i) => {
      if (cancelled) return;
      fetchedItem = i;
      fetchState = i ? 'idle' : 'missing';
    });
    return () => { cancelled = true; };
  });
  const item = $derived(storeItem ?? cachedItem ?? fetchedItem);
  const source = $derived(item ? sources.find(s => s.id === item.src) : undefined);
  const idx    = $derived(allIds.indexOf(itemId));
  const hasPrev = $derived(idx > 0);
  const hasNext = $derived(idx < allIds.length - 1);

  // Auto mark-read on open (narrow; desktop handles it via the shell)
  $effect(() => { if (mode === 'narrow' && itemId && settings.markReadOn === 'open') { untrack(() => markRead(itemId)); } });

  // Gesture + note-sheet state. Swipe handling lives in the `swipeable` action;
  // it drives the body transform and calls back for navigation. `navDir` only
  // drives the entrance animation here.
  let navDir = $state(0);
  let popoverOpen = $state(false);
  let noteSheetOpen = $state(false);
  let noteSeed = $state('');
  let saveToast = $state(false);
  let saveToastTimer: ReturnType<typeof setTimeout> | null = null;

  function goNext() { if (hasNext) onNavigate?.(allIds[idx + 1]); }
  function goPrev() { if (hasPrev) onNavigate?.(allIds[idx - 1]); }

  function swipeTo(dir: number) {
    if (dir < 0) goPrev(); else goNext();
    navDir = dir;
  }
  $effect(() => {
    if (navDir !== 0) {
      const timer = setTimeout(() => { navDir = 0; }, 250);
      return () => clearTimeout(timer);
    }
  });

  function openNoteSheet(seed: string) {
    noteSeed = seed;
    noteSheetOpen = true;
  }
  function closeNoteSheet() { noteSheetOpen = false; }
  function submitNote(n: string) {
    if (!item) return;
    saveWithNote(item.id, n);
    noteSheetOpen = false;
  }

  function showSaveToast() {
    saveToast = true;
    if (saveToastTimer) clearTimeout(saveToastTimer);
    saveToastTimer = setTimeout(() => { saveToast = false; }, 3000);
  }

  function handleHide() {
    if (!item) return;
    hideItem(item.id);
    if (hasPrev) onNavigate?.(allIds[idx - 1]);
    else onBack?.();
  }

  function handleKey(e: KeyboardEvent) {
    if (popoverOpen) return;
    switch (e.key) {
      case 'j': case 'ArrowDown': goNext(); break;
      case 'k': case 'ArrowUp':   goPrev(); break;
      case 'm': if (item) markRead(item.id, !item.read); break;
      case 's': if (item) toggleSaved(item.id); break;
      case 'o': if (item?.url || item?.domain) openExternal(item.url ?? `https://${item.domain}`); break;
      case 'Escape': onBack?.(); break;
    }
  }
</script>

<svelte:window onkeydown={mode === 'narrow' ? handleKey : undefined} />

{#if item}
  <div class="relative flex flex-col h-full bg-bg-0 text-ink-0">

    {#if mode === 'wide'}
      <div class="border-b border-bd-0 bg-bg-1 flex items-center gap-2 shrink-0 text-ink-2 px-3.5 py-1.5 text-[10px] leading-none font-mono">
        {#if source}
          <SourceGlyph kind={source.kind} />
          <span class="text-ink-1">{source.name}</span>
          <span class="text-ink-3">·</span>
        {/if}
        <span>{item.author}</span>
        <span class="text-ink-3">·</span>
        <span>{item.age}</span>
        {#if item.score > 0}<span class="text-ink-3">·</span><span class="text-amber">▲{item.score}</span>{/if}
        {#if item.n > 0}<span class="text-ink-3">·</span><span class="text-ink-2">{item.n}c</span>{/if}
      </div>
    {:else}
      <div class="h-11 flex items-center gap-1.5 shrink-0 bg-bg-1 border-b border-bd-0 px-2">
        <IconBtn onclick={() => onBack?.()} ariaLabel="Back" name="arrow-l" size={18} color={T.ink1} />
        <span class="text-ink-2 flex-1 text-[11px] leading-none font-mono">
          reader · {idx + 1}<span class="text-ink-3">/{allIds.length}</span>
        </span>
        <IconBtn onclick={goPrev} disabled={!hasPrev} ariaLabel="Previous item" name="arrow-up" size={18} color={T.ink1} style="opacity:{hasPrev ? 1 : 0.3};" />
        <IconBtn onclick={goNext} disabled={!hasNext} ariaLabel="Next item" name="arrow-dn" size={18} color={T.ink1} style="opacity:{hasNext ? 1 : 0.3};" />
      </div>
    {/if}

    <!-- Scrollable body (swipe action owns the touch handlers + transform) -->
    <div
      class="flex-1 overflow-y-auto touch-pan-y"
      style="animation: {navDir > 0 ? 'reader-slide-in-next' : navDir < 0 ? 'reader-slide-in-prev' : 'none'} 0.22s ease-out;"
      use:swipeable={{
        enabled: mode === 'narrow',
        blocked: () => noteSheetOpen || popoverOpen,
        canSwipeLeft: () => hasNext,
        canSwipeRight: () => hasPrev,
        onSwipeLeft: () => swipeTo(1),
        onSwipeRight: () => swipeTo(-1),
      }}
      role="feed"
    >
      <ReaderView item={item} noteMode={mode === 'wide' ? 'inline' : 'sheet'} onPopoverChange={(open) => { popoverOpen = open; }} showMetadata={mode === 'wide' ? false : true} />
    </div>

    {#if mode === 'narrow'}
      {#if saveToast}
        <div class="flex items-center justify-between bg-bg-1 text-ink-1 shrink-0 p-2 px-3 border-t border-t-bd-0 text-[11px] leading-none font-mono">
          <span>Saved <span class="text-amber">{source?.name ?? item.src}</span> post</span>
          <GhostButton onclick={() => { saveToast = false; openNoteSheet(item.note ?? ''); }} class="text-cyan py-0.5 px-1.5 text-[11px] leading-none">add note</GhostButton>
        </div>
      {/if}
    {/if}

    <!-- Action bar -->
    <div class="flex bg-bg-1 shrink-0 border-t border-t-bd-1">
      <GhostButton
        onclick={() => markRead(item.id, !item.read)}
        class="flex-1 flex flex-col items-center gap-1 py-2.5 tracking-[0.4px] min-h-13 text-[10px] leading-none" style="color:{item.read ? T.green : T.ink2};"
      >
        <div class="flex items-center gap-1">
          <Icon name="check" size={16} color={item.read ? T.green : T.ink1} />
          <KeyCap k="m" dim />
        </div>
        <span class="uppercase">{item.read ? 'unread' : 'read'}</span>
      </GhostButton>
      <SaveButton {item} narrow={mode === 'narrow'} onOpenNote={openNoteSheet} showToast={showSaveToast} />
      <GhostButton
        onclick={() => item.url && openExternal(item.url)}
        class="flex-1 flex flex-col items-center text-ink-2 gap-1 py-2.5 tracking-[0.4px] min-h-13 text-[10px] leading-none"
        title={item.domain ? `Open https://${item.domain}` : undefined}
      >
        <div class="flex items-center gap-1">
          <Icon name="ext" size={16} color={T.ink1} />
          <KeyCap k="o" dim />
        </div>
        <span class="uppercase">open</span>
      </GhostButton>
      <GhostButton
        onclick={() => shareItem(item.title, item.url ?? item.externalUrl)}
        class="flex-1 flex flex-col items-center text-ink-2 gap-1 py-2.5 tracking-[0.4px] min-h-13 text-[10px] leading-none"
      >
        <div class="flex items-center gap-1">
          <Icon name="share" size={16} color={T.ink1} />
        </div>
        <span class="uppercase">share</span>
      </GhostButton>
      <GhostButton
        onclick={handleHide}
        class="flex-1 flex flex-col items-center text-red gap-1 py-2.5 tracking-[0.4px] min-h-13 text-[10px] leading-none"
      >
        <div class="flex items-center gap-1">
          <Icon name="eye-off" size={16} color={T.red} />
          <KeyCap k={mode === 'wide' ? 'x' : 'h'} dim />
        </div>
        <span class="uppercase">hide</span>
      </GhostButton>
    </div>

    {#if mode === 'narrow'}
      <!-- Note input sheet -->
      <NoteSheet open={noteSheetOpen} note={noteSeed} onSave={submitNote} onClose={closeNoteSheet} />
    {/if}
  </div>
{:else}
  {#if fetchState === 'missing'}
    <div class="h-full flex flex-col items-center justify-center gap-4">
      <div class="text-ink-3 text-[11px] leading-none font-mono">item not found</div>
      <button onclick={onBack} class="bg-transparent border border-bd-1 text-ink-2 cursor-pointer rounded px-3 py-2 text-[10px] leading-none font-mono">← back to list</button>
    </div>
  {:else}
    <div class="h-full flex items-center justify-center text-ink-3 text-[11px] leading-none font-mono">loading…</div>
  {/if}
{/if}
