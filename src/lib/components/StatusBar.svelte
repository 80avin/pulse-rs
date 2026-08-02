<script lang="ts">
  let {
    density,
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

<div class="flex items-center shrink-0 border-t border-bd-0 bg-bg-1 text-ink-2 h-6 px-2.5 gap-3.5 text-[10px] leading-none font-mono">
  <span class="text-bg-0 bg-cyan uppercase px-1.5 py-0.75 rounded-sm font-semibold text-[9px] leading-none font-mono tracking-[0.6px]">{density}</span>
  <span><span class="text-ink-3">group:</span> {activeGroupLabel}</span>
  {#if activeSource}<span class="text-ink-4">·</span><span><span class="text-ink-3">src:</span> {activeSourceName}</span>{/if}
  <span class="text-ink-4">·</span>
  <span title="{itemCount} items in view · {totalCount} total matching filter"><span class="text-ink-3">items:</span> <span class="text-ink-0">{itemCount}</span> / {totalCount}</span>
  <span class="text-ink-4">·</span>
  <span title="{unreadCount} unread in current view"><span class="text-ink-3">unread:</span> <span style="color:{unreadCount > 0 ? 'var(--color-cyan)' : 'var(--color-ink-3)'};">{unreadCount}</span></span>
  <span class="text-ink-4">·</span>
  <span title="{taggedCount} items with at least one AI tag in current view"><span class="text-ink-3">tagged:</span> <span class="text-amber">{taggedCount}</span></span>
  {#if searchQuery}<span class="text-ink-4">·</span><span class="text-amber">"{searchQuery}"</span>{/if}
  <span class="flex-1"></span>
  <span title="last sync: {syncState.lastSyncAt}{syncState.lastNewCount > 0 ? ` · +${syncState.lastNewCount} new` : ''}">
    <span style="color:{syncing ? 'var(--color-cyan)' : 'var(--color-green)'};">●</span>
    <span class="text-ink-3"> sync</span>
    <span class="text-ink-1"> {syncState.lastSyncAt}</span>
    {#if syncState.lastNewCount > 0}<span class="text-cyan"> +{syncState.lastNewCount}</span>{/if}
  </span>
  <span class="text-ink-4">·</span>
  <button onclick={onToggleCheatsheet} class="bg-transparent border-none cursor-pointer text-ink-3 p-0 text-[10px] leading-none font-mono" title="keyboard shortcuts (?)" aria-label="Keyboard shortcuts (?)">?</button>
</div>
