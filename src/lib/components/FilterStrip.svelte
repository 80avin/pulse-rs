<script lang="ts">
  import { T } from '$lib/tokens';
  import Icon from './Icon.svelte';
  import TagFilterRow from './TagFilterRow.svelte';

  let { filter, onFilter, sort, onSort, onMarkAllRead, counts, activeTag = null, onClearTagFilter, topTags = [], onTagFilter }: {
    filter: string;
    onFilter: (f: string) => void;
    sort: string;
    onSort: (s: string) => void;
    onMarkAllRead: () => void;
    counts?: { all: number; unread: number; saved: number };
    activeTag?: string | null;
    onClearTagFilter?: () => void;
    topTags?: string[];
    onTagFilter?: (tag: string) => void;
  } = $props();

  const tabs = $derived([
    { id: 'all',    label: 'all',    n: counts?.all    ?? 0                         },
    { id: 'unread', label: 'unread', n: counts?.unread ?? 0                         },
    { id: 'saved',  label: 'saved',  n: counts?.saved  ?? 0, color: T.amber          },
  ]);
</script>

<div class="flex flex-col shrink-0 border-t border-bd-0 bg-bg-1">
  <!-- Tabs row -->
  <div class="flex items-center overflow-x-auto" style="scrollbar-width:none;">
    {#each tabs as tab}
      {@const a = tab.id === filter}
      <button
        onclick={() => onFilter(tab.id)}
        aria-pressed={a}
        class="shrink-0 border-none border-r border-bd-0 cursor-pointer flex items-center px-3 min-h-11 gap-1.25 tracking-[0.3px] text-[11px] leading-none font-mono" style="
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
      class="shrink-0 bg-transparent border-none border-l border-bd-0 text-ink-1 cursor-pointer flex items-center min-h-11 px-3 gap-1 tracking-[0.3px] text-[11px] leading-none font-mono" aria-label="Toggle sort order"
    >
      <span class="text-ink-3">sort</span>
      <span>{sort === 'time' ? 'time ↓' : 'score ↓'}</span>
    </button>
    <button
      onclick={onMarkAllRead}
      title="Mark all read"
      class="shrink-0 bg-transparent border-none border-l border-bd-0 text-green flex items-center cursor-pointer min-h-11 px-3" aria-label="Mark all read"
    >
      <Icon name="check" size={14} />
    </button>
  </div>

  <!-- Tag filter row (shown when activeTag is set or top tags are available) -->
  <TagFilterRow activeTag={activeTag} {topTags} onSelectTag={onTagFilter} onClearTag={onClearTagFilter} />
</div>
