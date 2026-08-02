<script lang="ts">
  import { T } from '$lib/tokens';

  let { active = 'all' as string, onChange }: {
    active?: string;
    onChange: (filter: string) => void;
  } = $props();

  const PILLS = ['all', 'unread', 'saved'] as const;
</script>

<div class="border-b border-bd-0 p-1.5 px-2 pb-1">
  <div class="text-ink-3 uppercase px-1 pb-[3px] text-[10px] leading-none font-mono tracking-[0.6px]">filter</div>
  <div class="flex flex-wrap gap-0.75" role="group" aria-label="Item filters">
    {#each PILLS as pill}
      {@const isActive = active === pill}
      <button
        onclick={() => onChange(pill)}
        aria-pressed={isActive}
        class={'cursor-pointer uppercase bg-transparent p-[4px_10px] rounded-[3px] text-[10px] leading-none font-mono tracking-[0.3px] ' + (isActive ? 'text-cyan' : 'text-ink-2')}
        style="border:1px solid {isActive ? T.cyan : T.bd1};background:{isActive ? 'rgba(78,205,214,0.10)' : 'transparent'};"
        onmouseenter={(e) => { if(!isActive)(e.currentTarget as HTMLElement).style.borderColor = T.ink3; }}
        onmouseleave={(e) => { if(!isActive)(e.currentTarget as HTMLElement).style.borderColor = T.bd1; }}
      >{pill}</button>
    {/each}
  </div>
</div>
