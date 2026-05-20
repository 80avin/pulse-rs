<script lang="ts">
  import { T } from '$lib/tokens';
  import { items, taggingProgress } from '$lib/store.svelte';
  import Icon from './Icon.svelte';

  let { active, onChange }: {
    active: string;
    onChange: (id: string) => void;
  } = $props();

  const totalUnread = $derived(items.filter(i => !i.read).length);

  const tabs = [
    { id: 'timeline', label: 'feed',     icon: 'list'   },
    { id: 'sources',  label: 'sources',  icon: 'rss'    },
    { id: 'search',   label: 'search',   icon: 'search' },
    { id: 'ai',       label: 'ai',       icon: 'cpu'    },
    { id: 'settings', label: 'settings', icon: 'cog'    },
  ] as const;
</script>

<div style="display:flex;flex-direction:column;border-top:1px solid {T.bd1};background:{T.bg1};flex-shrink:0;padding-bottom:env(safe-area-inset-bottom,0px);">
  <div style="display:flex;">
  {#each tabs as tab}
    {@const a = tab.id === active}
    <button
      onclick={() => onChange(tab.id)}
      style="
        flex:1;padding:10px 0 12px;
        display:flex;flex-direction:column;align-items:center;gap:4px;
        background:transparent;border:none;
        border-top:2px solid {a ? T.cyan : 'transparent'};
        color:{a ? T.cyan : T.ink2};
        cursor:pointer;font:9px/1 {T.mono};letter-spacing:0.5px;
        position:relative;min-height:52px;
      "
    >
      <div style="position:relative;">
        <Icon name={tab.icon} size={18}
          color={tab.id === 'ai' && taggingProgress.active ? T.amber : (a ? T.cyan : T.ink2)} />
        {#if tab.id === 'timeline' && totalUnread > 0}
          <span style="
            position:absolute;top:-5px;right:-8px;
            background:{T.cyan};color:{T.bg0};
            font:600 8px/1 {T.mono};
            padding:1px 4px;border-radius:6px;
            min-width:14px;text-align:center;
          ">{totalUnread > 99 ? '99+' : totalUnread}</span>
        {/if}
        {#if tab.id === 'ai' && taggingProgress.active}
          <span class="tagging-dot" style="
            position:absolute;top:-4px;right:-6px;
            width:7px;height:7px;border-radius:50%;
            background:{T.amber};
          "></span>
        {/if}
      </div>
      <span>{tab.label}</span>
    </button>
  {/each}
  </div>
</div>
