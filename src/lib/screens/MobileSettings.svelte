<script lang="ts">
  import { T } from '$lib/tokens';
  import PulseBottomNav from '$lib/components/PulseBottomNav.svelte';
  import SettingsPanelContent from '$lib/components/SettingsPanelContent.svelte';

  let { tab, onTabChange }: {
    tab: string;
    onTabChange: (id: string) => void;
  } = $props();
</script>

<div class="flex flex-col h-full bg-bg-0 text-ink-0">
  <div class="h-[44px] flex items-center px-3.5 border-b border-b-bd-0 bg-bg-1 shrink-0">
    <span class="text-[12px] leading-none font-mono text-ink-0 tracking-[0.5px]">settings</span>
  </div>

  <div class="flex-1 overflow-y-auto flex flex-col gap-2.5 p-3 px-2.5">
    <SettingsPanelContent showShortcuts={false} />
    <div class="h-3"></div>
  </div>

  <PulseBottomNav active={tab} onChange={onTabChange} />
</div>
