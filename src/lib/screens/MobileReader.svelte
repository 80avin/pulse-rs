<script lang="ts">
  import { untrack } from 'svelte';
  import { T } from '$lib/tokens';
  import { Dialog } from 'bits-ui';
  import { items, sources, markRead, toggleSaved, saveWithNote, hideItem } from '$lib/stores/data.svelte';
  import { settings } from '$lib/settings.svelte';
  import { openExternal, shareItem } from '$lib/utils';
  import KeyCap from '$lib/components/KeyCap.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import ReaderView from '$lib/components/ReaderView.svelte';

  let { itemId, allIds, onBack, onNavigate }: {
    itemId: string;
    allIds: string[];
    onBack: () => void;
    onNavigate: (id: string) => void;
  } = $props();

  const item   = $derived(items.find(i => i.id === itemId));
  const source = $derived(item ? sources.find(s => s.id === item.src) : undefined);
  const idx    = $derived(allIds.indexOf(itemId));
  const hasPrev = $derived(idx > 0);
  const hasNext = $derived(idx < allIds.length - 1);

  $effect(() => { if (itemId && settings.markReadOn === 'open') { untrack(() => markRead(itemId)); } });

  function goNext() { if (hasNext) onNavigate(allIds[idx + 1]); }
  function goPrev() { if (hasPrev) onNavigate(allIds[idx - 1]); }

  // Swipe gesture state
  let swipeX = $state(0);
  let swipeTransition = $state(false);
  let swipeStartX = 0;
  let swipeStartY = 0;
  let swipeTracking = false;
  let navDir = $state(0);
  let popoverOpen = $state(false);

  function onSwipeStart(e: TouchEvent) {
    if (noteSheetOpen || popoverOpen) return;
    swipeStartX = e.touches[0].clientX;
    swipeStartY = e.touches[0].clientY;
    swipeTracking = true;
    swipeTransition = false;
  }

  function onSwipeMove(e: TouchEvent) {
    if (!swipeTracking || popoverOpen) return;
    const dx = e.touches[0].clientX - swipeStartX;
    const dy = e.touches[0].clientY - swipeStartY;
    if (Math.abs(dy) > Math.abs(dx) || Math.abs(dx) < 8) return;
    swipeX = dx * 0.5;
  }

  function onSwipeEnd(_e: TouchEvent) {
    if (!swipeTracking) return;
    swipeTracking = false;
    swipeTransition = true;
    const threshold = 60;
    const w = typeof window !== 'undefined' ? window.innerWidth : 400;
    if (swipeX > threshold && hasPrev) {
      swipeX = w;
      setTimeout(() => { goPrev(); swipeX = 0; swipeTransition = false; navDir = -1; }, 180);
    } else if (swipeX < -threshold && hasNext) {
      swipeX = -w;
      setTimeout(() => { goNext(); swipeX = 0; swipeTransition = false; navDir = 1; }, 180);
    } else {
      swipeX = 0;
    }
  }

  $effect(() => {
    if (navDir !== 0) {
      const timer = setTimeout(() => { navDir = 0; }, 250);
      return () => clearTimeout(timer);
    }
  });

  // Note sheet state
  let noteSheetOpen = $state(false);
  let noteDraft = $state('');
  let saveToast = $state(false);
  let saveToastTimer: ReturnType<typeof setTimeout> | null = null;

  // Long-press on save button
  let savePressTimer: ReturnType<typeof setTimeout> | null = null;
  let saveLongPressed = false;
  let suppressNextClick = false;

  function startSavePress() {
    suppressNextClick = false;
    saveLongPressed = false;
    savePressTimer = setTimeout(() => {
      saveLongPressed = true;
      savePressTimer = null;
      suppressNextClick = true;
      noteDraft = item?.note ?? '';
      noteSheetOpen = true;
    }, 450);
  }
  function cancelSavePress() {
    if (savePressTimer) { clearTimeout(savePressTimer); savePressTimer = null; }
    saveLongPressed = false;
  }
  function endSavePress(e: TouchEvent) {
    const wasLong = saveLongPressed;
    cancelSavePress();
    if (wasLong) {
      // Suppress the ghost click that follows — otherwise it fires the button's
      // onclick and double-toggles the save on top of the note-sheet flow.
      suppressNextClick = true;
      e.preventDefault();
    }
  }

  function showSaveToast() {
    saveToast = true;
    if (saveToastTimer) clearTimeout(saveToastTimer);
    saveToastTimer = setTimeout(() => { saveToast = false; }, 3000);
  }

  function submitNote() {
    if (!item) return;
    saveWithNote(item.id, noteDraft);
    noteSheetOpen = false;
  }

  function handleKey(e: KeyboardEvent) {
    if (popoverOpen) return;
    switch (e.key) {
      case 'j': case 'ArrowDown': goNext(); break;
      case 'k': case 'ArrowUp':   goPrev(); break;
      case 'm': if (item) markRead(item.id, !item.read); break;
      case 's': if (item) toggleSaved(item.id); break;
      case 'o': if (item?.url || item?.domain) openExternal(item.url ?? `https://${item.domain}`); break;
      case 'Escape': onBack(); break;
    }
  }

</script>

<svelte:window onkeydown={handleKey} />

{#if item}
  <div class="relative flex flex-col h-full bg-bg-0 text-ink-0">

    <!-- Top bar -->
    <div class="h-11 flex items-center gap-1.5 shrink-0 bg-bg-1 border-b border-bd-0 px-2">
      <button
        onclick={onBack}
        class="inline-flex items-center justify-center bg-transparent border-none cursor-pointer rounded w-8.5 h-8.5">
        <Icon name="arrow-l" size={18} color={T.ink1} />
      </button>
      <span class="text-ink-2 flex-1 text-[11px] leading-none font-mono">
        reader · {idx + 1}<span class="text-ink-3">/{allIds.length}</span>
      </span>
      <button onclick={goPrev} disabled={!hasPrev} class="inline-flex items-center justify-center bg-transparent border-none cursor-pointer rounded w-8.5 h-8.5" style="opacity:{hasPrev ? 1 : 0.3};">
        <Icon name="arrow-up" size={18} color={T.ink1} />
      </button>
      <button onclick={goNext} disabled={!hasNext} class="inline-flex items-center justify-center bg-transparent border-none cursor-pointer rounded w-8.5 h-8.5" style="opacity:{hasNext ? 1 : 0.3};">
        <Icon name="arrow-dn" size={18} color={T.ink1} />
      </button>
    </div>

    <!-- Scrollable body -->
    <div
      class="flex-1 overflow-y-auto touch-pan-y"
      style="
        transform: translateX({navDir ? 0 : swipeX}px);
        transition: transform {swipeTransition && !navDir ? '0.2s ease-out' : 'none'};
        animation: {navDir > 0 ? 'reader-slide-in-next' : navDir < 0 ? 'reader-slide-in-prev' : 'none'} 0.22s ease-out;"
      ontouchstart={onSwipeStart}
      ontouchmove={onSwipeMove}
      ontouchend={onSwipeEnd}
      role="feed"
    >
      <ReaderView itemId={itemId} noteMode="sheet" onPopoverChange={(open) => { popoverOpen = open; }} />
    </div>

    <!-- Save toast -->
    {#if saveToast}
      <div class="flex items-center justify-between bg-bg-1 text-ink-1 shrink-0 p-2 px-3 border-t border-t-bd-0 text-[11px] leading-none font-mono">
        <span>Saved <span class="text-amber">{source?.name ?? item.src}</span> post</span>
        <button
          onclick={() => { saveToast = false; noteDraft = item?.note ?? ''; noteSheetOpen = true; }}
          class="bg-transparent border-none cursor-pointer text-cyan py-0.5 px-1.5 text-[11px] leading-none font-mono"
        >
          add note
        </button>
      </div>
    {/if}

    <!-- Action bar -->
    <div class="flex bg-bg-1 shrink-0 border-t border-t-bd-1">
      <button
        onclick={() => markRead(item.id, !item.read)}
        class="flex-1 flex flex-col items-center bg-transparent border-none cursor-pointer gap-1 py-2.5 tracking-[0.4px] min-h-13 text-[10px] leading-none font-mono" style="color:{item.read ? T.green : T.ink2};"
      >
        <div class="flex items-center gap-1">
          <Icon name="check" size={16} color={item.read ? T.green : T.ink1} />
          <KeyCap k="m" dim />
        </div>
        <span class="uppercase">{item.read ? 'unread' : 'read'}</span>
      </button>
      <button
        onclick={() => {
          if (suppressNextClick) { suppressNextClick = false; return; }
          toggleSaved(item.id);
          showSaveToast();
        }}
        ontouchstart={startSavePress}
        ontouchend={endSavePress}
        ontouchcancel={cancelSavePress}
        class="flex-1 flex flex-col items-center bg-transparent border-none cursor-pointer gap-1 py-2.5 tracking-[0.4px] min-h-13 text-[10px] leading-none font-mono" style="color:{item.saved ? T.amber : T.ink2};"
      >
        <div class="flex items-center gap-1">
          <Icon name="bookmark" size={16} color={item.saved ? T.amber : T.ink1} />
          {#if item.note}<span class="text-amber text-[10px] leading-none font-mono">*</span>{/if}
          <KeyCap k="s" dim />
        </div>
        <span class="uppercase">{item.saved ? 'saved' : 'save'}</span>
      </button>
      <button
        onclick={() => item.url && openExternal(item.url)}
        class="flex-1 flex flex-col items-center bg-transparent border-none cursor-pointer text-ink-2 gap-1 py-2.5 tracking-[0.4px] min-h-13 text-[10px] leading-none font-mono"
        title={item.domain ? `Open https://${item.domain}` : undefined}
      >
        <div class="flex items-center gap-1">
          <Icon name="ext" size={16} color={T.ink1} />
          <KeyCap k="o" dim />
        </div>
        <span class="uppercase">open</span>
      </button>
      <button
        onclick={() => shareItem(item.title, item.url ?? item.externalUrl)}
        class="flex-1 flex flex-col items-center bg-transparent border-none cursor-pointer text-ink-2 gap-1 py-2.5 tracking-[0.4px] min-h-13 text-[10px] leading-none font-mono"
      >
        <div class="flex items-center gap-1">
          <Icon name="share" size={16} color={T.ink1} />
        </div>
        <span class="uppercase">share</span>
      </button>
      <button
        onclick={() => { hideItem(item.id); onBack(); }}
        class="flex-1 flex flex-col items-center bg-transparent border-none cursor-pointer text-red gap-1 py-2.5 tracking-[0.4px] min-h-13 text-[10px] leading-none font-mono"
      >
        <div class="flex items-center gap-1">
          <Icon name="eye-off" size={16} color={T.red} />
          <KeyCap k="h" dim />
        </div>
        <span class="uppercase">hide</span>
      </button>
    </div>

    <!-- Note input sheet -->
      <Dialog.Root open={noteSheetOpen} onOpenChange={(open) => { if (!open) noteSheetOpen = false; }}>
        <Dialog.Portal>
          <Dialog.Overlay class="absolute inset-0 bg-black/55 z-20" />
          <Dialog.Content
            preventScroll={false}
            class="absolute bg-bg-2 text-ink-0 bottom-0 left-0 right-0 w-full p-[14px_14px_24px] z-20 border-t border-t-bd-1 text-[12px] leading-[1.4] font-sans"
          >
            <div class="flex items-center justify-between mb-2.5">
              <span class="uppercase tracking-[0.5px] text-[10px] leading-none font-mono text-ink-3">note</span>
              <Dialog.Close class="bg-transparent border-none text-ink-2 cursor-pointer flex">
                <Icon name="x" size={14} />
              </Dialog.Close>
            </div>
            <textarea
              bind:value={noteDraft}
              placeholder="Add a note about this post…"
              class="w-full box-border min-h-20 rounded p-2.5 resize-y bg-bg-0 border border-bd-1 text-[12px] leading-normal font-sans text-ink-0"
            ></textarea>
            <div class="flex gap-2 mt-3">
              <Dialog.Close class="flex-1 bg-transparent text-ink-1 border border-bd-2 cursor-pointer py-2.5 rounded tracking-[0.3px] text-[11px] leading-none font-mono">cancel</Dialog.Close>
              <button
                onclick={submitNote}
                class="flex-1 bg-amber text-bg-0 border-none cursor-pointer py-2.5 rounded tracking-[0.3px] text-[11px] leading-none font-mono"
              >save with note</button>
            </div>
          </Dialog.Content>
        </Dialog.Portal>
      </Dialog.Root>
  </div>
{:else}
  <div class="h-full flex items-center justify-center text-ink-3 text-[11px] leading-none font-mono">
    item not found
  </div>
{/if}
