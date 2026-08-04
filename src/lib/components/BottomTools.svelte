<script lang="ts">
  import Icon from './Icon.svelte';
  import GhostButton from './shared/GhostButton.svelte';

  let {
    showSources, showSettings,
    syncing, syncState,
    onToggleSources, onToggleSettings,
  }: {
    showSources: boolean; showSettings: boolean;
    syncing: boolean;
    syncState: { lastSyncAt: string; lastNewCount: number; syncing: boolean };
    onToggleSources: () => void;
    onToggleSettings: () => void;
  } = $props();
</script>

<div class="border-t border-bd-0 py-1" aria-label="Tools">
  <GhostButton
    onclick={onToggleSources}
    class="flex items-center gap-1.5 w-full text-left p-[5px_12px] text-[12px] leading-none" style="background:{showSources ? 'rgba(78,205,214,0.06)' : 'transparent'};border-left:2px solid {showSources ? 'var(--color-cyan)' : 'transparent'};color:{showSources ? 'var(--color-cyan)' : 'var(--color-ink-2)'};"
    title="Sources (r)"
    ariaLabel="Sources (r)"
  >
    <Icon name="rss" size={12} color={showSources ? 'var(--color-cyan)' : 'var(--color-ink-2)'} />
    <span>Sources</span>
  </GhostButton>
  <GhostButton
    onclick={onToggleSettings}
    class="flex items-center gap-1.5 w-full text-left p-[5px_12px] text-[12px] leading-none" style="background:{showSettings ? 'rgba(78,205,214,0.06)' : 'transparent'};border-left:2px solid {showSettings ? 'var(--color-cyan)' : 'transparent'};color:{showSettings ? 'var(--color-cyan)' : 'var(--color-ink-2)'};"
    title="Settings"
    ariaLabel="Settings"
  >
    <Icon name="cog" size={12} color={showSettings ? 'var(--color-cyan)' : 'var(--color-ink-2)'} />
    <span>Settings</span>
  </GhostButton>
  <div class="flex items-center gap-1.5 text-ink-3 p-1 px-3 text-[10px] leading-none font-mono" title={`Synced ${syncState.lastSyncAt}${syncState.lastNewCount > 0 ? ` · +${syncState.lastNewCount} new` : ''}`} aria-label={`Sync status: ${syncState.lastSyncAt}, ${syncState.lastNewCount > 0 ? syncState.lastNewCount + ' new items' : 'no new items'}`}>
    <span style="color:{syncing ? 'var(--color-cyan)' : 'var(--color-green)'};">●</span>
    <span>sync {syncState.lastSyncAt}</span>
  </div>
</div>
