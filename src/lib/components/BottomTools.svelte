<script lang="ts">
  import Icon from './Icon.svelte';

  let {
    showAI, showSources, showSettings,
    syncing, syncState, taggingProgress,
    onToggleAI, onToggleSources, onToggleSettings,
  }: {
    showAI: boolean; showSources: boolean; showSettings: boolean;
    syncing: boolean;
    syncState: { lastSyncAt: string; lastNewCount: number; syncing: boolean };
    taggingProgress: { active: boolean; tagged: number; total: number };
    onToggleAI: () => void;
    onToggleSources: () => void;
    onToggleSettings: () => void;
  } = $props();
</script>

<div style="border-top:1px solid var(--color-bd-0);padding:4px 0;" aria-label="Tools">
  <button
    onclick={onToggleAI}
    style="display:flex;align-items:center;gap:6px;padding:5px 12px;width:100%;background:{showAI ? 'rgba(78,205,214,0.06)' : 'transparent'};border:none;border-left:2px solid {showAI ? 'var(--color-cyan)' : 'transparent'};color:{showAI ? 'var(--color-cyan)' : 'var(--color-ink-2)'};cursor:pointer;text-align:left;font:10px/1 var(--font-mono);"
    onmouseenter={(e) => { if(!showAI)(e.currentTarget as HTMLElement).style.background = 'var(--color-bg-2)'; }}
    onmouseleave={(e) => { if(!showAI)(e.currentTarget as HTMLElement).style.background = 'transparent'; }}
    title={taggingProgress.active ? `Tagging ${taggingProgress.tagged}/${taggingProgress.total}…` : 'AI Signal (a)'}
    aria-label={`AI Signal${taggingProgress.active ? ` - tagging ${taggingProgress.tagged} of ${taggingProgress.total}` : ''}`}
  >
    <Icon name="cpu" size={12} color={taggingProgress.active ? 'var(--color-amber)' : (showAI ? 'var(--color-cyan)' : 'var(--color-ink-2)')} />
    <span style="flex:1;">AI Signal</span>
    {#if taggingProgress.active}<span style="width:5px;height:5px;border-radius:50%;background:var(--color-amber);flex-shrink:0;" aria-hidden="true"></span>{/if}
  </button>
  <button
    onclick={onToggleSources}
    style="display:flex;align-items:center;gap:6px;padding:5px 12px;width:100%;background:{showSources ? 'rgba(78,205,214,0.06)' : 'transparent'};border:none;border-left:2px solid {showSources ? 'var(--color-cyan)' : 'transparent'};color:{showSources ? 'var(--color-cyan)' : 'var(--color-ink-2)'};cursor:pointer;text-align:left;font:10px/1 var(--font-mono);"
    onmouseenter={(e) => { if(!showSources)(e.currentTarget as HTMLElement).style.background = 'var(--color-bg-2)'; }}
    onmouseleave={(e) => { if(!showSources)(e.currentTarget as HTMLElement).style.background = 'transparent'; }}
    title="Sources (r)"
    aria-label="Sources (r)"
  >
    <Icon name="rss" size={12} color={showSources ? 'var(--color-cyan)' : 'var(--color-ink-2)'} />
    <span>Sources</span>
  </button>
  <button
    onclick={onToggleSettings}
    style="display:flex;align-items:center;gap:6px;padding:5px 12px;width:100%;background:{showSettings ? 'rgba(78,205,214,0.06)' : 'transparent'};border:none;border-left:2px solid {showSettings ? 'var(--color-cyan)' : 'transparent'};color:{showSettings ? 'var(--color-cyan)' : 'var(--color-ink-2)'};cursor:pointer;text-align:left;font:10px/1 var(--font-mono);"
    onmouseenter={(e) => { if(!showSettings)(e.currentTarget as HTMLElement).style.background = 'var(--color-bg-2)'; }}
    onmouseleave={(e) => { if(!showSettings)(e.currentTarget as HTMLElement).style.background = 'transparent'; }}
    title="Settings"
    aria-label="Settings"
  >
    <Icon name="cog" size={12} color={showSettings ? 'var(--color-cyan)' : 'var(--color-ink-2)'} />
    <span>Settings</span>
  </button>
  <div style="display:flex;align-items:center;gap:6px;padding:4px 12px;font:10px/1 var(--font-mono);color:var(--color-ink-3);" title={`Synced ${syncState.lastSyncAt}${syncState.lastNewCount > 0 ? ` · +${syncState.lastNewCount} new` : ''}`} aria-label={`Sync status: ${syncState.lastSyncAt}, ${syncState.lastNewCount > 0 ? syncState.lastNewCount + ' new items' : 'no new items'}`}>
    <span style="color:{syncing ? 'var(--color-cyan)' : 'var(--color-green)'};">●</span>
    <span>sync {syncState.lastSyncAt}</span>
  </div>
</div>
