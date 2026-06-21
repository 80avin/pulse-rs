<script lang="ts">
  import { T, TAG_COLORS } from '$lib/tokens';
  import Icon from './Icon.svelte';

  let { filter, onFilter, sort, onSort, onMarkAllRead, counts, activeTag = null, onClearTagFilter, topTags = [], onTagFilter }: {
    filter: string;
    onFilter: (f: string) => void;
    sort: string;
    onSort: (s: string) => void;
    onMarkAllRead: () => void;
    counts?: { all: number; unread: number; saved: number; signal: number };
    activeTag?: string | null;
    onClearTagFilter?: () => void;
    topTags?: string[];
    onTagFilter?: (tag: string) => void;
  } = $props();

  const tabs = $derived([
    { id: 'all',    label: 'all',    n: counts?.all    ?? 0                         },
    { id: 'unread', label: 'unread', n: counts?.unread ?? 0                         },
    { id: 'saved',  label: 'saved',  n: counts?.saved  ?? 0, color: T.amber          },
    { id: 'signal', label: 'signal', n: counts?.signal ?? 0, color: T.cyan           },
  ]);

  const showTagRow = $derived(!!activeTag || topTags.length > 0);
</script>

<div class="flex flex-col shrink-0 border-t border-bd-0 bg-bg-1">
  <!-- Tabs row -->
  <div class="flex items-center overflow-x-auto" style="scrollbar-width:none;">
    {#each tabs as tab}
      {@const a = tab.id === filter}
      <button
        onclick={() => onFilter(tab.id)}
        aria-pressed={a}
        class="shrink-0 border-none border-r border-bd-0 cursor-pointer flex items-center px-3 py-2.5 gap-1.25 tracking-[0.3px] text-[11px] leading-none font-mono" style="
          background:{a ? T.bg3 : 'transparent'};
          color:{a ? (tab.color ?? T.cyan) : T.ink1};
          font-weight:{a ? '600' : '400'};
        "
      >
        <span>{tab.label}</span>
        <span class="tabular-nums text-[10px] leading-none font-mono" style="color:{a ? (tab.color ?? T.cyan) : T.ink3};">{tab.n}</span>
      </button>
    {/each}
    <div class="flex-1 min-w-[4px]"></div>
    <button
      onclick={() => onSort(sort === 'time' ? 'score' : 'time')}
      class="shrink-0 bg-transparent border-none border-l border-bd-0 text-ink-1 cursor-pointer flex items-center p-2.5 px-3 gap-1 tracking-[0.3px] text-[11px] leading-none font-mono"
    >
      <span class="text-ink-3">sort</span>
      <span>{sort === 'time' ? 'time ↓' : 'score ↓'}</span>
    </button>
    <button
      onclick={onMarkAllRead}
      title="Mark all read"
      class="shrink-0 bg-transparent border-none border-l border-bd-0 text-green flex items-center cursor-pointer p-2.5 px-3"
    >
      <Icon name="check" size={14} />
    </button>
  </div>

  <!-- Tag filter row (shown when activeTag is set or top tags are available) -->
  {#if showTagRow}
    <div class="flex items-center gap-1.5 border-t border-bd-0 overflow-x-auto flex-nowrap pt-[5px] px-2.5 pb-1.5 min-h-0" style="scrollbar-width:none;">
      {#if activeTag}
        {@const tc = TAG_COLORS[activeTag] ?? { fg: T.cyan, bg: 'rgba(78,205,214,0.10)', bd: 'rgba(78,205,214,0.30)' }}
        <button
          onclick={() => onClearTagFilter?.()}
          class="shrink-0 inline-flex items-center cursor-pointer whitespace-nowrap gap-1 px-1.75 py-0.5 rounded-sm tracking-[0.2px] text-[10px] leading-none font-mono" style="background:{tc.bg};border:1px solid {tc.bd};color:{tc.fg};"
        >
          <span class="text-ink-3">tag:</span>{activeTag} ×
        </button>
        {#if topTags.length > 0}
          <span class="shrink-0 text-ink-3 text-[10px] leading-none font-mono">·</span>
          {#each topTags as tag}
            {@const tc2 = TAG_COLORS[tag] ?? { fg: T.ink2, bg: 'transparent', bd: T.bd1 }}
            <button
              onclick={() => onTagFilter?.(tag)}
              class="shrink-0 inline-flex items-center bg-transparent cursor-pointer whitespace-nowrap px-1.75 py-0.5 rounded-sm border border-bd-1 text-[10px] leading-none font-mono" style="color:{tag === activeTag ? tc2.fg : T.ink3};opacity:{tag === activeTag ? 1 : 0.6};"
            >{tag}</button>
          {/each}
        {/if}
      {:else}
        <span class="shrink-0 text-ink-3 tracking-[0.3px] text-[10px] leading-none font-mono">top:</span>
        {#each topTags as tag}
          {@const tc = TAG_COLORS[tag] ?? { fg: T.ink2, bg: 'transparent', bd: T.bd1 }}
          <button
            onclick={() => onTagFilter?.(tag)}
            class="shrink-0 inline-flex items-center cursor-pointer whitespace-nowrap px-1.75 py-0.5 rounded-sm text-[10px] leading-none font-mono" style="background:{tc.bg};border:1px solid {tc.bd};color:{tc.fg};"
          >{tag}</button>
        {/each}
      {/if}
    </div>
  {/if}
</div>
