<script lang="ts">
  import { onMount } from 'svelte';
  import { T } from '$lib/tokens';
  import { tauriInvoke, ageLabel } from '$lib/stores/data.svelte';
  import GhostButton from '$lib/components/shared/GhostButton.svelte';
  import type { BackendItem } from '$lib/types';

  interface OverviewGroup {
    groupId: string;
    groupName: string;
    items: BackendItem[];
  }

  let { onOpenGroup, onExit }: { onOpenGroup: (groupId: string) => void; onExit: () => void } = $props();

  let groups = $state<OverviewGroup[]>([]);
  let loading = $state(true);
  let error = $state(false);

  async function load() {
    loading = true;
    error = false;
    try {
      groups = await tauriInvoke<OverviewGroup[]>('get_overview', { limit: 8 });
    } catch {
      error = true;
    } finally {
      loading = false;
    }
  }

  onMount(load);
</script>

<div class="flex flex-col w-full h-full bg-bg-0 text-ink-0 overflow-hidden">
  <div class="shrink-0 flex items-center gap-2.5 px-3 py-2 border-b border-bd-0 bg-bg-1 text-[10px] leading-none font-mono">
    <span class="uppercase tracking-[0.6px] text-ink-3">overview</span>
    <span class="flex-1"></span>
    <GhostButton onclick={onExit} class="shrink-0 text-[10px] leading-none uppercase tracking-[0.5px]" style="color:{T.cyan};">timeline</GhostButton>
  </div>

  {#if loading}
    <div class="flex-1 flex items-center justify-center text-ink-3 text-[11px] leading-none font-mono">loading overview…</div>
  {:else if error}
    <div class="flex-1 flex flex-col items-center justify-center gap-3 p-6">
      <div class="text-red text-[11px] leading-[1.6] font-mono text-center">couldn't load the overview.<br />check your connection and try again.</div>
      <button onclick={load} class="bg-bg-1 border border-bd-1 text-ink-1 cursor-pointer rounded p-2.5 px-4 text-[11px] leading-none font-mono">retry</button>
    </div>
  {:else if groups.length === 0}
    <div class="flex-1 flex items-center justify-center text-ink-3 text-[11px] leading-[1.6] font-mono text-center px-6">no feeds with items yet — add a feed or sync</div>
  {:else}
    <div class="flex-1 overflow-y-auto grid grid-cols-1 md:grid-cols-2 gap-3 p-3 content-start">
      {#each groups as g}
        <div class="flex flex-col rounded-sm overflow-hidden" style="background:{T.bg1};border:1px solid {T.bd0};">
          <div class="flex items-center gap-2 px-2.5 py-1.5 border-b" style="border-color:{T.bd0};">
            <button
              onclick={() => onOpenGroup(g.groupId)}
              class="flex-1 min-w-0 text-left cursor-pointer bg-transparent border-none p-0 font-mono text-[11px] leading-none font-semibold tracking-[0.3px] truncate"
              style="color:{T.ink0};"
            >{g.groupName}</button>
            <span class="shrink-0 text-[9px] leading-none font-mono" style="color:{T.ink3};">{g.items.length}</span>
          </div>
          <div class="flex-1 min-h-0">
            {#each g.items as item}
              <div class="flex items-center gap-1.5 px-2.5 py-1 border-b" style="border-color:{T.bd0};">
                <span class="flex-1 min-w-0 truncate text-[11px] leading-[1.4] font-mono" style="color:{item.read ? T.ink2 : T.ink0};">{item.title}</span>
                <span class="shrink-0 text-[9px] leading-none font-mono truncate" style="max-width:80px;color:{T.ink3};">{item.sourceName}</span>
                <span class="shrink-0 text-[9px] leading-none font-mono" style="color:{T.ink2};">{ageLabel(item.publishedAt)}</span>
              </div>
            {/each}
          </div>
          <div class="shrink-0 px-1.5 py-1">
            <GhostButton onclick={() => onOpenGroup(g.groupId)} class="w-full flex items-center justify-center text-[10px] leading-none" style="color:{T.cyan};">more →</GhostButton>
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>
