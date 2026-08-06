<script lang="ts">
  import { T } from '$lib/tokens';
  import Icon from '$lib/components/Icon.svelte';
  import SavedList from '$lib/components/SavedList.svelte';
  import type { FeedItem } from '$lib/types';

  let { onOpen }: {
    onOpen: (id: string, ids: string[], list: FeedItem[]) => void;
  } = $props();

  let savedCount = $state(0);
</script>

<div class="flex flex-col h-full bg-bg-0 text-ink-0">
  <div class="shrink-0 border-b border-bd-0 bg-bg-1 px-3.5 flex items-center gap-2.5" style="height:44px;">
    <Icon name="bookmark" size={15} color={T.amber} />
    <span class="text-[12px] leading-none font-mono text-ink-0 tracking-[0.5px]">saved</span>
    <span class="flex-1"></span>
    <span class="text-[10px] leading-none font-mono text-ink-3">{savedCount} saved</span>
  </div>

  <SavedList onOpen={(id, ids, list) => onOpen(id, ids, list)} onCountChange={(n) => { savedCount = n; }} />
</div>
