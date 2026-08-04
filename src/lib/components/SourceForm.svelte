<script lang="ts">
  import { T } from '$lib/tokens';
  import Icon from '$lib/components/Icon.svelte';
  import SegmentedControl from '$lib/components/SegmentedControl.svelte';
  import { logger } from '$lib/logger';
  import {
    addSource as storeAddSource,
    updateSource as storeUpdateSource,
    syncSource as storeSyncSource,
    createGroup,
    detectFeed,
  } from '$lib/stores/data.svelte';

  type SourceKind = 'rss' | 'hn' | 'reddit';

  export interface SourceFormValues {
    id?: string;
    name: string;
    url: string;
    kind: SourceKind;
    group: string;
    hue?: number;
  }

  let {
    mode,
    initial,
    groups,
    onSubmit,
    onCancel,
  }: {
    mode: 'add' | 'edit';
    initial: SourceFormValues;
    groups: { id: string; name: string }[];
    onSubmit?: (values: SourceFormValues) => void | Promise<void>;
    onCancel?: () => void;
  } = $props();

  const init = () => ({
    name: initial.name,
    url: initial.url,
    kind: initial.kind,
    group: initial.group,
    hue: initial.hue,
  });

  let urlDraft = $state(init().url);
  let nameDraft = $state(init().name);
  let kindDraft = $state<SourceKind>(init().kind);
  let groupDraft = $state(init().group);
  let hueDraft = $state<number | undefined>(init().hue);
  let newGroupName = $state('');
  let fetchingTitle = $state(false);
  let nameTouched = $state(false);
  let fetchError = $state('');
  let hueOpen = $state(false);
  let expanded = $state(false);

  $effect(() => {
    if (mode !== 'add' || typeof window === 'undefined') return;
    function open() { expanded = true; }
    window.addEventListener('pulse:open-add-source', open);
    return () => window.removeEventListener('pulse:open-add-source', open);
  });

  function inferSourceMeta(rawUrl: string): { kind: SourceKind; name: string; url: string } {
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

  async function fetchTitleForUrl(url: string) {
    if (!url.trim()) { fetchError = 'enter a URL first'; return; }
    fetchError = '';
    fetchingTitle = true;
    try {
      const preview = await detectFeed(url);
      if (preview?.name) {
        nameDraft = preview.name;
      } else {
        fetchError = 'no feed detected at that url';
      }
    } catch {
      fetchError = 'could not fetch the title';
    } finally {
      fetchingTitle = false;
    }
  }

  async function submitAddSource() {
    const url = urlDraft.trim();
    if (!url) return;
    const { url: normUrl } = inferSourceMeta(url);
    const name = nameDraft.trim() || normUrl;

    let groupId: string;
    if (groupDraft === '__new__') {
      const trimmed = newGroupName.trim();
      if (!trimmed) return;
      await createGroup(trimmed);
      groupId = trimmed.toLowerCase().replace(/\s+/g, '-').replace(/[^a-z0-9-]/g, '');
      if (!groupId) groupId = groups[0]?.id ?? 'all';
      newGroupName = '';
      groupDraft = groupId;
    } else {
      groupId = groupDraft || (groups[0]?.id ?? 'all');
    }

    urlDraft = '';
    nameDraft = '';
    nameTouched = false;
    const newSourceId = await storeAddSource(name, normUrl, kindDraft, groupId, hueDraft);
    storeSyncSource(newSourceId).catch(e => logger.warn('sync after source add failed', e));
    await onSubmit?.({ name, url: normUrl, kind: kindDraft, group: groupId, hue: hueDraft });
  }

  async function submitEditSource() {
    if (!initial.id) return;
    const { url: normUrl } = inferSourceMeta(urlDraft.trim());
    const name = nameDraft.trim() || normUrl;
    await storeUpdateSource(initial.id, name, normUrl, kindDraft, groupDraft, hueDraft);
    await onSubmit?.({ id: initial.id, name, url: normUrl, kind: kindDraft, group: groupDraft, hue: hueDraft });
  }
</script>

<div class={mode === 'add' ? 'add-source-target mx-2.5 my-3 p-2.5 px-3 bg-bg-1 border border-dashed border-bd-2 rounded text-ink-1 text-[11px] leading-[1.4] font-mono' : ''}>
  {#if mode === 'add'}
    <button
      onclick={() => { expanded = !expanded; }}
      aria-expanded={expanded}
      class="flex items-center gap-2 w-full bg-transparent border-none cursor-pointer p-0 mb-2"
    >
      <Icon name={expanded ? 'x' : 'plus'} size={13} color={T.cyan} />
      <span class="text-ink-0 tracking-[0.4px]">ADD SOURCE</span>
      <span class="flex-1"></span>
      <Icon name="chev-dn" size={10} color={T.ink3} />
    </button>
    {#if !expanded}
      <div class="text-ink-3 text-[10px] leading-none font-mono">add an rss / hn / reddit feed</div>
    {/if}
  {:else}
    <div class="text-ink-2 uppercase mb-1 tracking-[0.5px] text-[11px] leading-none font-mono">edit source</div>
  {/if}

  {#if mode === 'edit' || expanded}
    <div class="flex flex-col gap-1.5">
    <div class="flex flex-col gap-1.5">
      <label for="src-url" class="text-ink-3 text-[10px] leading-none font-mono">URL</label>
      <input
        id="src-url"
        bind:value={urlDraft}
        placeholder="https://example.com/feed.xml"
        onkeydown={(e) => { if (e.key === 'Enter' && mode === 'add') submitAddSource(); }}
        oninput={() => { fetchError = ''; const meta = inferSourceMeta(urlDraft); kindDraft = meta.kind; if (mode === 'add' && !nameTouched) nameDraft = meta.name; }}
        class="w-full p-2.5 bg-bg-0 border border-bd-1 rounded text-ink-0 outline-none box-border text-[12px] leading-none font-mono"
      />
    </div>

    <div class="flex flex-col gap-1.5">
      <div class="flex items-center justify-between">
        <label for="src-name" class="text-ink-3 text-[10px] leading-none font-mono">NAME</label>
        <button
          onclick={() => fetchTitleForUrl(urlDraft)}
          disabled={fetchingTitle}
          class="bg-transparent border border-bd-1 rounded p-[2px_8px] text-[9px] leading-none font-mono" style="color:{fetchingTitle ? T.ink3 : T.cyan};cursor:{fetchingTitle ? 'default' : 'pointer'};"
        >{fetchingTitle ? 'fetching…' : 'fetch title'}</button>
      </div>
      <input
        id="src-name"
        bind:value={nameDraft}
        oninput={() => { nameTouched = true; }}
        onkeydown={(e) => { if (e.key === 'Enter' && mode === 'add') submitAddSource(); }}
        placeholder="Display name"
        class="w-full p-2.5 bg-bg-0 border border-bd-1 rounded text-ink-0 outline-none box-border text-[12px] leading-none font-mono"
      />
      {#if fetchError}
        <div class="text-red text-[10px] leading-none font-mono">{fetchError}</div>
      {/if}
    </div>

    <div class="flex gap-2">
      <div class="flex-1 flex flex-col gap-1.5">
        <span class="text-ink-3 text-[10px] leading-none font-mono">TYPE</span>
        <SegmentedControl options={['rss','hn','reddit']} active={kindDraft} onChange={v => { kindDraft = v as typeof kindDraft; }} />
      </div>
      <div class="flex-1 flex flex-col gap-1.5">
        <label for="src-group" class="text-ink-3 text-[10px] leading-none font-mono">GROUP</label>
        <select id="src-group"
          bind:value={groupDraft}
          class="w-full p-2 bg-bg-0 border border-bd-1 rounded text-ink-0 cursor-pointer text-[12px] leading-none font-mono"
        >
          {#each groups as g}<option value={g.id}>{g.name}</option>{/each}
          {#if mode === 'add'}<option value="__new__">+ create new group</option>{/if}
        </select>
      </div>
    </div>
    {#if groupDraft === '__new__'}
      <input
        bind:value={newGroupName}
        placeholder="new group name"
        onkeydown={(e) => { if (e.key === 'Enter') submitAddSource(); }}
        class="w-full p-2 bg-bg-0 text-ink-0 border border-cyan rounded box-border outline-none text-[12px] leading-none font-mono"
      />
    {/if}

    <div class="flex items-center justify-between gap-2">
      <span class="text-ink-3 text-[10px] leading-none font-mono">COLOUR</span>
      <div class="flex items-center gap-1.5">
        {#if hueOpen}
          <input
            type="range"
            min="0" max="360"
            value={hueDraft ?? 200}
            oninput={(e) => hueDraft = parseInt((e.target as HTMLInputElement).value)}
            class="w-24 h-1.5 accent-cyan"
          />
          {#if hueDraft != null}
            <button
              onclick={() => hueDraft = undefined}
              class="bg-transparent border-none text-ink-3 cursor-pointer p-0 text-[9px] leading-none font-mono"
            >reset</button>
          {/if}
        {/if}
        <button
          onclick={() => hueOpen = !hueOpen}
          aria-label="Feed colour"
          aria-expanded={hueOpen}
          title="Colour"
          class="w-6 h-6 rounded-[3px] border border-bd-1 cursor-pointer shrink-0"
          style="background:{hueDraft != null ? `oklch(0.45 0.14 ${hueDraft})` : T.ink4};"
        ></button>
      </div>
    </div>

    <div class="flex gap-2 mt-1">
      {#if mode === 'edit'}
        <button
          onclick={onCancel}
          class="flex-1 p-3 bg-transparent border border-bd-1 rounded text-ink-2 cursor-pointer text-[12px] leading-none font-mono"
        >cancel</button>
        <button
          onclick={submitEditSource}
          class="flex-2 p-3 bg-cyan border-none rounded text-bg-0 cursor-pointer font-semibold text-[12px] leading-none font-mono"
        >save changes</button>
      {:else}
        <button
          onclick={submitAddSource}
          class="flex-2 p-3 bg-cyan border-none rounded text-bg-0 cursor-pointer font-semibold text-[12px] leading-none font-mono"
        >+ ADD</button>
      {/if}
    </div>
  </div>
  {/if}
</div>
