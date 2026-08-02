<script lang="ts">
  import { T } from '$lib/tokens';
  let { options, active, onChange }: { options: string[]; active: string; onChange: (v: string) => void } = $props();
</script>

<div class="flex gap-0.75 bg-bg-0 border border-bd-1 rounded p-0.5" role="group">
  {#each options as opt}
    <button
      onclick={() => onChange(opt)}
      aria-pressed={opt === active}
      class="flex-1 border-none rounded-sm cursor-pointer uppercase px-1 min-h-8 tracking-[0.4px] text-[10px] leading-none font-mono"
      style="background:{opt === active ? T.bg3 : 'transparent'};color:{opt === active ? T.cyan : T.ink2};"
    >{opt}</button>
  {/each}
</div>
