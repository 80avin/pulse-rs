<script lang="ts">
  import { T } from '$lib/tokens';

  let { active = 'all' as string, onChange }: {
    active?: string;
    onChange: (filter: string) => void;
  } = $props();

  const PILLS = ['all', 'unread', 'saved', 'signal'] as const;
</script>

<div style="padding:6px 8px 4px;border-bottom:1px solid var(--color-bd-0);">
  <div style="padding:0 4px 3px;font:10px/1 var(--font-mono);color:var(--color-ink-3);letter-spacing:0.6px;text-transform:uppercase;">filter</div>
  <div style="display:flex;flex-wrap:wrap;gap:3px;" role="group" aria-label="Item filters">
    {#each PILLS as pill}
      {@const isActive = active === pill}
      <button
        onclick={() => onChange(pill)}
        aria-pressed={isActive}
        class={'cursor-pointer uppercase ' + (isActive ? 'text-cyan' : 'text-ink-2')}
        style="padding:4px 10px;border-radius:3px;font:10px/1 var(--font-mono);letter-spacing:0.3px;border:1px solid {isActive ? T.cyan : T.bd1};background:{isActive ? 'rgba(78,205,214,0.10)' : 'transparent'};"
        onmouseenter={(e) => { if(!isActive)(e.currentTarget as HTMLElement).style.borderColor = T.ink3; }}
        onmouseleave={(e) => { if(!isActive)(e.currentTarget as HTMLElement).style.borderColor = T.bd1; }}
      >{pill}</button>
    {/each}
  </div>
</div>
