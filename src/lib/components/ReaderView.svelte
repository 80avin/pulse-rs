<script lang="ts">
  import { T, TAG_COLORS } from '$lib/tokens';
  import { Dialog } from 'bits-ui';
  import { sources, saveWithNote, toggleSaved, domainOf } from '$lib/stores/data.svelte';
  import { openExternal, sanitizeHtml, TAG_EVIDENCE } from '$lib/utils';
  import TagChip from './TagChip.svelte';
  import Icon from './Icon.svelte';
  import GhostButton from './shared/GhostButton.svelte';
  import { isDesktop } from '$lib/use-is-desktop.svelte';
  import type { FeedItem } from '$lib/types';

  let {
    item,
    noteMode = 'none',
    onTagClick,
    onPopoverChange,
    showMetadata = true,
  }: {
    item: FeedItem;
    noteMode?: 'inline' | 'sheet' | 'none';
    onTagClick?: (tag: string) => void;
    onPopoverChange?: (open: boolean) => void;
    showMetadata?: boolean;
  } = $props();

  // ReaderPane resolves the item (opener's list -> knownItems -> store ->
  // fetch) and passes it in; ReaderView never re-resolves from global state.
  const source = $derived(item ? sources.find(s => s.id === item.src) : undefined);
  // domainOf guards malformed URLs (a bad feed URL used to crash the render).
  const primaryDomain = $derived(item?.url ? domainOf(item.url) : '');

  let popoverTag = $state<string | null>(null);
  let noteEditing = $state(false);
  let noteDraft = $state('');

  $effect(() => {
    onPopoverChange?.(popoverTag !== null);
  });

  $effect(() => {
    if (!item) { noteEditing = false; noteDraft = ''; popoverTag = null; }
  });

  function startNoteEdit() { noteDraft = item?.note ?? ''; noteEditing = true; }

  function saveNote() {
    if (!item) return;
    saveWithNote(item.id, noteDraft);
    noteEditing = false;
  }

  const readTimeMin = $derived(
    Math.max(1, Math.round((item?.body || '').split(/\s+/).filter(Boolean).length / 238))
  );

</script>

{#if item}
  <div class="relative flex flex-col h-full bg-bg-0 text-ink-0">
    <div class="flex-1 overflow-y-auto p-[20px_28px_32px]">
      {#if showMetadata && source}
        <div class="flex items-center gap-2 mb-2.5 text-[11px] leading-none font-mono text-ink-2">
          <span class="font-semibold text-ink-1">{source.name}</span>
          <span class="text-ink-3">·</span>
          <span>{item.author}</span>
          <span class="text-ink-3">·</span>
          <span>{item.age}</span>
          {#if item.score > 0}
            <span class="text-ink-3">·</span>
            <span class="text-amber">▲{item.score}</span>
          {/if}
          {#if item.n > 0}
            <span class="text-ink-3">·</span>
            <span>{item.n}c</span>
          {/if}
        </div>
      {/if}
      <h1 class="text-ink-0 m-0 max-w-180 tracking-[-0.3px] text-[22px] leading-tight font-sans" style="font-weight:600;">{item.title}</h1>

      {#if item.url}
        <GhostButton onclick={() => openExternal(item.url!)} class="inline-flex items-center gap-1.5 p-0 mt-2 text-[11px] leading-none text-ink-2">
          <Icon name="ext" size={11} color={T.ink3} />
          <span class="underline underline-offset-2 decoration-bd-2">{primaryDomain}</span>
        </GhostButton>
      {/if}

      {#if item.externalUrl}
        <GhostButton onclick={() => openExternal(item.externalUrl!)} class="block p-0 text-left mt-1 text-[11px] leading-[1.4] text-cyan">
          <Icon name="ext" size={11} color={T.cyan} />
          <span class="underline ml-1 underline-offset-2">{item.externalUrl}</span>
        </GhostButton>
      {/if}

      <div class="flex items-center gap-2 flex-wrap mt-2.25">
        {#each item.tags as tag}
          <TagChip {tag} size={10} onclick={() => { onTagClick ? onTagClick(tag) : (popoverTag = tag); }} />
        {/each}
        <span class="flex-1 min-w-[12px]"></span>
        {#if item.score > 0}<span class="text-amber text-[11px] leading-none font-mono">▲ {item.score}</span>{/if}
        {#if item.n > 0}<span class="text-ink-1 text-[11px] leading-none font-mono">{item.n} comments</span>{/if}
      </div>

      {#if noteMode === 'inline'}
        {#if noteEditing}
          <div class="bg-bg-1 border border-bd-1 p-2.5 rounded mt-3.5">
            <textarea bind:value={noteDraft} placeholder="Add a note about this post…" class="w-full bg-transparent border-none outline-none min-h-15 resize-y text-[12px] leading-normal font-sans text-ink-0"></textarea>
            <div class="flex gap-1.5 justify-end mt-1.5">
              <button onclick={() => { noteEditing = false; }} class="bg-transparent text-ink-2 border border-bd-2 cursor-pointer p-[4px_10px] rounded-sm text-[10px] leading-none font-mono">cancel</button>
              <button onclick={saveNote} class="bg-amber text-bg-0 border-none cursor-pointer p-[4px_10px] rounded-sm text-[10px] leading-none font-mono">save note</button>
            </div>
          </div>
        {:else if item.note}
          <div class="bg-bg-1 border border-bd-1 px-3 py-2.5 rounded mt-3.5">
            <div class="flex items-center justify-between mb-1">
              <span class="uppercase tracking-[0.4px] text-[10px] leading-none font-mono text-ink-3">note</span>
              <GhostButton onclick={startNoteEdit} class="p-[2px_6px] text-[10px] leading-none text-ink-2">edit</GhostButton>
            </div>
            <p class="m-0 whitespace-pre-wrap text-[12px] leading-normal font-sans text-ink-1">{item.note}</p>
          </div>
        {:else}
          <div class="mt-3.5">
            <button onclick={startNoteEdit} class="bg-transparent border border-dashed border-bd-1 cursor-pointer rounded p-[6px_10px] text-[10px] leading-none font-mono text-ink-3">+ add note</button>
          </div>
        {/if}
      {:else if noteMode === 'sheet'}
        {#if item.note}
          <div class="bg-bg-1 border-l-[3px] border-amber p-[10px_12px] mt-3.5 whitespace-pre-wrap rounded-r rounded-sm text-[11px] leading-normal font-mono text-ink-1">
            <div class="flex items-center gap-1.5 mb-1">
              <Icon name="bookmark" size={11} color={T.amber} />
              <span class="uppercase tracking-[0.4px] text-[10px] leading-none font-mono text-ink-3">note</span>
            </div>
            {item.note}
          </div>
        {/if}
      {/if}

      <div class="item-body mt-5.5 max-w-180 text-[15px] leading-[1.65] font-sans text-ink-0">
        {#if item.bodyHtml}{@html sanitizeHtml(item.bodyHtml)}{:else if item.body}<p class="m-0 whitespace-pre-line">{item.body}</p>{/if}
        {#if item.url}
          <div class="flex gap-2 flex-wrap mt-6 pt-4 border-t border-t-bd-0">
            <button onclick={() => openExternal(item.url!)} class="inline-flex items-center gap-2 bg-bg-1 border border-bd-1 cursor-pointer rounded p-[10px_16px] text-[12px] leading-none font-mono text-cyan"><Icon name="ext" size={13} color={T.cyan} /><span>open post</span></button>
            {#if item.externalUrl}
              <button onclick={() => openExternal(item.externalUrl!)} class="inline-flex items-center gap-2 bg-bg-1 border border-bd-1 cursor-pointer rounded p-[10px_16px] text-[12px] leading-none font-mono text-ink-1"><Icon name="ext" size={13} color={T.ink2} /><span>open link</span></button>
            {/if}
          </div>
        {/if}
      </div>
    </div>

    <div class="bg-bg-1 text-ink-3 flex items-center gap-3 shrink-0 px-3.5 py-1 border-t border-t-bd-0 text-[10px] leading-none font-mono">
      <span>~{readTimeMin}min read</span>
      <span class="flex-1"></span>
      {#if noteMode === 'inline'}<span class="text-green">● readable view</span>{/if}
    </div>

    <!-- Tag explanation popover -->
    {#if popoverTag}
      {@const c = TAG_COLORS[popoverTag] ?? TAG_COLORS['low-effort']}
      {@const evidence = TAG_EVIDENCE[popoverTag] ?? ['title-token match', 'body-token match']}
      <Dialog.Root open={popoverTag !== null} onOpenChange={(open) => { if (!open) popoverTag = null; }}>
        <Dialog.Portal>
          <Dialog.Overlay class="fixed inset-0 bg-black/55 z-20" />
          <Dialog.Content
            preventScroll={false}
            class="bg-bg-2 text-ink-0"
            style="{isDesktop() ? 'position:fixed;left:50%;top:50%;transform:translate(-50%,-50%);width:380px;max-width:90vw;border-radius:8px;' : 'position:fixed;bottom:0;left:0;right:0;width:100%;border-radius:0;'}padding:14px 14px 24px;font:12px/1.4 {T.sans};z-index:20;"
          >
            <div class="flex items-center justify-between mb-2.5">
              <div class="flex items-center gap-2">
                <TagChip tag={popoverTag} size={11} />
                <span class="text-[10px] leading-none font-mono text-ink-3">rule engine</span>
              </div>
              <Dialog.Close class="bg-transparent border-none text-ink-2 cursor-pointer flex"><Icon name="x" size={14} /></Dialog.Close>
            </div>
            <div class="text-ink-1 mb-2">Why tagged <b style="color:{c.fg};">{popoverTag}</b>:</div>
            <ul class="m-0 p-[0_0_0_14px] text-ink-1 text-[12px] leading-[1.55] font-sans">
              {#each evidence as ev}<li class="mb-0.5">{ev}</li>{/each}
            </ul>
          </Dialog.Content>
        </Dialog.Portal>
      </Dialog.Root>
    {/if}
  </div>
{:else}
  <div class="h-full flex items-center justify-center text-ink-3 text-[11px] leading-none font-mono">item not found</div>
{/if}
