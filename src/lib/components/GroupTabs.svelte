<script lang="ts">
  import { T } from '$lib/tokens';
  import type { Group } from '$lib/types';
  import { createGroup, renameGroup, deleteGroup } from '$lib/stores/data.svelte';
  import Icon from './Icon.svelte';
  import { longpress } from './longpress.svelte';

  let { groups, active, onSelect, counts = {}, orientation = 'horizontal' }: {
    groups: Group[];
    active: string;
    onSelect: (id: string) => void;
    counts?: Record<string, number>;
    orientation?: 'horizontal' | 'vertical';
  } = $props();

  let editing = $state(false);
  let newName = $state('');
  let showNewInput = $state(false);
  let renamingId = $state<string | null>(null);
  let renameVal = $state('');

  function startRename(g: Group) {
    renamingId = g.id;
    renameVal = g.name;
  }

  function commitRename() {
    if (renamingId && renameVal.trim()) {
      renameGroup(renamingId, renameVal.trim());
    }
    renamingId = null;
  }

  function commitNewGroup() {
    if (newName.trim()) createGroup(newName.trim());
    newName = '';
    showNewInput = false;
  }

  async function handleDelete(id: string) {
    await deleteGroup(id);
    if (active === id) onSelect('all');
  }

  let suppressClick = false;
</script>

{#if orientation === 'vertical'}
  <div class="flex flex-col shrink-0 bg-bg-1 py-1">
    <div class="flex flex-col">
      {#each groups as g}
        <button
          onclick={() => { if (suppressClick) { suppressClick = false; return; } onSelect(g.id); }}
          oncontextmenu={(e: MouseEvent) => { e.preventDefault(); editing = true; }}
          onfocus={(e: FocusEvent) => { (e.currentTarget as HTMLElement).style.outline = `1px solid ${T.cyan}`; }}
          onblur={(e: FocusEvent) => { (e.currentTarget as HTMLElement).style.outline = 'none'; }}
          class="flex items-center gap-2 cursor-pointer text-left w-full border-none px-3 py-1.5 shrink-0" style="
            background:{active === g.id ? 'rgba(78,205,214,0.06)' : 'transparent'};
            border-left:2px solid {active === g.id ? T.cyan : 'transparent'};
            color:{active === g.id ? T.ink0 : T.ink1};
            font:{active === g.id ? '600' : '400'} 13px/1.2 {T.mono};
          "
        >
          <span use:longpress={{ onLongpress: () => { suppressClick = true; editing = true; } }} class="flex-1">{g.name}</span>
          <span use:longpress={{ onLongpress: () => { suppressClick = true; editing = true; } }} class="tabular-nums text-[10px] leading-none font-mono" style="color:{active === g.id ? T.cyan : T.ink3};">{counts[g.id] ?? g.n}</span>
        </button>
      {/each}
    </div>
    <button
      onclick={() => { editing = true; }}
      aria-label="Edit groups"
      class="flex items-center gap-1.5 bg-transparent border-none text-ink-3 cursor-pointer text-left w-full mt-0.5 p-[5px_12px] text-[10px] leading-none font-mono"
    >
      <Icon name="edit" size={10} color={T.ink3} />
      <span>Edit Groups</span>
    </button>
  </div>
{:else}
  <div class="flex shrink-0 border-b border-bd-0 bg-bg-1">
    <div class="flex-1 min-w-0 overflow-x-auto flex" style="scrollbar-width:none;">
      {#each groups as g}
        <button
          onclick={() => { if (suppressClick) { suppressClick = false; return; } onSelect(g.id); }}
          class="shrink-0 bg-transparent border-none cursor-pointer flex items-center gap-1.5" style="
            padding:13px 14px;min-height:44px;
            border-bottom:2px solid {active === g.id ? T.cyan : 'transparent'};
            color:{active === g.id ? T.ink0 : T.ink2};
            font:{active === g.id ? '600' : '400'} 13px/1.2 {T.mono};
            letter-spacing:0.3px;
          "
        >
          <span use:longpress={{ onLongpress: () => { suppressClick = true; editing = true; } }}>{g.name}</span>
          <span use:longpress={{ onLongpress: () => { suppressClick = true; editing = true; } }} class="tabular-nums text-[10px] leading-none font-mono" style="color:{active === g.id ? T.cyan : T.ink3};">{counts[g.id] ?? g.n}</span>
        </button>
      {/each}
    </div>
    <button
      onclick={() => { editing = true; }}
      aria-label="Edit groups"
      class="shrink-0 bg-transparent border-none border-l border-bd-0 text-ink-2 flex items-center cursor-pointer px-3"
    >
      <Icon name="edit" size={14} />
    </button>
  </div>
{/if}

{#if editing}
  <div class="bg-bg-1 border-b border-bd-0">
    <div class="flex items-center justify-between text-cyan uppercase px-2.5 py-1.5 text-[10px] leading-none font-mono tracking-[0.6px] bg-[rgba(78,205,214,0.06)]">
      <span class="inline-flex items-center gap-1.5">
        <Icon name="edit" size={11} />
        edit groups
      </span>
      <button
        onclick={() => { editing = false; showNewInput = false; renamingId = null; }}
        class="bg-transparent text-cyan cursor-pointer rounded-sm border border-cyan-dim p-[3px_8px] text-[10px] leading-none font-mono tracking-[0.4px]"
      >DONE</button>
    </div>

    {#each groups as g}
      {#if g.id === 'all'}
        <div class="grid items-center gap-2 border-t border-bd-0 grid-cols-[20px_1fr_auto] p-2 px-2.5 opacity-40">
          <Icon name="grip" size={14} color={T.ink3} />
          <span class="text-ink-0 text-[13px] leading-[1.2] font-mono">{g.name}</span>
          <span class="text-ink-2 text-[10px] leading-none font-mono">{counts[g.id] ?? g.n}</span>
        </div>
      {:else}
        <div class="grid items-center gap-2 border-t border-bd-0 grid-cols-[20px_1fr_auto_auto_auto] p-2 px-2.5">
          <Icon name="grip" size={14} color={T.ink3} />
          {#if renamingId === g.id}
            <input
              value={renameVal}
              oninput={(e) => renameVal = (e.target as HTMLInputElement).value}
              onkeydown={(e) => { if (e.key === 'Enter') commitRename(); if (e.key === 'Escape') renamingId = null; }}
              onblur={commitRename}
              class="text-ink-0 bg-bg-0 border border-cyan outline-none w-full px-1.5 py-1 rounded-sm text-[13px] leading-[1.2] font-mono"
            />
          {:else}
            <span class="text-ink-0 bg-bg-0 border border-bd-1 px-1.5 py-1 rounded-sm text-[13px] leading-[1.2] font-mono">{g.name}</span>
          {/if}
          <span class="text-ink-2 tabular-nums text-[10px] leading-none font-mono">{counts[g.id] ?? g.n}</span>
          <button
            onclick={() => startRename(g)}
            aria-label={`Rename group ${g.name}`}
            class="p-0 bg-transparent border border-bd-1 text-ink-1 flex items-center justify-center cursor-pointer rounded-sm min-w-11 min-h-11"
          >
            <Icon name="edit" size={13} />
          </button>
          <button
            onclick={() => handleDelete(g.id)}
            aria-label={`Delete group ${g.name}`}
            class="p-0 bg-transparent text-red flex items-center justify-center cursor-pointer rounded-sm min-w-11 min-h-11 border border-red-dim"
          >
            <Icon name="trash" size={13} />
          </button>
        </div>
      {/if}
    {/each}

    {#if showNewInput}
      <div class="flex items-center gap-2 border-t border-bd-0 p-2 px-2.5">
        <Icon name="plus" size={13} color={T.cyan} />
        <input
          bind:value={newName}
          placeholder="group name"
          onkeydown={(e) => { if (e.key === 'Enter') commitNewGroup(); if (e.key === 'Escape') { showNewInput = false; newName = ''; } }}
          onblur={commitNewGroup}
          class="flex-1 text-ink-0 bg-bg-0 border border-cyan outline-none rounded-sm px-2 py-1.5 text-[12px] leading-none font-mono"
        />
      </div>
    {:else}
      <button
        onclick={() => { showNewInput = true; }}
        class="w-full bg-transparent border-none border-t border-bd-0 text-cyan text-left cursor-pointer flex items-center gap-2 px-3 py-2.5 text-[11px] leading-none font-mono tracking-[0.4px]"
      >
        <Icon name="plus" size={13} />
        NEW GROUP
      </button>
    {/if}
  </div>
{/if}
