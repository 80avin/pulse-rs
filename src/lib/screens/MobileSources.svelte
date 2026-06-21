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

<div class="flex flex-col h-full bg-bg-0 text-ink-0">
  <!-- Top bar -->
  <div class="h-[44px] flex items-center px-[10px] border-b border-b-bd-0 bg-bg-1 shrink-0 gap-2">
    <span class="text-[12px] leading-none font-mono text-ink-0 tracking-[0.5px] flex-1">
      sources <span class="text-ink-3">· {sources.length}</span>
    </span>
    <button onclick={doSync} class="inline-flex items-center justify-center bg-transparent border-none cursor-pointer rounded w-8.5 h-8.5">
      <span class={syncState.syncing ? 'syncing' : ''}>
        <Icon name="sync" size={16} color={syncState.syncing ? T.cyan : T.ink1} />
      </span>
    </button>
    <button
      onclick={() => { document.querySelector('.add-source-target')?.scrollIntoView({ behavior: 'smooth', block: 'center' }); }}
      class="inline-flex items-center justify-center bg-transparent border-none cursor-pointer rounded w-8.5 h-8.5"
      title="Add source"
    >
      <Icon name="plus" size={16} color={T.cyan} />
    </button>
  </div>

  <div class="flex-1 overflow-auto">
    <SourceExplorer {onSourceSelect} onSync={doSync} />
  </div>

  <PulseBottomNav active={tab} onChange={onTabChange} />
</div>
