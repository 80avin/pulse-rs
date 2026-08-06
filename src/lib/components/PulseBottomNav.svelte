<script lang="ts">
  import { T } from '$lib/tokens';
  import { groups, dbStats } from '$lib/stores/data.svelte';
  import { TABS, type TabId } from '$lib/nav';
  import Icon from './Icon.svelte';
  import GhostButton from './shared/GhostButton.svelte';

  let { active, onChange }: {
    active: TabId | string;
    onChange: (id: TabId) => void;
  } = $props();

  const totalUnread = $derived(groups.find(g => g.id === 'all')?.n ?? dbStats.unreadItems);

  const META: Record<TabId, { label: string; icon: string }> = {
    feed: { label: 'feed', icon: 'list' },
    sources: { label: 'sources', icon: 'rss' },
    search: { label: 'home', icon: 'search' },
    saved: { label: 'saved', icon: 'bookmark' },
    settings: { label: 'settings', icon: 'cog' },
  };
</script>

<nav class="flex flex-col shrink-0 border-t border-t-bd-1 bg-bg-1" style="padding-bottom:env(safe-area-inset-bottom,0px);">
  <div class="flex">
  {#each TABS as tab}
    {@const m = META[tab]}
    {@const a = tab === active}
    <GhostButton
      onclick={() => onChange(tab)}
      ariaCurrent={a ? 'page' : undefined}
      class="flex-1 flex flex-col items-center gap-1 tracking-[0.5px] relative min-h-13 pt-[10px] pb-[12px] text-[9px] leading-none"
      style="
        border-top:2px solid {a ? T.cyan : 'transparent'};
        color:{a ? T.cyan : T.ink2};
      "
    >
      <div class="relative">
        <Icon name={m.icon} size={18}
          color={tab === 'saved' ? T.amber : (a ? T.cyan : T.ink2)} />
        {#if tab === 'feed' && totalUnread > 0}
          <span class="absolute top-[-5px] right-[-8px] px-1 py-[1px] rounded-[6px] min-w-[14px] text-center bg-cyan text-bg-0 font-semibold text-[8px] leading-none font-mono">{totalUnread > 99 ? '99+' : totalUnread}</span>
        {/if}
      </div>
      <span>{m.label}</span>
    </GhostButton>
  {/each}
  </div>
</nav>
