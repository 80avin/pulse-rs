<script lang="ts">
  import { T } from '$lib/tokens';
  import { Dialog } from 'bits-ui';
  import { sources, groups, items, markSourceRead, removeSource as storeRemoveSource, syncSource as storeSyncSource } from '$lib/stores/data.svelte';
  import StatusDot from '$lib/components/StatusDot.svelte';
  import SourceGlyph from '$lib/components/SourceGlyph.svelte';
  import Sparkline from '$lib/components/Sparkline.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import SourceForm, { type SourceFormValues } from '$lib/components/SourceForm.svelte';
  import { longpress } from '$lib/components/longpress.svelte';

  let {
    onSourceSelect,
    onSync = () => {},
    compact = false,
    isDesktop = false,
  }: {
    onSourceSelect: (sourceId: string) => void;
    onSync?: () => void;
    compact?: boolean;
    isDesktop?: boolean;
  } = $props();

  const byGroup = $derived.by(() => {
    const grouped: { group: string; name: string; sources: typeof sources }[] = [];
    for (const g of groups) {
      const list = sources.filter(s => s.group === g.id);
      if (list.length > 0) {
        grouped.push({ group: g.id, name: g.name, sources: list });
      }
    }
    return grouped;
  });

  const okCount    = $derived(sources.filter(s => s.status === 'ok').length);
  const staleCount = $derived(sources.filter(s => s.status === 'stale').length);
  const errCount   = $derived(sources.filter(s => s.status === 'error').length);

  function sparkData(sourceId: string): number[] {
    const buckets = new Array<number>(14).fill(0);
    for (const item of items) {
      if (item.src !== sourceId) continue;
      const a = item.age;
      let daysAgo = 0;
      if (a.endsWith('d'))      daysAgo = parseInt(a);
      else if (a.endsWith('h')) daysAgo = 0;
      const bucket = Math.min(13, Math.max(0, daysAgo));
      buckets[13 - bucket]++;
    }
    return buckets;
  }

  let actionSheet = $state<string | null>(null);
  let suppressClick = false;
  let ctxMenuPos = $state<{ x: number; y: number } | null>(null);

  let editingSourceId = $state<string | null>(null);

  function openEditSheet(id: string) {
    const s = sources.find(s => s.id === id);
    if (!s) return;
    editingSourceId = id;
    actionSheet = null;
  }

  function closeEdit() {
    editingSourceId = null;
  }

  function handleContextMenu(e: MouseEvent, sourceId: string) {
    e.preventDefault();
    ctxMenuPos = { x: e.clientX, y: e.clientY };
    actionSheet = sourceId;
  }

  const actionSource = $derived(actionSheet ? sources.find(s => s.id === actionSheet) : null);

  const addInitial = { name: '', url: '', kind: 'rss' as const, group: groups[0]?.id ?? '', hue: undefined as number | undefined };

  let lastEditInitial: SourceFormValues = { id: '', name: '', url: '', kind: 'rss', group: '', hue: undefined };
  const editInitial = $derived.by(() => {
    if (editingSourceId) {
      const s = sources.find(s => s.id === editingSourceId);
      if (s) {
        lastEditInitial = { id: s.id, name: s.name, url: s.url ?? s.host ?? '', kind: s.kind, group: s.group, hue: s.hue };
        return lastEditInitial;
      }
    }
    return lastEditInitial;
  });

  async function removeSource(id: string) {
    await storeRemoveSource(id);
  }

  const sourceActions = [
    { icon: 'list',  label: 'View feed',     action: () => { onSourceSelect(actionSheet!); actionSheet = null; } },
    { icon: 'sync',  label: 'Refresh now',   action: () => { storeSyncSource(actionSheet!); actionSheet = null; } },
    { icon: 'edit',  label: 'Edit source',   action: () => openEditSheet(actionSheet!) },
    { icon: 'star',  label: 'Mark all read', action: () => { markSourceRead(actionSheet!); actionSheet = null; } },
    { icon: 'trash', label: 'Remove source', action: () => { removeSource(actionSheet!); actionSheet = null; } },
  ];
</script>

<div class="flex flex-col flex-1 min-h-0 bg-bg-0 text-ink-0">
  <!-- Status summary -->
  {#if !compact}
    <div class="flex gap-3 py-2 px-3 border-b border-bd-0 bg-bg-1 text-ink-2 shrink-0 text-[10px] leading-none font-mono">
      <span><span class="text-green">● </span>ok {okCount}</span>
      <span><span class="text-amber">● </span>stale {staleCount}</span>
      <span><span class="text-red">● </span>err {errCount}</span>
      <span class="flex-1"></span>
      <span class="text-ink-3">{sources.length} feeds</span>
    </div>
  {/if}

  <div class="flex-1 overflow-y-auto">
    <!-- Add source card -->
    <SourceForm mode="add" initial={addInitial} groups={groups} />

    <!-- Hint -->
    {#if !compact}
      <div class="pt-1 px-3 pb-2 text-ink-3 text-center text-[10px] leading-[1.4] font-mono">
        tap to view · hold for actions
      </div>
    {/if}

    {#each byGroup as g}
      <div class="flex items-center justify-between py-2 px-3 border-t border-b border-bd-0 bg-bg-1 text-ink-2 uppercase tracking-[0.8px] text-[10px] leading-none font-mono">
        <span>{g.name}</span>
        <span class="text-cyan">{g.sources.length}</span>
      </div>
      {#each g.sources as s}
        {@const spark = sparkData(s.id)}
        <div
          role="button"
          tabindex="0"
          use:longpress={{ onLongpress: () => { suppressClick = true; actionSheet = s.id; ctxMenuPos = null; } }}
          onclick={() => { if (suppressClick) { suppressClick = false; return; } onSourceSelect?.(s.id); }}
          oncontextmenu={(e) => handleContextMenu(e, s.id)}
          onkeydown={(e) => { if (e.key === 'Enter') onSourceSelect?.(s.id); }}
          class="grid gap-2.5 p-2.5 px-3 border-b border-bd-0 cursor-pointer items-center select-none grid-cols-[auto_1fr_auto]"
        >
          <div class="flex items-center justify-center bg-bg-2 border border-bd-1 rounded w-7.5 h-7.5">
            <SourceGlyph kind={s.kind} size={12} />
          </div>
          <div class="min-w-0">
            <div class="flex items-center gap-1.5 text-ink-0 text-[13px] leading-[1.2] font-mono">
              <StatusDot status={s.status} />
              <span class="truncate">{s.name}</span>
            </div>
            <div class="text-ink-2 flex items-center gap-1.5 mt-0.75 text-[10px] leading-none font-mono">
              <span class="text-ink-3 truncate max-w-25">{s.host}</span>
              <span class="text-ink-3">·</span>
              <span><span class="text-cyan">{s.unread}</span>{#if s.items > 0}<span class="text-ink-3">/{s.items}</span>{/if}</span>
              <span class="text-ink-3">·</span>
              <span>{s.lastSync}</span>
              {#if s.latencyMs > 0}
                <span class="text-ink-3">·</span>
                <span style="color:{s.latencyMs > 250 ? T.amber : T.ink2};">{s.latencyMs}ms</span>
              {/if}
            </div>
          </div>
          <Sparkline data={spark} w={56} h={20} color={s.status === 'error' ? T.red : s.status === 'stale' ? T.amber : T.cyan} />
        </div>
      {/each}
    {/each}
    <div class="h-3"></div>
  </div>

  <!-- Long-press action sheet / Desktop context menu -->
    <Dialog.Root open={actionSheet !== null && actionSource !== undefined} onOpenChange={(open) => { if (!open) { actionSheet = null; ctxMenuPos = null; } }}>
      <Dialog.Portal>
        {#if isDesktop && ctxMenuPos}
          <Dialog.Overlay class="fixed inset-0 z-210 anim-sheet-overlay-in" />
          <Dialog.Content
            preventScroll={false}
            class="fixed overflow-hidden bg-bg-2 border border-bd-1 rounded w-55 shadow-[0_8px_32px_rgba(0,0,0,0.6)] z-210 anim-pop-in"
            style="
              top:{Math.min(ctxMenuPos.y, (typeof window !== 'undefined' ? window.innerHeight : 600) - 280)}px;
              left:{Math.min(ctxMenuPos.x, (typeof window !== 'undefined' ? window.innerWidth : 1000) - 220)}px;
            "
          >
            {#if actionSource}
            <div class="flex items-center gap-2.5 p-2.5 px-3 border-b border-bd-0">
              <div class="w-6 h-6 flex items-center justify-center bg-bg-1 border border-bd-1 rounded">
                <SourceGlyph kind={actionSource.kind} size={11} />
              </div>
              <div class="min-w-0">
                <div class="text-ink-0 truncate text-[11px] leading-none font-mono">{actionSource.name}</div>
                <div class="text-ink-3 truncate text-[9px] leading-none font-mono">{actionSource.host}</div>
              </div>
            </div>
            {@render actions(13, 'gap-2.5 px-3 pt-2.25 pb-2.25 text-[11px] leading-none')}
            {/if}
          </Dialog.Content>
        {:else}
          <Dialog.Overlay class="fixed inset-0 z-210 bg-black/60 anim-sheet-overlay-in" />
          <Dialog.Content
            preventScroll={false}
            class="fixed bottom-0 left-0 right-0 w-full bg-bg-2 border-t border-bd-1 pb-6 z-210 anim-sheet-in"
          >
            {#if actionSource}
            <div class="flex items-center gap-2.5 px-4 py-3.5 border-b border-bd-0">
              <div class="w-8 h-8 flex items-center justify-center bg-bg-1 border border-bd-1 rounded">
                <SourceGlyph kind={actionSource.kind} size={14} />
              </div>
              <div>
                <div class="text-ink-0 text-[13px] leading-none font-mono">{actionSource.name}</div>
                <div class="mt-1 text-ink-3 text-[10px] leading-none font-mono">{actionSource.host}</div>
              </div>
              <span class="flex-1"></span>
              <StatusDot status={actionSource.status} />
            </div>
            {@render actions(16, 'gap-3.5 px-4 pt-3.5 pb-3.5 text-[13px] leading-none')}
            <Dialog.Close
              class="flex items-center justify-center w-full px-4 bg-transparent border-none text-ink-2 cursor-pointer pt-3.5 pb-3.5 text-[12px] leading-none font-mono"
            >cancel</Dialog.Close>
            {/if}
          </Dialog.Content>
        {/if}
      </Dialog.Portal>
    </Dialog.Root>

  <!-- Edit source sheet / Desktop popover -->
    <Dialog.Root open={editingSourceId !== null} onOpenChange={(open) => { if (!open) editingSourceId = null; }}>
      <Dialog.Portal>
        {#if isDesktop}
          <Dialog.Overlay class="fixed inset-0 z-210 bg-black/50 anim-sheet-overlay-in" />
          <Dialog.Content preventScroll={false} class="fixed left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2 z-210 bg-bg-1 rounded-lg p-5 flex flex-col gap-3 w-100 max-w-[90vw] max-h-[90vh] overflow-y-auto anim-modal-in">
            {#key editingSourceId}
              <SourceForm mode="edit" initial={editInitial} groups={groups} onSubmit={closeEdit} onCancel={closeEdit} />
            {/key}
          </Dialog.Content>
        {:else}
          <Dialog.Overlay class="fixed inset-0 z-210 flex flex-col justify-end bg-black/50 anim-sheet-overlay-in" />
          <Dialog.Content preventScroll={false} class="fixed bottom-0 left-0 right-0 z-210 bg-bg-1 rounded-t-xl p-4 flex flex-col gap-3 anim-sheet-in" style="padding-bottom:max(16px, env(safe-area-inset-bottom));">
            {#key editingSourceId}
              <SourceForm mode="edit" initial={editInitial} groups={groups} onSubmit={closeEdit} onCancel={closeEdit} />
            {/key}
          </Dialog.Content>
        {/if}
      </Dialog.Portal>
    </Dialog.Root>
</div>

{#snippet actions(iconSize: number, padCls: string)}
  {#each sourceActions as act}
    <button
      onclick={act.action}
      class="flex items-center w-full bg-transparent border-none border-b border-bd-0 cursor-pointer text-left font-mono {padCls}"
      style="color:{act.label === 'Remove source' ? T.red : T.ink0};-webkit-tap-highlight-color:transparent;"
    >
      <Icon name={act.icon} size={iconSize} color={act.label === 'Remove source' ? T.red : act.label === 'Edit source' ? T.cyan : T.ink2} />
      {act.label}
    </button>
  {/each}
{/snippet}
