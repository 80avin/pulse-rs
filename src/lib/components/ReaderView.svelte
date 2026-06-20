<script lang="ts">
  import { T, TAG_COLORS } from '$lib/tokens';
  import { createDialog, melt } from '@melt-ui/svelte';
  import { items, sources, toggleSaved, aiStatus } from '$lib/store.svelte';
  import { openExternal, sanitizeHtml, TAG_EVIDENCE } from '$lib/utils';
  import TagChip from './TagChip.svelte';
  import ScoreBar from './ScoreBar.svelte';
  import Icon from './Icon.svelte';

  let {
    itemId,
    noteMode = 'none',
    onTagClick,
    onPopoverChange,
    showMetadata = true,
    isDesktop = false,
  }: {
    itemId: string;
    noteMode?: 'inline' | 'sheet' | 'none';
    onTagClick?: (tag: string) => void;
    onPopoverChange?: (open: boolean) => void;
    showMetadata?: boolean;
    isDesktop?: boolean;
  } = $props();

  const item = $derived(items.find(i => i.id === itemId));
  const source = $derived(item ? sources.find(s => s.id === item.src) : undefined);

  let popoverTag = $state<string | null>(null);
  let noteEditing = $state(false);
  let noteDraft = $state('');

  const tagPopover = createDialog({
    defaultOpen: true,
    preventScroll: false,
    onOpenChange: ({ next }) => { if (!next) popoverTag = null; return next; },
  });
  const { overlay: popoverOverlay, content: popoverContent, close: popoverClose } = tagPopover.elements;

  $effect(() => {
    onPopoverChange?.(popoverTag !== null);
  });

  $effect(() => {
    if (!item) { noteEditing = false; noteDraft = ''; popoverTag = null; }
  });

  function startNoteEdit() { noteDraft = item?.note ?? ''; noteEditing = true; }

  function saveNote() {
    if (!item) return;
    toggleSaved(item.id, noteDraft.trim() || undefined);
    noteEditing = false;
  }

  const readTimeMin = $derived(
    Math.max(1, Math.round((item?.body || '').split(/\s+/).filter(Boolean).length / 238))
  );



</script>




{#if item}
  <div style="position:relative;display:flex;flex-direction:column;height:100%;background:{T.bg0};color:{T.ink0};">
    <div style="flex:1;overflow-y:auto;padding:20px 28px 32px;">
      {#if showMetadata && source}
        <div style="display:flex;align-items:center;gap:8px;margin-bottom:10px;font:11px/1 {T.mono};color:{T.ink2};">
          <span style="font-weight:600;color:{T.ink1};">{source.name}</span>
          <span style="color:{T.ink3};">·</span>
          <span>{item.author}</span>
          <span style="color:{T.ink3};">·</span>
          <span>{item.age}</span>
          {#if item.score > 0}
            <span style="color:{T.ink3};">·</span>
            <span style="color:{T.amber};">▲{item.score}</span>
          {/if}
          {#if item.n > 0}
            <span style="color:{T.ink3};">·</span>
            <span>{item.n}c</span>
          {/if}
        </div>
      {/if}
      <h1 style="margin:0;font:600 22px/1.25 {T.sans};color:{T.ink0};letter-spacing:-0.3px;max-width:720px;">{item.title}</h1>

      {#if item.url}
        {@const primaryDomain = new URL(item.url).hostname.replace(/^www\./, '')}
        <button onclick={() => openExternal(item.url!)} style="margin-top:8px;display:inline-flex;align-items:center;gap:6px;background:transparent;border:none;cursor:pointer;padding:0;font:11px/1 {T.mono};color:{T.ink2};">
          <Icon name="ext" size={11} color={T.ink3} />
          <span style="text-decoration:underline;text-underline-offset:2px;text-decoration-color:{T.bd2};">{primaryDomain}</span>
        </button>
      {/if}

      {#if item.externalUrl}
        <button onclick={() => openExternal(item.externalUrl!)} style="margin-top:4px;display:block;font:11px/1.4 {T.mono};color:{T.cyan};background:transparent;border:none;cursor:pointer;padding:0;text-align:left;">
          <Icon name="ext" size={11} color={T.cyan} />
          <span style="margin-left:4px;text-decoration:underline;text-underline-offset:2px;">{item.externalUrl}</span>
        </button>
      {/if}

      <div style="margin-top:9px;display:flex;align-items:center;gap:8px;flex-wrap:wrap;">
        {#each item.tags as tag}
          <TagChip {tag} size={10} onclick={() => { onTagClick ? onTagClick(tag) : (popoverTag = tag); }} />
        {/each}
        <span style="flex:1;min-width:12px;"></span>
        <span style="font:10px/1 {T.mono};color:{T.ink2};">signal</span>
        <ScoreBar value={item.aiScore} w={36} />
        {#if item.score > 0}<span style="font:11px/1 {T.mono};color:{T.amber};">▲ {item.score}</span>{/if}
        {#if item.n > 0}<span style="font:11px/1 {T.mono};color:{T.ink1};">{item.n} comments</span>{/if}
      </div>

      {#if noteMode === 'inline'}
        {#if noteEditing}
          <div style="margin-top:14px;padding:10px;background:{T.bg1};border:1px solid {T.bd1};border-radius:3px;">
            <textarea bind:value={noteDraft} placeholder="Add a note about this post…" style="width:100%;min-height:60px;background:transparent;border:none;outline:none;font:12px/1.5 {T.sans};color:{T.ink0};resize:vertical;"></textarea>
            <div style="display:flex;gap:6px;justify-content:flex-end;margin-top:6px;">
              <button onclick={() => { noteEditing = false; }} style="padding:4px 10px;background:transparent;color:{T.ink2};border:1px solid {T.bd2};border-radius:2px;font:10px/1 {T.mono};cursor:pointer;">cancel</button>
              <button onclick={saveNote} style="padding:4px 10px;background:{T.amber};color:{T.bg0};border:none;border-radius:2px;font:10px/1 {T.mono};cursor:pointer;">save note</button>
            </div>
          </div>
        {:else if item.note}
          <div style="margin-top:14px;padding:10px 12px;background:{T.bg1};border:1px solid {T.bd1};border-radius:3px;">
            <div style="display:flex;align-items:center;justify-content:space-between;margin-bottom:4px;">
              <span style="font:10px/1 {T.mono};color:{T.ink3};text-transform:uppercase;letter-spacing:0.4px;">note</span>
              <button onclick={startNoteEdit} style="background:transparent;border:none;cursor:pointer;font:10px/1 {T.mono};color:{T.ink2};padding:2px 6px;">edit</button>
            </div>
            <p style="margin:0;font:12px/1.5 {T.sans};color:{T.ink1};white-space:pre-wrap;">{item.note}</p>
          </div>
        {:else}
          <div style="margin-top:14px;">
            <button onclick={startNoteEdit} style="background:transparent;border:1px dashed {T.bd1};border-radius:3px;padding:6px 10px;cursor:pointer;font:10px/1 {T.mono};color:{T.ink3};">+ add note</button>
          </div>
        {/if}
      {:else if noteMode === 'sheet'}
        {#if item.note}
          <div style="padding:10px 12px;margin-top:14px;background:{T.bg1};border-left:3px solid {T.amber};border-radius:0 3px 3px 0;font:11px/1.5 {T.mono};color:{T.ink1};white-space:pre-wrap;">
            <div style="display:flex;align-items:center;gap:6px;margin-bottom:4px;">
              <Icon name="bookmark" size={11} color={T.amber} />
              <span style="font:10px/1 {T.mono};color:{T.ink3};text-transform:uppercase;letter-spacing:0.4px;">note</span>
            </div>
            {item.note}
          </div>
        {/if}
      {/if}

      <div style="margin-top:22px;font:15px/1.65 {T.sans};color:{T.ink0};max-width:720px;" class="item-body">
        {#if item.bodyHtml}{@html sanitizeHtml(item.bodyHtml)}{:else if item.body}<p style="margin:0;white-space:pre-line;">{item.body}</p>{/if}
        {#if item.url}
          <div style="margin-top:24px;padding-top:16px;border-top:1px solid {T.bd0};display:flex;gap:8px;flex-wrap:wrap;">
            <button onclick={() => openExternal(item.url!)} style="display:inline-flex;align-items:center;gap:8px;background:{T.bg1};border:1px solid {T.bd1};border-radius:3px;padding:10px 16px;cursor:pointer;font:12px/1 {T.mono};color:{T.cyan};"><Icon name="ext" size={13} color={T.cyan} /><span>open post</span></button>
            {#if item.externalUrl}
              <button onclick={() => openExternal(item.externalUrl!)} style="display:inline-flex;align-items:center;gap:8px;background:{T.bg1};border:1px solid {T.bd1};border-radius:3px;padding:10px 16px;cursor:pointer;font:12px/1 {T.mono};color:{T.ink1};"><Icon name="ext" size={13} color={T.ink2} /><span>open link</span></button>
            {/if}
          </div>
        {/if}
      </div>
    </div>

    <div style="padding:4px 14px;border-top:1px solid {T.bd0};background:{T.bg1};font:10px/1 {T.mono};color:{T.ink3};display:flex;align-items:center;gap:12px;flex-shrink:0;">
      <span>~{readTimeMin}min read</span>
      <span style="flex:1;"></span>
      {#if noteMode === 'inline'}<span style="color:{T.green};">● readable view</span>{/if}
    </div>

    <!-- Tag explanation popover -->
    {#if popoverTag}
      {@const c = TAG_COLORS[popoverTag] ?? TAG_COLORS['low-effort']}
      {@const evidence = TAG_EVIDENCE[popoverTag] ?? ['title-token match', 'body-token match']}
      <div {...$popoverOverlay} use:melt={$popoverOverlay} style="position:fixed;inset:0;background:rgba(0,0,0,0.55);display:flex;align-items:{isDesktop ? 'center' : 'flex-end'};justify-content:{isDesktop ? 'center' : 'stretch'};z-index:20;">
        <div {...$popoverContent} use:melt={$popoverContent} style="{isDesktop ? 'width:380px;max-width:90vw;border-radius:8px;' : 'width:100%;border-radius:0;border-top:1px solid {c.bd};'}background:{T.bg2};padding:14px 14px 24px;font:12px/1.4 {T.sans};color:{T.ink0};">
          <div style="display:flex;align-items:center;justify-content:space-between;margin-bottom:10px;">
            <div style="display:flex;align-items:center;gap:8px;">
              <TagChip tag={popoverTag} size={11} />
              <span style="font:10px/1 {T.mono};color:{T.ink3};">tagged by {aiStatus.modelName ?? aiStatus.taggingMode} · {Math.round((item.aiScore ?? 0.8) * 100)}% conf</span>
            </div>
            <button use:melt={$popoverClose} style="background:transparent;border:none;color:{T.ink2};cursor:pointer;display:flex;"><Icon name="x" size={14} /></button>
          </div>
          <div style="color:{T.ink1};margin-bottom:8px;">Why tagged <b style="color:{c.fg};">{popoverTag}</b>:</div>
          <ul style="margin:0;padding:0 0 0 14px;color:{T.ink1};font:12px/1.55 {T.sans};">
            {#each evidence as ev}<li style="margin-bottom:2px;">{ev}</li>{/each}
          </ul>
        </div>
      </div>
    {/if}
  </div>
{:else}
  <div style="height:100%;display:flex;align-items:center;justify-content:center;color:{T.ink3};font:11px/1 {T.mono};">item not found</div>
{/if}
