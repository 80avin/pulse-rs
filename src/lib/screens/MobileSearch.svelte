<script lang="ts">
  import { T } from '$lib/tokens';
  import PulseBottomNav from '$lib/components/PulseBottomNav.svelte';
  import SearchView from '$lib/components/SearchView.svelte';

  let { tab, onTabChange, onOpen }: {
    tab: string;
    onTabChange: (id: string) => void;
    onOpen: (id: string, ids: string[]) => void;
  } = $props();
</script>

<div class="flex flex-col h-full bg-bg-0 text-ink-0">
  <SearchView onItemOpen={(id, ids) => onOpen(id, ids)} />
  <PulseBottomNav active={tab} onChange={onTabChange} />
</div>
