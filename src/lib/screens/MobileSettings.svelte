<script lang="ts">
  import { T } from '$lib/tokens';
  import PulseBottomNav from '$lib/components/PulseBottomNav.svelte';
  import SettingsPanelContent from '$lib/components/SettingsPanelContent.svelte';

  let { tab, onTabChange }: {
    tab: string;
    onTabChange: (id: string) => void;
  } = $props();
</script>

<div style="display:flex;flex-direction:column;height:100%;background:{T.bg0};color:{T.ink0};">
  <div style="height:44px;display:flex;align-items:center;padding:0 14px;border-bottom:1px solid {T.bd0};background:{T.bg1};flex-shrink:0;">
    <span style="font:12px/1 {T.mono};color:{T.ink0};letter-spacing:0.5px;">settings</span>
  </div>

  <div style="flex:1;overflow-y:auto;padding:12px 10px;display:flex;flex-direction:column;gap:10px;">
    <SettingsPanelContent showShortcuts={false} />
    <div style="height:12px;"></div>
  </div>

  <PulseBottomNav active={tab} onChange={onTabChange} />
</div>
