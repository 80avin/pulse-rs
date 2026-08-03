<script lang="ts">
  import { T } from '$lib/tokens';
  import { toggleSaved } from '$lib/stores/data.svelte';
  import type { FeedItem } from '$lib/types';
  import Icon from './Icon.svelte';
  import KeyCap from './KeyCap.svelte';

  let { item, narrow, onOpenNote, showToast }: {
    item: FeedItem;
    narrow: boolean;
    onOpenNote: (note: string) => void;
    showToast?: () => void;
  } = $props();

  // Long-press on save button (narrow; harmless on desktop)
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
      onOpenNote(item.note ?? '');
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
      suppressNextClick = true;
      e.preventDefault();
    }
  }
  function handleSaveClick() {
    if (suppressNextClick) { suppressNextClick = false; return; }
    toggleSaved(item.id);
    if (narrow) showToast?.();
  }
</script>

<button
  onclick={handleSaveClick}
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
