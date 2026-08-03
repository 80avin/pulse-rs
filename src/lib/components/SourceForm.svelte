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
    if (!url) return;
    fetchingTitle = true;
    try {
      const preview = await detectFeed(url);
      if (preview?.name) {
        nameDraft = preview.name;
      }
    } finally {
      fetchingTitle = false;
    }
  }

  async function submitAddSource() {
    const url = urlDraft.trim();
    if (!url) return;
    const { kind, name, url: normUrl } = inferSourceMeta(url);

    let groupId: string;
    if (groupDraft === '__new__') {
      const trimmed = newGroupName.trim();
      if (!trimmed) return;
      await createGroup(trimmed);
      const newId = trimmed.toLowerCase().replace(/\s+/g, '-').replace(/[^a-z0-9-]/g, '');
      groupId = newId || (groups[0]?.id ?? 'all');
      newGroupName = '';
      groupDraft = groupId;
    } else {
      groupId = groupDraft || (groups[0]?.id ?? 'all');
    }

    urlDraft = '';
    const newSourceId = await storeAddSource(name, normUrl, kind, groupId);
    storeSyncSource(newSourceId).catch(e => logger.warn('sync after source add failed', e));
    await onSubmit?.({ name, url: normUrl, kind, group: groupId });
  }

  async function submitEditSource() {
    if (!initial.id) return;
    const { url: normUrl } = inferSourceMeta(urlDraft.trim());
    const name = nameDraft.trim() || normUrl;
    await storeUpdateSource(initial.id, name, normUrl, kindDraft, groupDraft, hueDraft);
    await onSubmit?.({ id: initial.id, name, url: normUrl, kind: kindDraft, group: groupDraft, hue: hueDraft });
  }
</script>

{#if mode === 'add'}
  <div class="add-source-target mx-2.5 my-3 p-2.5 px-3 bg-bg-1 border border-dashed border-bd-2 rounded text-ink-1 text-[11px] leading-[1.4] font-mono">
    <div class="flex items-center gap-2 mb-2">
      <Icon name="plus" size={13} color={T.cyan} />
      <span class="text-ink-0 tracking-[0.4px]">ADD SOURCE</span>
    </div>
    <div class="flex bg-bg-0 border border-bd-1 rounded mb-2">
      <div class="px-2 py-1.5 text-cyan border-r border-bd-1 text-[11px] leading-none font-mono">$</div>
      <input
        bind:value={urlDraft}
        placeholder="https://example.com/feed.xml"
        onkeydown={(e) => { if (e.key === 'Enter') submitAddSource(); }}
        class="flex-1 px-2 py-1.5 text-ink-0 text-[11px] leading-none font-mono"
      />
    </div>
    <div class="flex gap-1.5">
      <select bind:value={groupDraft} class="flex-1 bg-bg-0 text-ink-1 border border-bd-1 rounded px-2 py-1.5 text-[11px] leading-none font-mono">
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
    {#if groupDraft === '__new__'}
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
{:else}
  <div class="text-ink-2 uppercase mb-1 tracking-[0.5px] text-[11px] leading-none font-mono">edit source</div>

  <div class="flex flex-col gap-1.5">
    <label for="edit-url" class="text-ink-3 text-[10px] leading-none font-mono">URL</label>
    <input
      id="edit-url"
      bind:value={urlDraft}
      placeholder="https://example.com/feed.xml"
      class="w-full p-2.5 bg-bg-0 border border-bd-1 rounded text-ink-0 outline-none box-border text-[12px] leading-none font-mono"
      oninput={() => { kindDraft = inferSourceMeta(urlDraft).kind; }}
    />
  </div>

  <div class="flex flex-col gap-1.5">
    <div class="flex items-center justify-between">
      <label for="edit-name" class="text-ink-3 text-[10px] leading-none font-mono">NAME</label>
      <button
        onclick={() => fetchTitleForUrl(urlDraft)}
        disabled={fetchingTitle}
        class="bg-transparent border border-bd-1 rounded p-[2px_8px] text-[9px] leading-none font-mono" style="color:{fetchingTitle ? T.ink3 : T.cyan};cursor:{fetchingTitle ? 'default' : 'pointer'};"
      >{fetchingTitle ? 'fetching…' : 'fetch title'}</button>
    </div>
    <input
      id="edit-name"
      bind:value={nameDraft}
      placeholder="Display name"
      class="w-full p-2.5 bg-bg-0 border border-bd-1 rounded text-ink-0 outline-none box-border text-[12px] leading-none font-mono"
    />
  </div>

  <div class="flex gap-2">
    <div class="flex-1 flex flex-col gap-1.5">
      <span class="text-ink-3 text-[10px] leading-none font-mono">TYPE</span>
      <SegmentedControl options={['rss','hn','reddit']} active={kindDraft} onChange={v => { kindDraft = v as typeof kindDraft; }} />
    </div>
    <div class="flex-1 flex flex-col gap-1.5">
      <label for="se-group" class="text-ink-3 text-[10px] leading-none font-mono">GROUP</label>
      <select id="se-group"
        bind:value={groupDraft}
        class="w-full p-2 bg-bg-0 border border-bd-1 rounded text-ink-0 cursor-pointer text-[12px] leading-none font-mono"
      >
        {#each groups as g}<option value={g.id}>{g.name}</option>{/each}
      </select>
    </div>
  </div>

  <div class="flex flex-col gap-1.5">
    <div class="flex items-center justify-between">
      <span class="text-ink-3 text-[10px] leading-none font-mono">COLOUR</span>
      {#if hueDraft != null}
        <button
          onclick={() => hueDraft = undefined}
          class="bg-transparent border-none text-ink-3 cursor-pointer p-0 text-[9px] leading-none font-mono"
        >reset</button>
      {/if}
    </div>
    <div class="flex items-center gap-2">
      <input
        type="range"
        min="0" max="360"
        value={hueDraft ?? 200}
        oninput={(e) => hueDraft = parseInt((e.target as HTMLInputElement).value)}
        class="flex-1 h-1.5" style="accent-color:{T.cyan};"
      />
      <div class="w-7 h-7 rounded-[3px] shrink-0 border border-bd-1" style="
        background:{hueDraft != null ? `oklch(0.45 0.14 ${hueDraft})` : T.ink4};
      "></div>
    </div>
  </div>

  <div class="flex gap-2 mt-1">
    <button
      onclick={onCancel}
      class="flex-1 p-3 bg-transparent border border-bd-1 rounded text-ink-2 cursor-pointer text-[12px] leading-none font-mono"
    >cancel</button>
    <button
      onclick={submitEditSource}
      class="flex-2 p-3 bg-cyan border-none rounded text-bg-0 cursor-pointer font-semibold text-[12px] leading-none font-mono"
    >save changes</button>
  </div>
{/if}
