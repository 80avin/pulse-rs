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

<div style="display:flex;flex-direction:column;border-top:1px solid {T.bd0};background:{T.bg1};flex-shrink:0;">
  <!-- Tabs row -->
  <div style="display:flex;align-items:center;overflow-x:auto;scrollbar-width:none;">
    {#each tabs as tab}
      {@const a = tab.id === filter}
      <button
        onclick={() => onFilter(tab.id)}
        style="
          flex-shrink:0;padding:10px 12px;
          background:{a ? T.bg3 : 'transparent'};
          border:none;border-right:1px solid {T.bd0};
          color:{a ? (tab.color ?? T.cyan) : T.ink1};
          font:{a ? '600' : '400'} 11px/1 {T.mono};
          cursor:pointer;display:flex;align-items:center;gap:5px;letter-spacing:0.3px;
        "
      >
        <span>{tab.label}</span>
        <span style="font:10px/1 {T.mono};color:{a ? (tab.color ?? T.cyan) : T.ink3};font-variant-numeric:tabular-nums;">{tab.n}</span>
      </button>
    {/each}
    <div style="flex:1;min-width:4px;"></div>
    <button
      onclick={() => onSort(sort === 'time' ? 'score' : 'time')}
      style="flex-shrink:0;padding:10px 12px;background:transparent;border:none;border-left:1px solid {T.bd0};color:{T.ink1};font:11px/1 {T.mono};letter-spacing:0.3px;cursor:pointer;display:flex;align-items:center;gap:4px;"
    >
      <span style="color:{T.ink3};">sort</span>
      <span>{sort === 'time' ? 'time ↓' : 'score ↓'}</span>
    </button>
    <button
      onclick={onMarkAllRead}
      title="Mark all read"
      style="flex-shrink:0;padding:10px 12px;background:transparent;border:none;border-left:1px solid {T.bd0};color:{T.green};display:flex;align-items:center;cursor:pointer;"
    >
      <Icon name="check" size={14} />
    </button>
  </div>

  <!-- Tag filter row (shown when activeTag is set or top tags are available) -->
  {#if showTagRow}
    <div style="display:flex;align-items:center;gap:6px;padding:5px 10px 6px;border-top:1px solid {T.bd0};overflow-x:auto;scrollbar-width:none;flex-wrap:nowrap;min-height:0;">
      {#if activeTag}
        {@const tc = TAG_COLORS[activeTag] ?? { fg: T.cyan, bg: 'rgba(78,205,214,0.10)', bd: 'rgba(78,205,214,0.30)' }}
        <button
          onclick={() => onClearTagFilter?.()}
          style="flex-shrink:0;display:inline-flex;align-items:center;gap:4px;padding:2px 7px;background:{tc.bg};border:1px solid {tc.bd};border-radius:2px;font:9px/1 {T.mono};color:{tc.fg};cursor:pointer;letter-spacing:0.2px;white-space:nowrap;"
        >
          <span style="color:{T.ink3};">tag:</span>{activeTag} ×
        </button>
        {#if topTags.length > 0}
          <span style="flex-shrink:0;color:{T.ink3};font:9px/1 {T.mono};">·</span>
          {#each topTags as tag}
            {@const tc2 = TAG_COLORS[tag] ?? { fg: T.ink2, bg: 'transparent', bd: T.bd1 }}
            <button
              onclick={() => onTagFilter?.(tag)}
              style="flex-shrink:0;display:inline-flex;align-items:center;padding:2px 7px;background:transparent;border:1px solid {T.bd1};border-radius:2px;font:9px/1 {T.mono};color:{tag === activeTag ? tc2.fg : T.ink3};cursor:pointer;white-space:nowrap;opacity:{tag === activeTag ? 1 : 0.6};"
            >{tag}</button>
          {/each}
        {/if}
      {:else}
        <span style="flex-shrink:0;font:9px/1 {T.mono};color:{T.ink3};letter-spacing:0.3px;">top:</span>
        {#each topTags as tag}
          {@const tc = TAG_COLORS[tag] ?? { fg: T.ink2, bg: 'transparent', bd: T.bd1 }}
          <button
            onclick={() => onTagFilter?.(tag)}
            style="flex-shrink:0;display:inline-flex;align-items:center;padding:2px 7px;background:{tc.bg};border:1px solid {tc.bd};border-radius:2px;font:9px/1 {T.mono};color:{tc.fg};cursor:pointer;white-space:nowrap;"
          >{tag}</button>
        {/each}
      {/if}
    </div>
  {/if}
</div>
