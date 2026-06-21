<script lang="ts">
  import { T } from '$lib/tokens';
  import { aiStatus } from '$lib/stores/ai.svelte';
  import PulseBottomNav from '$lib/components/PulseBottomNav.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import AiPanelContent from '$lib/components/AiPanelContent.svelte';

  let { tab, onTabChange, onTagFilter, onItemOpen, onSourceFilter }: {
    tab: string;
    onTabChange: (id: string) => void;
    onTagFilter?: (tag: string) => void;
    onItemOpen?: (id: string, ids: string[]) => void;
    onSourceFilter?: (sourceId: string) => void;
  } = $props();

  const modeColor = $derived(
    aiStatus.taggingMode === 'loading' ? T.ink3 :
    aiStatus.taggingMode === 'none'    ? T.amber : T.cyan
  );
  const modeLabel = $derived(
    aiStatus.taggingMode === 'loading' ? 'loading…' : aiStatus.taggingMode
  );
</script>

<div class="flex flex-col h-full bg-bg-0 text-ink-0">
  <!-- Header -->
  <div class="h-[44px] flex items-center px-3.5 border-b border-b-bd-0 bg-bg-1 shrink-0 gap-2.5">
    <Icon name="cpu" size={15} color={T.cyan} />
    <span class="text-[12px] leading-none font-mono text-ink-0 tracking-[0.5px]">ai signal</span>
    <span class="flex-1"></span>
    <span class="text-[10px] leading-none font-mono" style="color:{modeColor};">{modeLabel}</span>
  </div>

  <div class="flex-1 overflow-y-auto p-3 px-2.5">
    <AiPanelContent compact={false} {onTagFilter} onItemClick={onItemOpen} {onSourceFilter} />
    <div class="h-3"></div>
  </div>

  <PulseBottomNav active={tab} onChange={onTabChange} />
</div>
