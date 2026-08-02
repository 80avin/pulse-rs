<script lang="ts">
  import { T } from '$lib/tokens';
  import { Dialog } from 'bits-ui';
  import { sources, groups, items, markSourceRead, addSource as storeAddSource, removeSource as storeRemoveSource, updateSource as storeUpdateSource, syncSource as storeSyncSource, createGroup, detectFeed } from '$lib/stores/data.svelte';
  import { logger } from '$lib/logger';
  import StatusDot from '$lib/components/StatusDot.svelte';
  import SourceGlyph from '$lib/components/SourceGlyph.svelte';
  import Sparkline from '$lib/components/Sparkline.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import SegmentedControl from '$lib/components/SegmentedControl.svelte';
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

  let addUrl = $state('');
  let addGroup = $state(groups[0]?.id ?? '');
  let newGroupName = $state('');
  let addInputEl: HTMLInputElement | null = $state(null);
  let actionSheet = $state<string | null>(null);
  let ctxMenuPos = $state<{ x: number; y: number } | null>(null);

  let editingSourceId = $state<string | null>(null);
  let editUrl  = $state('');
  let editName = $state('');
  let editKind = $state<'rss'|'hn'|'reddit'>('rss');
  let editGroup = $state('all');
  let editHue  = $state<number | undefined>(undefined);
  let fetchingTitle = $state(false);

  function openEditSheet(id: string) {
    const s = sources.find(s => s.id === id);
    if (!s) return;
    editingSourceId = id;
    editUrl   = s.url ?? s.host ?? '';
    editName  = s.name;
    editKind  = s.kind;
    editGroup = s.group;
    editHue   = s.hue;
    actionSheet = null;
  }

  async function fetchTitleForUrl(url: string) {
    if (!url) return;
    fetchingTitle = true;
    try {
      const preview = await detectFeed(url);
      if (preview?.name) {
        editName = preview.name;
      }
    } finally {
      fetchingTitle = false;
    }
  }

  async function submitEditSource() {
    if (!editingSourceId) return;
    const { url: normUrl } = inferSourceMeta(editUrl.trim());
    await storeUpdateSource(editingSourceId, editName.trim() || normUrl, normUrl, editKind, editGroup, editHue);
    editingSourceId = null;
  }

  function handleContextMenu(e: MouseEvent, sourceId: string) {
    e.preventDefault();
    ctxMenuPos = { x: e.clientX, y: e.clientY };
    actionSheet = sourceId;
  }

  const actionSource = $derived(actionSheet ? sources.find(s => s.id === actionSheet) : null);

  function inferSourceMeta(rawUrl: string): { kind: 'rss' | 'reddit' | 'hn'; name: string; url: string } {
    const normalised = /^https?:\/\//i.test(rawUrl) ? rawUrl : `https://${rawUrl}`;
    let parsed: URL | null = null;
    try { parsed = new URL(normalised); } catch {}
    const host = parsed?.hostname ?? '';
    if (host.includes('reddit.com')) {
      const m = (parsed?.pathname ?? '').match(/^\/r\/([^/]+)/i);
      return { kind: 'reddit', name: m ? `r/${m[1]}` : 'Reddit', url: normalised };
    }
    if (host.includes('ycombinator.com')) {
      return { kind: 'hn', name: 'Hacker News', url: normalised };
    }
    const domain = host.replace(/^www\./, '');
    const baseName = domain.split('.')[0];
    return { kind: 'rss', name: baseName || domain || rawUrl, url: normalised };
  }

  async function submitAddSource() {
    const url = addUrl.trim();
    if (!url) return;
    const { kind, name, url: normUrl } = inferSourceMeta(url);

    let groupId: string;
    if (addGroup === '__new__') {
      const trimmed = newGroupName.trim();
      if (!trimmed) return;
      await createGroup(trimmed);
      const newId = trimmed.toLowerCase().replace(/\s+/g, '-').replace(/[^a-z0-9-]/g, '');
      groupId = newId || (groups[0]?.id ?? 'all');
      newGroupName = '';
      addGroup = groupId;
    } else {
      groupId = addGroup || (groups[0]?.id ?? 'all');
    }

    addUrl = '';
    const newSourceId = await storeAddSource(name, normUrl, kind, groupId);
    storeSyncSource(newSourceId).catch(e => logger.warn('sync after source add failed', e));
  }

  async function removeSource(id: string) {
    await storeRemoveSource(id);
  }
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
    <div class="add-source-target mx-2.5 my-3 p-2.5 px-3 bg-bg-1 border border-dashed border-bd-2 rounded text-ink-1 text-[11px] leading-[1.4] font-mono">
      <div class="flex items-center gap-2 mb-2">
        <Icon name="plus" size={13} color={T.cyan} />
        <span class="text-ink-0 tracking-[0.4px]">ADD SOURCE</span>
      </div>
      <div class="flex bg-bg-0 border border-bd-1 rounded mb-2">
        <div class="px-2 py-1.5 text-cyan border-r border-bd-1 text-[11px] leading-none font-mono">$</div>
        <input
          bind:this={addInputEl}
          bind:value={addUrl}
          placeholder="https://example.com/feed.xml"
          onkeydown={(e) => { if (e.key === 'Enter') submitAddSource(); }}
          class="flex-1 px-2 py-1.5 text-ink-0 text-[11px] leading-none font-mono"
        />
      </div>
      <div class="flex gap-1.5">
        <select bind:value={addGroup} class="flex-1 bg-bg-0 text-ink-1 border border-bd-1 rounded px-2 py-1.5 text-[11px] leading-none font-mono">
          {#each groups as g}
            <option value={g.id}>group: {g.name}</option>
          {/each}
          <option value="__new__">+ create new group</option>
        </select>
        <button
          onclick={submitAddSource}
          class="px-3.5 bg-cyan text-bg-0 border-none rounded cursor-pointer font-semibold tracking-[0.4px] text-[11px] leading-none font-mono"
        >+ ADD</button>
      </div>
      {#if addGroup === '__new__'}
        <div class="mt-1">
          <input
            bind:value={newGroupName}
            placeholder="new group name"
            onkeydown={(e) => { if (e.key === 'Enter') submitAddSource(); }}
            class="w-full px-2 py-1.5 bg-bg-0 text-ink-0 border border-cyan rounded box-border outline-none text-[11px] leading-none font-mono"
          />
        </div>
      {/if}
    </div>

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
          use:longpress={{ onLongpress: () => { actionSheet = s.id; ctxMenuPos = null; } }}
          onclick={() => onSourceSelect?.(s.id)}
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
          <Dialog.Overlay class="fixed inset-0 z-210" />
          <Dialog.Content
            preventScroll={false}
            class="fixed overflow-hidden bg-bg-2 border border-bd-1 rounded w-55 shadow-[0_8px_32px_rgba(0,0,0,0.6)] z-210"
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
            {#each [
              { icon: 'list',  label: 'View feed',     action: () => { onSourceSelect(actionSheet!); actionSheet = null; } },
              { icon: 'sync',  label: 'Refresh now',   action: () => { storeSyncSource(actionSheet!); actionSheet = null; } },
              { icon: 'edit',  label: 'Edit source',   action: () => openEditSheet(actionSheet!) },
              { icon: 'star',  label: 'Mark all read', action: () => { markSourceRead(actionSheet!); actionSheet = null; } },
              { icon: 'trash', label: 'Remove source', action: () => { removeSource(actionSheet!); actionSheet = null; } },
            ] as act}
              <button
                onclick={act.action}
                class="flex items-center gap-2.5 w-full px-3 bg-transparent border-none border-b border-bd-0 cursor-pointer text-left pt-2.25 pb-2.25 text-[11px] leading-none font-mono"
                style="color:{act.label === 'Remove source' ? T.red : T.ink0};"
              >
                <Icon name={act.icon} size={13} color={act.label === 'Remove source' ? T.red : act.label === 'Edit source' ? T.cyan : T.ink2} />
                {act.label}
              </button>
            {/each}
            {/if}
          </Dialog.Content>
        {:else}
          <Dialog.Overlay class="fixed inset-0 z-210 bg-black/60" />
          <Dialog.Content
            preventScroll={false}
            class="fixed bottom-0 left-0 right-0 w-full bg-bg-2 border-t border-bd-1 pb-6 z-210"
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
            {#each [
              { icon: 'list',  label: 'View feed',     action: () => { onSourceSelect(actionSheet!); actionSheet = null; } },
              { icon: 'sync',  label: 'Refresh now',   action: () => { storeSyncSource(actionSheet!); actionSheet = null; } },
              { icon: 'edit',  label: 'Edit source',   action: () => openEditSheet(actionSheet!) },
              { icon: 'star',  label: 'Mark all read', action: () => { markSourceRead(actionSheet!); actionSheet = null; } },
              { icon: 'trash', label: 'Remove source', action: () => { removeSource(actionSheet!); actionSheet = null; } },
            ] as act}
              <button
                onclick={act.action}
                class="flex items-center gap-3.5 w-full px-4 bg-transparent border-none border-b border-bd-0 cursor-pointer text-left pt-3.5 pb-3.5 text-[13px] leading-none font-mono"
                style="color:{act.label === 'Remove source' ? T.red : T.ink0};-webkit-tap-highlight-color:transparent;"
              >
                <Icon name={act.icon} size={16} color={act.label === 'Remove source' ? T.red : act.label === 'Edit source' ? T.cyan : T.ink2} />
                {act.label}
              </button>
            {/each}
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
          <Dialog.Overlay class="fixed inset-0 z-210 bg-black/50" />
          <Dialog.Content preventScroll={false} class="fixed left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2 z-210 bg-bg-1 rounded-lg p-5 flex flex-col gap-3 w-100 max-w-[90vw] max-h-[90vh] overflow-y-auto">
            {@render editForm()}
          </Dialog.Content>
        {:else}
          <Dialog.Overlay class="fixed inset-0 z-210 flex flex-col justify-end bg-black/50" />
          <Dialog.Content preventScroll={false} class="relative bg-bg-1 rounded-t-xl p-4 flex flex-col gap-3" style="padding-bottom:max(16px, env(safe-area-inset-bottom));">
            {@render editForm()}
          </Dialog.Content>
        {/if}
      </Dialog.Portal>
    </Dialog.Root>
</div>

{#snippet editForm()}
  <div class="text-ink-2 uppercase mb-1 tracking-[0.5px] text-[11px] leading-none font-mono">edit source</div>

  <div class="flex flex-col gap-1.5">
    <label for="edit-url" class="text-ink-3 text-[10px] leading-none font-mono">URL</label>
    <input
      id="edit-url"
      bind:value={editUrl}
      placeholder="https://example.com/feed.xml"
      class="w-full p-2.5 bg-bg-0 border border-bd-1 rounded text-ink-0 outline-none box-border text-[12px] leading-none font-mono"
      oninput={() => { editKind = inferSourceMeta(editUrl).kind; }}
    />
  </div>

  <div class="flex flex-col gap-1.5">
    <div class="flex items-center justify-between">
      <label for="edit-name" class="text-ink-3 text-[10px] leading-none font-mono">NAME</label>
      <button
        onclick={() => fetchTitleForUrl(editUrl)}
        disabled={fetchingTitle}
        class="bg-transparent border border-bd-1 rounded p-[2px_8px] text-[9px] leading-none font-mono" style="color:{fetchingTitle ? T.ink3 : T.cyan};cursor:{fetchingTitle ? 'default' : 'pointer'};"
      >{fetchingTitle ? 'fetching…' : 'fetch title'}</button>
    </div>
    <input
      id="edit-name"
      bind:value={editName}
      placeholder="Display name"
      class="w-full p-2.5 bg-bg-0 border border-bd-1 rounded text-ink-0 outline-none box-border text-[12px] leading-none font-mono"
    />
  </div>

  <div class="flex gap-2">
    <div class="flex-1 flex flex-col gap-1.5">
      <span class="text-ink-3 text-[10px] leading-none font-mono">TYPE</span>
        <SegmentedControl options={['rss','hn','reddit']} active={editKind} onChange={v => { editKind = v as typeof editKind; }} />
    </div>
    <div class="flex-1 flex flex-col gap-1.5">
      <label for="se-group" class="text-ink-3 text-[10px] leading-none font-mono">GROUP</label>
      <select id="se-group"
        bind:value={editGroup}
        class="w-full p-2 bg-bg-0 border border-bd-1 rounded text-ink-0 cursor-pointer text-[12px] leading-none font-mono"
      >
        {#each groups as g}<option value={g.id}>{g.name}</option>{/each}
      </select>
    </div>
  </div>

  <div class="flex flex-col gap-1.5">
    <div class="flex items-center justify-between">
      <span class="text-ink-3 text-[10px] leading-none font-mono">COLOUR</span>
      {#if editHue != null}
        <button
          onclick={() => editHue = undefined}
          class="bg-transparent border-none text-ink-3 cursor-pointer p-0 text-[9px] leading-none font-mono"
        >reset</button>
      {/if}
    </div>
    <div class="flex items-center gap-2">
      <input
        type="range"
        min="0" max="360"
        value={editHue ?? 200}
        oninput={(e) => editHue = parseInt((e.target as HTMLInputElement).value)}
        class="flex-1 h-1.5" style="accent-color:{T.cyan};"
      />
      <div class="w-7 h-7 rounded-[3px] shrink-0 border border-bd-1" style="
        background:{editHue != null ? `oklch(0.45 0.14 ${editHue})` : T.ink4};
      "></div>
    </div>
  </div>

  <div class="flex gap-2 mt-1">
    <Dialog.Close
      class="flex-1 p-3 bg-transparent border border-bd-1 rounded text-ink-2 cursor-pointer text-[12px] leading-none font-mono"
    >cancel</Dialog.Close>
    <button
      onclick={submitEditSource}
      class="flex-2 p-3 bg-cyan border-none rounded text-bg-0 cursor-pointer font-semibold text-[12px] leading-none font-mono"
    >save changes</button>
  </div>
{/snippet}
