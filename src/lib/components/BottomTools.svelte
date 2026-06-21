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

<div class="border-t border-bd-0 py-1" aria-label="Tools">
  <button
    onclick={onToggleAI}
    class="flex items-center gap-1.5 w-full border-none cursor-pointer text-left p-[5px_12px] text-[10px] leading-none font-mono" style="background:{showAI ? 'rgba(78,205,214,0.06)' : 'transparent'};border-left:2px solid {showAI ? 'var(--color-cyan)' : 'transparent'};color:{showAI ? 'var(--color-cyan)' : 'var(--color-ink-2)'};"
    onmouseenter={(e) => { if(!showAI)(e.currentTarget as HTMLElement).style.background = 'var(--color-bg-2)'; }}
    onmouseleave={(e) => { if(!showAI)(e.currentTarget as HTMLElement).style.background = 'transparent'; }}
    title={taggingProgress.active ? `Tagging ${taggingProgress.tagged}/${taggingProgress.total}…` : 'AI Signal (a)'}
    aria-label={`AI Signal${taggingProgress.active ? ` - tagging ${taggingProgress.tagged} of ${taggingProgress.total}` : ''}`}
  >
    <Icon name="cpu" size={12} color={taggingProgress.active ? 'var(--color-amber)' : (showAI ? 'var(--color-cyan)' : 'var(--color-ink-2)')} />
    <span class="flex-1">AI Signal</span>
    {#if taggingProgress.active}<span class="shrink-0 rounded-full bg-amber w-1.25 h-1.25" aria-hidden="true"></span>{/if}
  </button>
  <button
    onclick={onToggleSources}
    class="flex items-center gap-1.5 w-full border-none cursor-pointer text-left p-[5px_12px] text-[10px] leading-none font-mono" style="background:{showSources ? 'rgba(78,205,214,0.06)' : 'transparent'};border-left:2px solid {showSources ? 'var(--color-cyan)' : 'transparent'};color:{showSources ? 'var(--color-cyan)' : 'var(--color-ink-2)'};"
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
    class="flex items-center gap-1.5 w-full border-none cursor-pointer text-left p-[5px_12px] text-[10px] leading-none font-mono" style="background:{showSettings ? 'rgba(78,205,214,0.06)' : 'transparent'};border-left:2px solid {showSettings ? 'var(--color-cyan)' : 'transparent'};color:{showSettings ? 'var(--color-cyan)' : 'var(--color-ink-2)'};"
    onmouseenter={(e) => { if(!showSettings)(e.currentTarget as HTMLElement).style.background = 'var(--color-bg-2)'; }}
    onmouseleave={(e) => { if(!showSettings)(e.currentTarget as HTMLElement).style.background = 'transparent'; }}
    title="Settings"
    aria-label="Settings"
  >
    <Icon name="cog" size={12} color={showSettings ? 'var(--color-cyan)' : 'var(--color-ink-2)'} />
    <span>Settings</span>
  </button>
  <div class="flex items-center gap-1.5 text-ink-3 p-1 px-3 text-[10px] leading-none font-mono" title={`Synced ${syncState.lastSyncAt}${syncState.lastNewCount > 0 ? ` · +${syncState.lastNewCount} new` : ''}`} aria-label={`Sync status: ${syncState.lastSyncAt}, ${syncState.lastNewCount > 0 ? syncState.lastNewCount + ' new items' : 'no new items'}`}>
    <span style="color:{syncing ? 'var(--color-cyan)' : 'var(--color-green)'};">●</span>
    <span>sync {syncState.lastSyncAt}</span>
  </div>
</div>
