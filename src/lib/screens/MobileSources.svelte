<script lang="ts">
  import { T } from '$lib/tokens';
  import { sources, groups, items, markAllRead, markSourceRead, addSource as storeAddSource, removeSource as storeRemoveSource, updateSource as storeUpdateSource, doSync as storeSync, syncSource as storeSyncSource, createGroup } from '$lib/store.svelte';
  import { logger } from '$lib/logger';
  import PulseBottomNav from '$lib/components/PulseBottomNav.svelte';
  import StatusDot from '$lib/components/StatusDot.svelte';
  import SourceGlyph from '$lib/components/SourceGlyph.svelte';
  import Sparkline from '$lib/components/Sparkline.svelte';
  import Icon from '$lib/components/Icon.svelte';

  let { tab, onTabChange, onSourceSelect }: {
    tab: string;
    onTabChange: (id: string) => void;
    onSourceSelect: (sourceId: string) => void;
  } = $props();

  // Group sources by kind
  const byKind = $derived.by(() => {
    const r: Record<string, typeof sources> = { hn: [], reddit: [], rss: [] };
    for (const s of sources) {
      const k = s.kind in r ? s.kind : 'rss';
      r[k].push(s);
    }
    return r;
  });

  const okCount    = $derived(sources.filter(s => s.status === 'ok').length);
  const staleCount = $derived(sources.filter(s => s.status === 'stale').length);
  const errCount   = $derived(sources.filter(s => s.status === 'error').length);

  // 14-day activity sparkline from in-memory items for a given source.
  // Buckets: index 0 = 13 days ago, index 13 = today.
  function sparkData(sourceId: string): number[] {
    const buckets = new Array<number>(14).fill(0);
    for (const item of items) {
      if (item.src !== sourceId) continue;
      const a = item.age;
      let daysAgo = 0;
      if (a.endsWith('d'))      daysAgo = parseInt(a);
      else if (a.endsWith('h')) daysAgo = 0; // same day
      // 'm' / 'now' → 0; anything unparseable → 0
      const bucket = Math.min(13, Math.max(0, daysAgo));
      buckets[13 - bucket]++;
    }
    return buckets;
  }

  let addUrl = $state('');
  let addGroup = $state(groups[0]?.id ?? '');
  let newGroupName = $state('');
  let addInputEl: HTMLInputElement | null = $state(null);
  let syncing = $state(false);
  let actionSheet = $state<string | null>(null); // source ID for long-press sheet

  // Edit sheet state
  let editingSourceId = $state<string | null>(null);
  let editUrl  = $state('');
  let editName = $state('');
  let editKind = $state<'rss'|'hn'|'reddit'>('rss');
  let editGroup = $state('all');

  function openEditSheet(id: string) {
    const s = sources.find(s => s.id === id);
    if (!s) return;
    editingSourceId = id;
    editUrl   = s.url ?? s.host ?? '';
    editName  = s.name;
    editKind  = s.kind;
    editGroup = s.group;
    actionSheet = null;
  }

  async function submitEditSource() {
    if (!editingSourceId) return;
    const { url: normUrl } = inferSourceMeta(editUrl.trim());
    await storeUpdateSource(editingSourceId, editName.trim() || normUrl, normUrl, editKind, editGroup);
    editingSourceId = null;
  }

  // Gesture detection: distinguishes tap, long-press, and scroll.
  // On mobile, e.preventDefault() in touchend suppresses the synthetic
  // click the browser fires ~300ms later, eliminating ghost navigation.
  let pressTimer: ReturnType<typeof setTimeout> | null = null;
  let longPressed  = false;
  let touchMoved   = false;
  let touchStartX  = 0;
  let touchStartY  = 0;

  function startPress(e: TouchEvent, sourceId: string) {
    touchStartX = e.touches[0].clientX;
    touchStartY = e.touches[0].clientY;
    touchMoved  = false;
    longPressed = false;
    pressTimer = setTimeout(() => {
      if (touchMoved) return; // user is scrolling — don't open sheet
      longPressed = true;
      pressTimer  = null;
      actionSheet = sourceId;
    }, 450);
  }

  function handleTouchMove(e: TouchEvent) {
    const dx = Math.abs(e.touches[0].clientX - touchStartX);
    const dy = Math.abs(e.touches[0].clientY - touchStartY);
    if (dx > 8 || dy > 8) {
      touchMoved = true;
      // Cancel long-press if the user starts scrolling.
      if (pressTimer) { clearTimeout(pressTimer); pressTimer = null; }
    }
  }

  function cancelPress() {
    if (pressTimer) { clearTimeout(pressTimer); pressTimer = null; }
    touchMoved  = false;
    longPressed = false;
  }

  function handleTouchEnd(e: TouchEvent, sourceId: string) {
    // Suppress the synthetic mouse/click events the browser fires after touchend.
    e.preventDefault();
    const wasLong  = longPressed;
    const wasMoved = touchMoved;
    cancelPress();
    if (!wasLong && !wasMoved) onSourceSelect(sourceId);
  }
  function handleContextMenu(e: MouseEvent, sourceId: string) {
    e.preventDefault();
    actionSheet = sourceId;
  }

  async function doSync() {
    if (syncing) return;
    syncing = true;
    await storeSync();
    syncing = false;
  }

  const actionSource = $derived(actionSheet ? sources.find(s => s.id === actionSheet) : null);

  function inferSourceMeta(rawUrl: string): { kind: 'rss' | 'reddit' | 'hn'; name: string; url: string } {
    // Normalise: add https:// if no protocol so URL() can parse it.
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
      if (!trimmed) return; // don't submit without a new group name
      await createGroup(trimmed);
      // After createGroup, find the newly created group's id (derived from name)
      const newId = trimmed.toLowerCase().replace(/\s+/g, '-').replace(/[^a-z0-9-]/g, '');
      groupId = newId || (groups[0]?.id ?? 'all');
      newGroupName = '';
      addGroup = groupId;
    } else {
      groupId = addGroup || (groups[0]?.id ?? 'all');
    }

    addUrl = '';
    const newSourceId = await storeAddSource(name, normUrl, kind, groupId);
    // Immediately fetch the new feed so items appear without a manual sync.
    storeSyncSource(newSourceId).catch(e => logger.warn('sync after mobile source add failed', e));
  }

  async function removeSource(id: string) {
    await storeRemoveSource(id);
  }
</script>

<div style="display:flex;flex-direction:column;height:100%;background:{T.bg0};color:{T.ink0};">
  <!-- Top bar -->
  <div style="height:44px;display:flex;align-items:center;padding:0 10px;border-bottom:1px solid {T.bd0};background:{T.bg1};flex-shrink:0;gap:8px;">
    <span style="font:12px/1 {T.mono};color:{T.ink0};letter-spacing:0.5px;flex:1;">
      sources <span style="color:{T.ink3};">· {sources.length}</span>
    </span>
    <button onclick={doSync} style="width:34px;height:34px;display:inline-flex;align-items:center;justify-content:center;background:transparent;border:none;cursor:pointer;border-radius:4px;">
      <span class={syncing ? 'syncing' : ''}>
        <Icon name="sync" size={16} color={syncing ? T.cyan : T.ink1} />
      </span>
    </button>
    <button
      onclick={() => { addInputEl?.focus(); addInputEl?.scrollIntoView({ behavior: 'smooth', block: 'center' }); }}
      style="width:34px;height:34px;display:inline-flex;align-items:center;justify-content:center;background:transparent;border:none;cursor:pointer;border-radius:4px;"
      title="Add source"
    >
      <Icon name="plus" size={16} color={T.cyan} />
    </button>
  </div>

  <!-- Status summary -->
  <div style="display:flex;gap:12px;padding:8px 12px;border-bottom:1px solid {T.bd0};background:{T.bg1};font:10px/1 {T.mono};color:{T.ink2};flex-shrink:0;">
    <span><span style="color:{T.green};">● </span>ok {okCount}</span>
    <span><span style="color:{T.amber};">● </span>stale {staleCount}</span>
    <span><span style="color:{T.red};">● </span>err {errCount}</span>
    <span style="flex:1;"></span>
    <span style="color:{T.ink3};">{sources.length} feeds</span>
  </div>

  <div style="flex:1;overflow-y:auto;">
    <!-- Add source card -->
    <div style="margin:12px 10px;padding:10px 12px;background:{T.bg1};border:1px dashed {T.bd2};border-radius:3px;font:11px/1.4 {T.mono};color:{T.ink1};">
      <div style="display:flex;align-items:center;gap:8px;margin-bottom:8px;">
        <Icon name="plus" size={13} color={T.cyan} />
        <span style="color:{T.ink0};letter-spacing:0.4px;">ADD SOURCE</span>
      </div>
      <div style="display:flex;background:{T.bg0};border:1px solid {T.bd1};border-radius:3px;margin-bottom:8px;">
        <div style="padding:6px 8px;font:11px/1 {T.mono};color:{T.cyan};border-right:1px solid {T.bd1};">$</div>
        <input
          bind:this={addInputEl}
          bind:value={addUrl}
          placeholder="https://example.com/feed.xml"
          onkeydown={(e) => { if (e.key === 'Enter') submitAddSource(); }}
          style="flex:1;padding:6px 8px;font:11px/1 {T.mono};color:{T.ink0};"
        />
      </div>
      <div style="display:flex;gap:6px;">
        <select bind:value={addGroup} style="flex:1;background:{T.bg0};color:{T.ink1};border:1px solid {T.bd1};border-radius:3px;padding:6px 8px;font:11px/1 {T.mono};">
          {#each groups as g}
            <option value={g.id}>group: {g.name}</option>
          {/each}
          <option value="__new__">+ create new group</option>
        </select>
        <button
          onclick={submitAddSource}
          style="padding:0 14px;background:{T.cyan};color:{T.bg0};border:none;border-radius:3px;font:600 11px/1 {T.mono};letter-spacing:0.4px;cursor:pointer;"
        >+ ADD</button>
      </div>
      {#if addGroup === '__new__'}
        <div style="margin-top:4px;">
          <input
            bind:value={newGroupName}
            placeholder="new group name"
            onkeydown={(e) => { if (e.key === 'Enter') submitAddSource(); }}
            style="width:100%;padding:6px 8px;background:{T.bg0};color:{T.ink0};border:1px solid {T.cyan};border-radius:3px;font:11px/1 {T.mono};box-sizing:border-box;outline:none;"
          />
        </div>
      {/if}
    </div>

    <!-- Hint -->
    <div style="padding:4px 12px 8px;font:10px/1.4 {T.mono};color:{T.ink3};text-align:center;">
      tap to view · hold for actions
    </div>

    <!-- Hacker News -->
    {#if byKind.hn.length > 0}
      <div style="display:flex;align-items:center;justify-content:space-between;padding:8px 12px;border-top:1px solid {T.bd0};border-bottom:1px solid {T.bd0};background:{T.bg1};font:10px/1 {T.mono};color:{T.ink2};letter-spacing:0.8px;text-transform:uppercase;">
        <span>hacker news</span>
        <span style="color:{T.orange};">{byKind.hn.length}</span>
      </div>
      {#each byKind.hn as s}
        {@const spark = sparkData(s.id)}
        <div
          role="button"
          tabindex="0"
          ontouchstart={(e) => startPress(e, s.id)}
          ontouchmove={handleTouchMove}
          ontouchend={(e) => handleTouchEnd(e, s.id)}
          ontouchcancel={cancelPress}
          onclick={() => onSourceSelect(s.id)}
          oncontextmenu={(e) => handleContextMenu(e, s.id)}
          onkeydown={(e) => { if (e.key === 'Enter') onSourceSelect(s.id); }}
          style="display:grid;grid-template-columns:auto 1fr auto;gap:10px;padding:10px 12px;border-bottom:1px solid {T.bd0};cursor:pointer;align-items:center;user-select:none;-webkit-user-select:none;"
        >
          <div style="width:30px;height:30px;display:flex;align-items:center;justify-content:center;background:{T.bg2};border:1px solid {T.bd1};border-radius:3px;">
            <SourceGlyph kind={s.kind} size={12} />
          </div>
          <div style="min-width:0;">
            <div style="display:flex;align-items:center;gap:6px;font:13px/1.2 {T.mono};color:{T.ink0};">
              <StatusDot status={s.status} />
              <span style="overflow:hidden;text-overflow:ellipsis;white-space:nowrap;">{s.name}</span>
            </div>
            <div style="margin-top:3px;font:10px/1 {T.mono};color:{T.ink2};display:flex;align-items:center;gap:6px;">
              <span style="color:{T.ink3};overflow:hidden;text-overflow:ellipsis;white-space:nowrap;max-width:100px;">{s.host}</span>
              <span style="color:{T.ink3};">·</span>
              <span><span style="color:{T.cyan};">{s.unread}</span>{#if s.items > 0}<span style="color:{T.ink3};">/{s.items}</span>{/if}</span>
              <span style="color:{T.ink3};">·</span>
              <span>{s.lastSync}</span>
              {#if s.latencyMs > 0}
                <span style="color:{T.ink3};">·</span>
                <span style="color:{s.latencyMs > 250 ? T.amber : T.ink2};">{s.latencyMs}ms</span>
              {/if}
            </div>
          </div>
          <Sparkline data={spark} w={56} h={20} color={s.status === 'error' ? T.red : s.status === 'stale' ? T.amber : T.cyan} />
        </div>
      {/each}
    {/if}

    <!-- Reddit -->
    {#if byKind.reddit.length > 0}
      <div style="display:flex;align-items:center;justify-content:space-between;padding:8px 12px;border-top:1px solid {T.bd0};border-bottom:1px solid {T.bd0};background:{T.bg1};font:10px/1 {T.mono};color:{T.ink2};letter-spacing:0.8px;text-transform:uppercase;">
        <span>reddit</span>
        <span style="color:{T.cyan};">{byKind.reddit.length}</span>
      </div>
      {#each byKind.reddit as s}
        {@const spark = sparkData(s.id)}
        <div
          role="button"
          tabindex="0"
          ontouchstart={(e) => startPress(e, s.id)}
          ontouchmove={handleTouchMove}
          ontouchend={(e) => handleTouchEnd(e, s.id)}
          ontouchcancel={cancelPress}
          onclick={() => onSourceSelect(s.id)}
          oncontextmenu={(e) => handleContextMenu(e, s.id)}
          onkeydown={(e) => { if (e.key === 'Enter') onSourceSelect(s.id); }}
          style="display:grid;grid-template-columns:auto 1fr auto;gap:10px;padding:10px 12px;border-bottom:1px solid {T.bd0};cursor:pointer;align-items:center;user-select:none;-webkit-user-select:none;"
        >
          <div style="width:30px;height:30px;display:flex;align-items:center;justify-content:center;background:{T.bg2};border:1px solid {T.bd1};border-radius:3px;">
            <SourceGlyph kind={s.kind} size={12} />
          </div>
          <div style="min-width:0;">
            <div style="display:flex;align-items:center;gap:6px;font:13px/1.2 {T.mono};color:{T.ink0};">
              <StatusDot status={s.status} />
              <span style="overflow:hidden;text-overflow:ellipsis;white-space:nowrap;">{s.name}</span>
            </div>
            <div style="margin-top:3px;font:10px/1 {T.mono};color:{T.ink2};display:flex;align-items:center;gap:6px;">
              <span style="color:{T.ink3};">{s.host}</span>
              <span style="color:{T.ink3};">·</span>
              <span><span style="color:{T.cyan};">{s.unread}</span>{#if s.items > 0}<span style="color:{T.ink3};">/{s.items}</span>{/if}</span>
              <span style="color:{T.ink3};">·</span>
              <span>{s.lastSync}</span>
              {#if s.latencyMs > 0}
                <span style="color:{T.ink3};">·</span>
                <span style="color:{s.latencyMs > 250 ? T.amber : T.ink2};">{s.latencyMs}ms</span>
              {/if}
            </div>
          </div>
          <Sparkline data={spark} w={56} h={20} color={T.cyan} />
        </div>
      {/each}
    {/if}

    <!-- RSS -->
    {#if byKind.rss.length > 0}
      <div style="display:flex;align-items:center;justify-content:space-between;padding:8px 12px;border-top:1px solid {T.bd0};border-bottom:1px solid {T.bd0};background:{T.bg1};font:10px/1 {T.mono};color:{T.ink2};letter-spacing:0.8px;text-transform:uppercase;">
        <span>rss / atom</span>
        <span style="color:{T.amber};">{byKind.rss.length}</span>
      </div>
      {#each byKind.rss as s}
        {@const spark = sparkData(s.id)}
        <div
          role="button"
          tabindex="0"
          ontouchstart={(e) => startPress(e, s.id)}
          ontouchmove={handleTouchMove}
          ontouchend={(e) => handleTouchEnd(e, s.id)}
          ontouchcancel={cancelPress}
          onclick={() => onSourceSelect(s.id)}
          oncontextmenu={(e) => handleContextMenu(e, s.id)}
          onkeydown={(e) => { if (e.key === 'Enter') onSourceSelect(s.id); }}
          style="display:grid;grid-template-columns:auto 1fr auto;gap:10px;padding:10px 12px;border-bottom:1px solid {T.bd0};cursor:pointer;align-items:center;user-select:none;-webkit-user-select:none;"
        >
          <div style="width:30px;height:30px;display:flex;align-items:center;justify-content:center;background:{T.bg2};border:1px solid {T.bd1};border-radius:3px;">
            <SourceGlyph kind={s.kind} size={12} />
          </div>
          <div style="min-width:0;">
            <div style="display:flex;align-items:center;gap:6px;font:13px/1.2 {T.mono};color:{T.ink0};">
              <StatusDot status={s.status} />
              <span style="overflow:hidden;text-overflow:ellipsis;white-space:nowrap;">{s.name}</span>
            </div>
            <div style="margin-top:3px;font:10px/1 {T.mono};color:{T.ink2};display:flex;align-items:center;gap:6px;flex-wrap:wrap;">
              <span style="color:{T.ink3};">{s.host}</span>
              <span style="color:{T.ink3};">·</span>
              <span><span style="color:{T.cyan};">{s.unread}</span>{#if s.items > 0}<span style="color:{T.ink3};">/{s.items}</span>{/if}</span>
              <span style="color:{T.ink3};">·</span>
              <span>{s.lastSync}</span>
              {#if s.latencyMs > 0}
                <span style="color:{s.latencyMs > 250 ? T.amber : T.ink2};">{s.latencyMs}ms</span>
              {/if}
              {#if s.status === 'error'}
                <span style="color:{T.red};">sync error</span>
              {:else if s.status === 'stale'}
                <span style="color:{T.amber};">stale</span>
              {/if}
            </div>
          </div>
          <Sparkline data={spark} w={56} h={20} color={s.status === 'error' ? T.red : s.status === 'stale' ? T.amber : T.amber} />
        </div>
      {/each}
    {/if}
    <div style="height:12px;"></div>
  </div>

  <PulseBottomNav active={tab} onChange={onTabChange} />
</div>

<!-- Long-press action sheet -->
{#if actionSheet && actionSource}
  <div
    role="button"
    tabindex="-1"
    onclick={() => { actionSheet = null; }}
    onkeydown={(e) => { if (e.key === 'Escape') actionSheet = null; }}
    style="position:fixed;inset:0;background:rgba(0,0,0,0.6);z-index:100;display:flex;align-items:flex-end;"
  >
    <div
      role="button"
      tabindex="-1"
      onclick={(e) => e.stopPropagation()}
      onkeydown={() => {}}
      style="width:100%;background:{T.bg2};border-top:1px solid {T.bd1};padding:0 0 24px;"
    >
      <!-- Source info header -->
      <div style="display:flex;align-items:center;gap:10px;padding:14px 16px;border-bottom:1px solid {T.bd0};">
        <div style="width:32px;height:32px;display:flex;align-items:center;justify-content:center;background:{T.bg1};border:1px solid {T.bd1};border-radius:4px;">
          <SourceGlyph kind={actionSource.kind} size={14} />
        </div>
        <div>
          <div style="font:13px/1 {T.mono};color:{T.ink0};">{actionSource.name}</div>
          <div style="margin-top:4px;font:10px/1 {T.mono};color:{T.ink3};">{actionSource.host}</div>
        </div>
        <span style="flex:1;"></span>
        <StatusDot status={actionSource.status} />
      </div>
      <!-- Actions -->
      {#each [
        { icon: 'list',  label: 'View feed',     action: () => { onSourceSelect(actionSheet!); actionSheet = null; } },
        { icon: 'sync',  label: 'Refresh now',   action: () => { storeSyncSource(actionSheet!); actionSheet = null; } },
        { icon: 'edit',  label: 'Edit source',   action: () => openEditSheet(actionSheet!) },
        { icon: 'star',  label: 'Mark all read', action: () => { markSourceRead(actionSheet!); actionSheet = null; } },
        { icon: 'trash', label: 'Remove source', action: () => { removeSource(actionSheet!); actionSheet = null; } },
      ] as act}
        <button
          onclick={act.action}
          style="display:flex;align-items:center;gap:14px;width:100%;padding:14px 16px;background:transparent;border:none;border-bottom:1px solid {T.bd0};font:13px/1 {T.mono};color:{act.label === 'Remove source' ? T.red : T.ink0};cursor:pointer;text-align:left;-webkit-tap-highlight-color:transparent;"
        >
          <Icon name={act.icon} size={16} color={act.label === 'Remove source' ? T.red : act.label === 'Edit source' ? T.cyan : T.ink2} />
          {act.label}
        </button>
      {/each}
      <button
        onclick={() => { actionSheet = null; }}
        style="display:flex;align-items:center;justify-content:center;width:100%;padding:14px 16px;background:transparent;border:none;font:12px/1 {T.mono};color:{T.ink2};cursor:pointer;"
      >cancel</button>
    </div>
  </div>
{/if}

<!-- Edit source sheet -->
{#if editingSourceId}
  <div style="position:fixed;inset:0;z-index:60;display:flex;flex-direction:column;justify-content:flex-end;">
    <div
      role="presentation"
      onclick={() => { editingSourceId = null; }}
      style="position:absolute;inset:0;background:rgba(0,0,0,0.5);"
    ></div>
    <div style="position:relative;background:{T.bg1};border-top-left-radius:12px;border-top-right-radius:12px;padding:16px;display:flex;flex-direction:column;gap:12px;padding-bottom:max(16px, env(safe-area-inset-bottom));">
      <div style="font:11px/1 {T.mono};color:{T.ink2};letter-spacing:0.5px;text-transform:uppercase;margin-bottom:4px;">edit source</div>

      <div style="display:flex;flex-direction:column;gap:6px;">
        <label for="edit-url" style="font:10px/1 {T.mono};color:{T.ink3};">URL</label>
        <input
          id="edit-url"
          bind:value={editUrl}
          placeholder="https://example.com/feed.xml"
          style="width:100%;padding:10px;background:{T.bg0};border:1px solid {T.bd1};border-radius:3px;font:12px/1 {T.mono};color:{T.ink0};outline:none;box-sizing:border-box;"
          oninput={() => { editKind = inferSourceMeta(editUrl).kind; }}
        />
      </div>

      <div style="display:flex;flex-direction:column;gap:6px;">
        <label for="edit-name" style="font:10px/1 {T.mono};color:{T.ink3};">NAME</label>
        <input
          id="edit-name"
          bind:value={editName}
          placeholder="Display name"
          style="width:100%;padding:10px;background:{T.bg0};border:1px solid {T.bd1};border-radius:3px;font:12px/1 {T.mono};color:{T.ink0};outline:none;box-sizing:border-box;"
        />
      </div>

      <div style="display:flex;gap:8px;">
        <div style="flex:1;display:flex;flex-direction:column;gap:6px;">
          <label style="font:10px/1 {T.mono};color:{T.ink3};">TYPE</label>
          <div style="display:flex;gap:3px;background:{T.bg0};border:1px solid {T.bd1};border-radius:3px;padding:2px;">
            {#each (['rss', 'hn', 'reddit'] as const) as k}
              <button
                onclick={() => editKind = k}
                style="flex:1;padding:6px 4px;border:none;border-radius:2px;cursor:pointer;font:9px/1 {T.mono};text-transform:uppercase;background:{editKind===k ? T.bg3 : 'transparent'};color:{editKind===k ? T.cyan : T.ink2};"
              >{k}</button>
            {/each}
          </div>
        </div>
        <div style="flex:1;display:flex;flex-direction:column;gap:6px;">
          <label style="font:10px/1 {T.mono};color:{T.ink3};">GROUP</label>
          <select
            bind:value={editGroup}
            style="width:100%;padding:8px;background:{T.bg0};border:1px solid {T.bd1};border-radius:3px;font:12px/1 {T.mono};color:{T.ink0};cursor:pointer;"
          >
            {#each groups as g}<option value={g.id}>{g.name}</option>{/each}
          </select>
        </div>
      </div>

      <div style="display:flex;gap:8px;margin-top:4px;">
        <button
          onclick={() => { editingSourceId = null; }}
          style="flex:1;padding:12px;background:transparent;border:1px solid {T.bd1};border-radius:4px;font:12px/1 {T.mono};color:{T.ink2};cursor:pointer;"
        >cancel</button>
        <button
          onclick={submitEditSource}
          style="flex:2;padding:12px;background:{T.cyan};border:none;border-radius:4px;font:12px/1 {T.mono};color:{T.bg0};cursor:pointer;font-weight:600;"
        >save changes</button>
      </div>
    </div>
  </div>
{/if}
