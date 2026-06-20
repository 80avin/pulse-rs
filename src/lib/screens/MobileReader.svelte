<script lang="ts">
  import { untrack } from 'svelte';
  import { T } from '$lib/tokens';
  import { createDialog, melt } from '@melt-ui/svelte';
  import { items, sources, markRead, toggleSaved, hideItem } from '$lib/store.svelte';
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

  const noteSheetDialog = createDialog({
    defaultOpen: true,
    preventScroll: false,
    onOpenChange: ({ next }) => { if (!next) noteSheetOpen = false; return next; },
  });
  const { overlay: noteOverlay, content: noteContent, close: noteClose } = noteSheetDialog.elements;

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

  function startSavePress() {
    saveLongPressed = false;
    savePressTimer = setTimeout(() => {
      saveLongPressed = true;
      savePressTimer = null;
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
      e.preventDefault();
    } else {
      toggleSaved(item!.id);
      showSaveToast();
    }
  }

  function showSaveToast() {
    saveToast = true;
    if (saveToastTimer) clearTimeout(saveToastTimer);
    saveToastTimer = setTimeout(() => { saveToast = false; }, 3000);
  }

  function saveWithNote() {
    if (!item) return;
    toggleSaved(item.id, noteDraft.trim() || undefined);
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
  <div style="position:relative;display:flex;flex-direction:column;height:100%;background:{T.bg0};color:{T.ink0};">

    <!-- Top bar -->
    <div style="height:44px;display:flex;align-items:center;padding:0 8px;border-bottom:1px solid {T.bd0};background:{T.bg1};flex-shrink:0;gap:6px;">
      <button
        onclick={onBack}
        style="width:34px;height:34px;display:inline-flex;align-items:center;justify-content:center;background:transparent;border:none;cursor:pointer;border-radius:4px;">
        <Icon name="arrow-l" size={18} color={T.ink1} />
      </button>
      <span style="font:11px/1 {T.mono};color:{T.ink2};flex:1;">
        reader · {idx + 1}<span style="color:{T.ink3};">/{allIds.length}</span>
      </span>
      <button onclick={goPrev} disabled={!hasPrev} style="width:34px;height:34px;display:inline-flex;align-items:center;justify-content:center;background:transparent;border:none;cursor:pointer;border-radius:4px;opacity:{hasPrev ? 1 : 0.3};">
        <Icon name="arrow-up" size={18} color={T.ink1} />
      </button>
      <button onclick={goNext} disabled={!hasNext} style="width:34px;height:34px;display:inline-flex;align-items:center;justify-content:center;background:transparent;border:none;cursor:pointer;border-radius:4px;opacity:{hasNext ? 1 : 0.3};">
        <Icon name="arrow-dn" size={18} color={T.ink1} />
      </button>
    </div>

    <!-- Scrollable body -->
    <div
      style="flex:1;overflow-y:auto;touch-action:pan-y;
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
      <div style="display:flex;align-items:center;justify-content:space-between;padding:8px 12px;border-top:1px solid {T.bd0};background:{T.bg1};font:11px/1 {T.mono};color:{T.ink1};flex-shrink:0;">
        <span>Saved <span style="color:{T.amber};">{source?.name ?? item.src}</span> post</span>
        <button
          onclick={() => { saveToast = false; noteDraft = item?.note ?? ''; noteSheetOpen = true; }}
          style="background:transparent;border:none;cursor:pointer;font:11px/1 {T.mono};color:{T.cyan};padding:2px 6px;"
        >
          add note
        </button>
      </div>
    {/if}

    <!-- Action bar -->
    <div style="display:flex;border-top:1px solid {T.bd1};background:{T.bg1};flex-shrink:0;">
      <button
        onclick={() => markRead(item.id, !item.read)}
        style="flex:1;display:flex;flex-direction:column;align-items:center;gap:4px;padding:10px 0;background:transparent;border:none;color:{item.read ? T.green : T.ink2};cursor:pointer;font:10px/1 {T.mono};letter-spacing:0.4px;min-height:52px;"
      >
        <div style="display:flex;align-items:center;gap:4px;">
          <Icon name="check" size={16} color={item.read ? T.green : T.ink1} />
          <KeyCap k="m" dim />
        </div>
        <span style="text-transform:uppercase;">{item.read ? 'unread' : 'read'}</span>
      </button>
      <button
        onclick={() => { toggleSaved(item.id); showSaveToast(); }}
        ontouchstart={startSavePress}
        ontouchend={endSavePress}
        ontouchcancel={cancelSavePress}
        style="flex:1;display:flex;flex-direction:column;align-items:center;gap:4px;padding:10px 0;background:transparent;border:none;color:{item.saved ? T.amber : T.ink2};cursor:pointer;font:10px/1 {T.mono};letter-spacing:0.4px;min-height:52px;"
      >
        <div style="display:flex;align-items:center;gap:4px;">
          <Icon name="bookmark" size={16} color={item.saved ? T.amber : T.ink1} />
          {#if item.note}<span style="font:10px/1 {T.mono};color:{T.amber};">*</span>{/if}
          <KeyCap k="s" dim />
        </div>
        <span style="text-transform:uppercase;">{item.saved ? 'saved' : 'save'}</span>
      </button>
      <button
        onclick={() => item.url && openExternal(item.url)}
        style="flex:1;display:flex;flex-direction:column;align-items:center;gap:4px;padding:10px 0;background:transparent;border:none;color:{T.ink2};cursor:pointer;font:10px/1 {T.mono};letter-spacing:0.4px;min-height:52px;"
        title={item.domain ? `Open https://${item.domain}` : undefined}
      >
        <div style="display:flex;align-items:center;gap:4px;">
          <Icon name="ext" size={16} color={T.ink1} />
          <KeyCap k="o" dim />
        </div>
        <span style="text-transform:uppercase;">open</span>
      </button>
      <button
        onclick={() => shareItem(item.title, item.url ?? item.externalUrl)}
        style="flex:1;display:flex;flex-direction:column;align-items:center;gap:4px;padding:10px 0;background:transparent;border:none;color:{T.ink2};cursor:pointer;font:10px/1 {T.mono};letter-spacing:0.4px;min-height:52px;"
      >
        <div style="display:flex;align-items:center;gap:4px;">
          <Icon name="share" size={16} color={T.ink1} />
        </div>
        <span style="text-transform:uppercase;">share</span>
      </button>
      <button
        onclick={() => { hideItem(item.id); onBack(); }}
        style="flex:1;display:flex;flex-direction:column;align-items:center;gap:4px;padding:10px 0;background:transparent;border:none;color:{T.red};cursor:pointer;font:10px/1 {T.mono};letter-spacing:0.4px;min-height:52px;"
      >
        <div style="display:flex;align-items:center;gap:4px;">
          <Icon name="eye-off" size={16} color={T.red} />
          <KeyCap k="h" dim />
        </div>
        <span style="text-transform:uppercase;">hide</span>
      </button>
    </div>

    <!-- Note input sheet -->
    {#if noteSheetOpen}
      <div {...$noteOverlay} use:melt={$noteOverlay} style="position:absolute;inset:0;background:rgba(0,0,0,0.55);display:flex;align-items:flex-end;z-index:20;">
        <div {...$noteContent} use:melt={$noteContent} style="width:100%;background:{T.bg2};border-top:1px solid {T.bd1};padding:14px 14px 24px;font:12px/1.4 {T.sans};color:{T.ink0};"
        >
          <div style="display:flex;align-items:center;justify-content:space-between;margin-bottom:10px;">
            <span style="font:10px/1 {T.mono};color:{T.ink3};text-transform:uppercase;letter-spacing:0.5px;">note</span>
            <button use:melt={$noteClose} style="background:transparent;border:none;color:{T.ink2};cursor:pointer;display:flex;">
              <Icon name="x" size={14} />
            </button>
          </div>
          <textarea
            bind:value={noteDraft}
            placeholder="Add a note about this post…"
            style="width:100%;min-height:80px;background:{T.bg0};border:1px solid {T.bd1};border-radius:3px;padding:10px;font:12px/1.5 {T.sans};color:{T.ink0};resize:vertical;box-sizing:border-box;"
          ></textarea>
          <div style="margin-top:12px;display:flex;gap:8px;">
            <button
              use:melt={$noteClose}
              style="flex:1;padding:10px 0;background:transparent;color:{T.ink1};border:1px solid {T.bd2};border-radius:3px;font:11px/1 {T.mono};cursor:pointer;letter-spacing:0.3px;"
            >cancel</button>
            <button
              onclick={saveWithNote}
              style="flex:1;padding:10px 0;background:{T.amber};color:{T.bg0};border:none;border-radius:3px;font:11px/1 {T.mono};cursor:pointer;letter-spacing:0.3px;"
            >save with note</button>
          </div>
        </div>
      </div>
    {/if}
  </div>
{:else}
  <div style="height:100%;display:flex;align-items:center;justify-content:center;color:{T.ink3};font:11px/1 {T.mono};">
    item not found
  </div>
{/if}
