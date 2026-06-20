<script lang="ts">
  let {
    density,
    aiTagging,
    activeGroupLabel,
    activeSource,
    activeSourceName,
    itemCount,
    totalCount,
    unreadCount,
    taggedCount,
    searchQuery,
    syncing,
    syncState,
    onToggleCheatsheet,
  }: {
    density: string;
    aiTagging: boolean;
    activeGroupLabel: string;
    activeSource: string | null;
    activeSourceName: string | undefined;
    itemCount: number;
    totalCount: number;
    unreadCount: number;
    taggedCount: number;
    searchQuery: string;
    syncing: boolean;
    syncState: { lastSyncAt: string; lastNewCount: number };
    onToggleCheatsheet: () => void;
  } = $props();
</script>

<div style="height:24px;display:flex;align-items:center;padding:0 10px;gap:14px;border-top:1px solid var(--color-bd-0);background:var(--color-bg-1);flex-shrink:0;font:10px/1 var(--font-mono);color:var(--color-ink-2);">
  <span style="color:var(--color-bg-0);background:var(--color-cyan);padding:3px 6px;border-radius:2px;font:600 9px/1 var(--font-mono);letter-spacing:0.6px;text-transform:uppercase;">{density}</span>
  <span><span style="color:var(--color-ink-3);">group:</span> {activeGroupLabel}</span>
  {#if activeSource}<span style="color:var(--color-ink-4);">·</span><span><span style="color:var(--color-ink-3);">src:</span> {activeSourceName}</span>{/if}
  <span style="color:var(--color-ink-4);">·</span>
  <span title="{itemCount} items in view · {totalCount} total matching filter"><span style="color:var(--color-ink-3);">items:</span> <span style="color:var(--color-ink-0);">{itemCount}</span> / {totalCount}</span>
  <span style="color:var(--color-ink-4);">·</span>
  <span title="{unreadCount} unread in current view"><span style="color:var(--color-ink-3);">unread:</span> <span style="color:{unreadCount > 0 ? 'var(--color-cyan)' : 'var(--color-ink-3)'};">{unreadCount}</span></span>
  <span style="color:var(--color-ink-4);">·</span>
  <span title="{taggedCount} items with at least one AI tag in current view"><span style="color:var(--color-ink-3);">tagged:</span> <span style="color:var(--color-amber);">{taggedCount}</span></span>
  {#if searchQuery}<span style="color:var(--color-ink-4);">·</span><span style="color:var(--color-amber);">"{searchQuery}"</span>{/if}
  <span style="flex:1;"></span>
  <span title="last sync: {syncState.lastSyncAt}{syncState.lastNewCount > 0 ? ` · +${syncState.lastNewCount} new` : ''}">
    <span style="color:{syncing ? 'var(--color-cyan)' : 'var(--color-green)'};">●</span>
    <span style="color:var(--color-ink-3);"> sync</span>
    <span style="color:var(--color-ink-1);"> {syncState.lastSyncAt}</span>
    {#if syncState.lastNewCount > 0}<span style="color:var(--color-cyan);"> +{syncState.lastNewCount}</span>{/if}
  </span>
  <span style="color:var(--color-ink-4);">·</span>
  <span><span style="color:var(--color-ink-3);">ai</span> <span style="color:{aiTagging ? 'var(--color-amber)' : 'var(--color-ink-3)'};">{aiTagging ? 'on' : 'off'}</span></span>
  <span style="color:var(--color-ink-4);">·</span>
  <button onclick={onToggleCheatsheet} style="background:transparent;border:none;cursor:pointer;font:10px/1 var(--font-mono);color:var(--color-ink-3);padding:0;" title="keyboard shortcuts (?)" aria-label="Keyboard shortcuts (?)">?</button>
</div>
