<script lang="ts">
  import { T } from '$lib/tokens';
  import { sources } from '$lib/stores/data.svelte';
  import { doSync as storeSync, syncState } from '$lib/stores/sync.svelte';
  import PulseBottomNav from '$lib/components/PulseBottomNav.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import SourceExplorer from '$lib/components/SourceExplorer.svelte';

  let { tab, onTabChange, onSourceSelect }: {
    tab: string;
    onTabChange: (id: string) => void;
    onSourceSelect: (sourceId: string) => void;
  } = $props();

  async function doSync() {
    await storeSync();
  }
</script>

<div style="display:flex;flex-direction:column;height:100%;background:{T.bg0};color:{T.ink0};">
  <!-- Top bar -->
  <div style="height:44px;display:flex;align-items:center;padding:0 10px;border-bottom:1px solid {T.bd0};background:{T.bg1};flex-shrink:0;gap:8px;">
    <span style="font:12px/1 {T.mono};color:{T.ink0};letter-spacing:0.5px;flex:1;">
      sources <span style="color:{T.ink3};">· {sources.length}</span>
    </span>
    <button onclick={doSync} style="width:34px;height:34px;display:inline-flex;align-items:center;justify-content:center;background:transparent;border:none;cursor:pointer;border-radius:4px;">
      <span class={syncState.syncing ? 'syncing' : ''}>
        <Icon name="sync" size={16} color={syncState.syncing ? T.cyan : T.ink1} />
      </span>
    </button>
    <button
      onclick={() => { document.querySelector('.add-source-target')?.scrollIntoView({ behavior: 'smooth', block: 'center' }); }}
      style="width:34px;height:34px;display:inline-flex;align-items:center;justify-content:center;background:transparent;border:none;cursor:pointer;border-radius:4px;"
      title="Add source"
    >
      <Icon name="plus" size={16} color={T.cyan} />
    </button>
  </div>

  <div style="flex:1;overflow:auto;">
    <SourceExplorer {onSourceSelect} onSync={doSync} />
  </div>

  <PulseBottomNav active={tab} onChange={onTabChange} />
</div>
