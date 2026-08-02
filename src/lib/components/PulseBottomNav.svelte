<script lang="ts">
  import { T } from '$lib/tokens';
  import { groups, dbStats } from '$lib/stores/data.svelte';
  import Icon from './Icon.svelte';
  import { Toolbar } from 'bits-ui';

  let { active, onChange }: {
    active: string;
    onChange: (id: string) => void;
  } = $props();

  const totalUnread = $derived(groups.find(g => g.id === 'all')?.n ?? dbStats.unreadItems);

  const tabs = [
    { id: 'feed', label: 'feed',     icon: 'list'     },
    { id: 'sources',  label: 'sources',  icon: 'rss'      },
    { id: 'search',   label: 'search',   icon: 'search'   },
    { id: 'saved',    label: 'saved',    icon: 'bookmark' },
    { id: 'settings', label: 'settings', icon: 'cog'      },
  ] as const;
</script>

<Toolbar.Root loop={false} orientation="horizontal" class="flex flex-col shrink-0 border-t border-t-bd-1 bg-bg-1" style="padding-bottom:env(safe-area-inset-bottom,0px);">
  <div class="flex">
  {#each tabs as tab}
    {@const a = tab.id === active}
    <Toolbar.Button
      onclick={() => onChange(tab.id)}
      class="flex-1 flex flex-col items-center gap-1 bg-transparent border-none cursor-pointer tracking-[0.5px] relative min-h-13 pt-[10px] pb-[12px] text-[9px] leading-none font-mono"
      style="
        border-top:2px solid {a ? T.cyan : 'transparent'};
        color:{a ? T.cyan : T.ink2};
      "
    >
      <div class="relative">
        <Icon name={tab.icon} size={18}
          color={tab.id === 'saved' ? T.amber : (a ? T.cyan : T.ink2)} />
        {#if tab.id === 'feed' && totalUnread > 0}
          <span class="absolute top-[-5px] right-[-8px] px-1 py-[1px] rounded-[6px] min-w-[14px] text-center bg-cyan text-bg-0 font-semibold text-[8px] leading-none font-mono">{totalUnread > 99 ? '99+' : totalUnread}</span>
        {/if}
      </div>
      <span>{tab.label}</span>
    </Toolbar.Button>
  {/each}
  </div>
</Toolbar.Root>
