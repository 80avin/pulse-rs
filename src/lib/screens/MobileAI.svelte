<script lang="ts">
  import { T } from '$lib/tokens';
  import { aiStatus } from '$lib/store.svelte';
  import PulseBottomNav from '$lib/components/PulseBottomNav.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import AiPanelContent from '$lib/components/AiPanelContent.svelte';

  let { tab, onTabChange, onTagFilter }: {
    tab: string;
    onTabChange: (id: string) => void;
    onTagFilter?: (tag: string) => void;
  } = $props();

  const modeColor = $derived(
    aiStatus.taggingMode === 'loading' ? T.ink3 :
    aiStatus.taggingMode === 'none'    ? T.amber : T.cyan
  );
  const modeLabel = $derived(
    aiStatus.taggingMode === 'loading' ? 'loading…' : aiStatus.taggingMode
  );
</script>

<div style="display:flex;flex-direction:column;height:100%;background:{T.bg0};color:{T.ink0};">
  <!-- Header -->
  <div style="height:44px;display:flex;align-items:center;padding:0 14px;border-bottom:1px solid {T.bd0};background:{T.bg1};flex-shrink:0;gap:10px;">
    <Icon name="cpu" size={15} color={T.cyan} />
    <span style="font:12px/1 {T.mono};color:{T.ink0};letter-spacing:0.5px;">ai signal</span>
    <span style="flex:1;"></span>
    <span style="font:10px/1 {T.mono};color:{modeColor};">{modeLabel}</span>
  </div>

  <div style="flex:1;overflow-y:auto;padding:12px 10px;">
    <AiPanelContent compact={false} {onTagFilter} />
    <div style="height:12px;"></div>
  </div>

  <PulseBottomNav active={tab} onChange={onTabChange} />
</div>
