<script lang="ts">
  import { T, TAG_COLORS } from '$lib/tokens';

  let { activeTag = null, topTags = [], onSelectTag, onClearTag }: {
    activeTag?: string | null;
    topTags?: string[];
    onSelectTag?: (tag: string) => void;
    onClearTag?: () => void;
  } = $props();
</script>

{#if activeTag || topTags.length > 0}
  <div class="flex items-center gap-1.5 border-t border-bd-0 overflow-x-auto flex-nowrap pt-[5px] px-2.5 pb-1.5 min-h-0" style="scrollbar-width:none;">
    {#if activeTag}
      {@const tc = TAG_COLORS[activeTag] ?? { fg: T.cyan, bg: 'rgba(78,205,214,0.10)', bd: 'rgba(78,205,214,0.30)' }}
      <button
        type="button"
        onclick={() => onClearTag?.()}
        class="shrink-0 inline-flex items-center cursor-pointer whitespace-nowrap gap-1 px-1.75 py-0.5 rounded-sm tracking-[0.2px] text-[10px] leading-none font-mono" style="background:{tc.bg};border:1px solid {tc.bd};color:{tc.fg};"
      >
        <span class="text-ink-3">tag:</span>{activeTag} ×
      </button>
      {#if topTags.length > 0}
        <span class="shrink-0 text-ink-3 text-[10px] leading-none font-mono">·</span>
        {#each topTags as tag}
          {@const tc2 = TAG_COLORS[tag] ?? { fg: T.ink2, bg: 'transparent', bd: T.bd1 }}
          <button
            type="button"
            onclick={() => onSelectTag?.(tag)}
            class="shrink-0 inline-flex items-center bg-transparent cursor-pointer whitespace-nowrap px-1.75 py-0.5 rounded-sm border border-bd-1 text-[10px] leading-none font-mono" style="color:{tag === activeTag ? tc2.fg : T.ink3};opacity:{tag === activeTag ? 1 : 0.6};"
          >{tag}</button>
        {/each}
      {/if}
    {:else}
      <span class="shrink-0 text-ink-3 tracking-[0.3px] text-[10px] leading-none font-mono">top:</span>
      {#each topTags as tag}
        {@const tc = TAG_COLORS[tag] ?? { fg: T.ink2, bg: 'transparent', bd: T.bd1 }}
        <button
          type="button"
          onclick={() => onSelectTag?.(tag)}
          class="shrink-0 inline-flex items-center cursor-pointer whitespace-nowrap px-1.75 py-0.5 rounded-sm text-[10px] leading-none font-mono" style="background:{tc.bg};border:1px solid {tc.bd};color:{tc.fg};"
        >{tag}</button>
      {/each}
    {/if}
  </div>
{/if}
