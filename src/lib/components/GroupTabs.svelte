<script lang="ts">
  import { T } from '$lib/tokens';
  import type { Group } from '$lib/types';
  import { createGroup, renameGroup, deleteGroup } from '$lib/store.svelte';
  import Icon from './Icon.svelte';
  import { createTabs, melt } from '@melt-ui/svelte';

  let { groups, active, onSelect, counts = {}, orientation = 'horizontal' }: {
    groups: Group[];
    active: string;
    onSelect: (id: string) => void;
    counts?: Record<string, number>;
    orientation?: 'horizontal' | 'vertical';
  } = $props();

  let editing = $state(false);
  let pressTimer: ReturnType<typeof setTimeout> | null = null;
  let newName = $state('');
  let showNewInput = $state(false);
  let renamingId = $state<string | null>(null);
  let renameVal = $state('');

  function startPress(_id: string) {
    pressTimer = setTimeout(() => { pressTimer = null; editing = true; }, 480);
  }
  function cancelPress() {
    if (pressTimer) { clearTimeout(pressTimer); pressTimer = null; }
  }

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

  const tabs = createTabs({
    activateOnFocus: true,
    loop: false,
    orientation: orientation,
    defaultValue: active,
  });
  const { root, list, trigger } = tabs.elements;

  $effect(() => {
    if (!active) return;
    if (tabs.states.value.get() !== active) {
      tabs.states.value.set(active);
    }
  });

  let pendingSelect = $state<string | null>(null);
  $effect(() => {
    const val = tabs.states.value.get();
    if (val && val !== active) {
      pendingSelect = val;
    }
  });
  $effect(() => {
    if (pendingSelect) {
      const id = pendingSelect;
      pendingSelect = null;
      onSelect(id);
    }
  });
</script>

{#if editing}
  <div style="background:{T.bg1};border-bottom:1px solid {T.bd0};">
    <div style="display:flex;align-items:center;justify-content:space-between;padding:6px 10px;font:10px/1 {T.mono};color:{T.cyan};letter-spacing:0.6px;text-transform:uppercase;background:rgba(78,205,214,0.06);">
      <span style="display:inline-flex;align-items:center;gap:6px;">
        <Icon name="edit" size={11} />
        edit groups
      </span>
      <button
        onclick={() => { editing = false; showNewInput = false; renamingId = null; }}
        style="background:transparent;border:1px solid {T.cyanDim};color:{T.cyan};padding:3px 8px;font:10px/1 {T.mono};letter-spacing:0.4px;border-radius:2px;cursor:pointer;"
      >DONE</button>
    </div>

    {#each groups as g}
      {#if g.id === 'all'}
        <div style="display:grid;grid-template-columns:20px 1fr auto;gap:8px;align-items:center;padding:8px 10px;border-top:1px solid {T.bd0};opacity:0.4;">
          <Icon name="grip" size={14} color={T.ink3} />
          <span style="font:13px/1.2 {T.mono};color:{T.ink0};">{g.name}</span>
          <span style="font:10px/1 {T.mono};color:{T.ink2};">{counts[g.id] ?? g.n}</span>
        </div>
      {:else}
        <div style="display:grid;grid-template-columns:20px 1fr auto auto auto;gap:8px;align-items:center;padding:8px 10px;border-top:1px solid {T.bd0};">
          <Icon name="grip" size={14} color={T.ink3} />
          {#if renamingId === g.id}
            <input
              value={renameVal}
              oninput={(e) => renameVal = (e.target as HTMLInputElement).value}
              onkeydown={(e) => { if (e.key === 'Enter') commitRename(); if (e.key === 'Escape') renamingId = null; }}
              onblur={commitRename}
              style="font:13px/1.2 {T.mono};color:{T.ink0};padding:4px 6px;background:{T.bg0};border:1px solid {T.cyan};border-radius:2px;outline:none;width:100%;"
              autofocus
            />
          {:else}
            <span style="font:13px/1.2 {T.mono};color:{T.ink0};padding:4px 6px;background:{T.bg0};border:1px solid {T.bd1};border-radius:2px;">{g.name}</span>
          {/if}
          <span style="font:10px/1 {T.mono};color:{T.ink2};font-variant-numeric:tabular-nums;">{counts[g.id] ?? g.n}</span>
          <button
            onclick={() => startRename(g)}
            style="width:30px;height:30px;padding:0;background:transparent;border:1px solid {T.bd1};border-radius:2px;color:{T.ink1};display:flex;align-items:center;justify-content:center;cursor:pointer;"
          >
            <Icon name="edit" size={13} />
          </button>
          <button
            onclick={() => handleDelete(g.id)}
            style="width:30px;height:30px;padding:0;background:transparent;border:1px solid {T.redDim};border-radius:2px;color:{T.red};display:flex;align-items:center;justify-content:center;cursor:pointer;"
          >
            <Icon name="trash" size={13} />
          </button>
        </div>
      {/if}
    {/each}

    {#if showNewInput}
      <div style="display:flex;align-items:center;gap:8px;padding:8px 10px;border-top:1px solid {T.bd0};">
        <Icon name="plus" size={13} color={T.cyan} />
        <input
          bind:value={newName}
          placeholder="group name"
          onkeydown={(e) => { if (e.key === 'Enter') commitNewGroup(); if (e.key === 'Escape') { showNewInput = false; newName = ''; } }}
          onblur={commitNewGroup}
          style="flex:1;font:12px/1 {T.mono};color:{T.ink0};background:{T.bg0};border:1px solid {T.cyan};border-radius:2px;padding:6px 8px;outline:none;"
          autofocus
        />
      </div>
    {:else}
      <button
        onclick={() => { showNewInput = true; }}
        style="width:100%;padding:10px 12px;background:transparent;border:none;border-top:1px solid {T.bd0};color:{T.cyan};font:11px/1 {T.mono};letter-spacing:0.4px;text-align:left;cursor:pointer;display:flex;align-items:center;gap:8px;"
      >
        <Icon name="plus" size={13} />
        NEW GROUP
      </button>
    {/if}
  </div>
{:else if orientation === 'vertical'}
  <div use:melt={$root} style="display:flex;flex-direction:column;padding:4px 0;background:{T.bg1};flex-shrink:0;">
    <div use:melt={$list} style="display:flex;flex-direction:column;">
      {#each groups as g}
        {@const triggerAttrs = $trigger(g.id)}
        <button
          {...triggerAttrs}
          use:melt={triggerAttrs}
          onclick={() => onSelect(g.id)}
          oncontextmenu={(e: MouseEvent) => { e.preventDefault(); editing = true; }}
          onmouseenter={(e: MouseEvent) => { if (g.id !== active)(e.currentTarget as HTMLElement).style.background = T.bg2; }}
          onmouseleave={(e: MouseEvent) => { if (g.id !== active)(e.currentTarget as HTMLElement).style.background = 'transparent'; }}
          onfocus={(e: FocusEvent) => { (e.currentTarget as HTMLElement).style.outline = `1px solid ${T.cyan}`; }}
          onblur={(e: FocusEvent) => { (e.currentTarget as HTMLElement).style.outline = 'none'; }}
          style="
            display:flex;align-items:center;gap:8px;padding:6px 12px;
            background:{g.id === active ? 'rgba(78,205,214,0.06)' : 'transparent'};
            border:none;border-left:2px solid {g.id === active ? T.cyan : 'transparent'};
            color:{g.id === active ? T.ink0 : T.ink1};
            font:{g.id === active ? '600' : '400'} 13px/1.2 {T.mono};
            cursor:pointer;text-align:left;width:100%;
          "
        >
          <span style="flex:1;">{g.name}</span>
          <span style="font:10px/1 {T.mono};color:{g.id === active ? T.cyan : T.ink3};font-variant-numeric:tabular-nums;">{counts[g.id] ?? g.n}</span>
        </button>
      {/each}
    </div>
    <button
      onclick={() => { editing = true; }}
      onmouseenter={(e: MouseEvent) => { (e.currentTarget as HTMLElement).style.background = T.bg2; }}
      onmouseleave={(e: MouseEvent) => { (e.currentTarget as HTMLElement).style.background = 'transparent'; }}
      style="display:flex;align-items:center;gap:6px;padding:5px 12px;background:transparent;border:none;color:{T.ink3};font:10px/1 {T.mono};cursor:pointer;text-align:left;width:100%;margin-top:2px;"
    >
      <Icon name="plus" size={10} color={T.ink3} />
      <span>New Group</span>
    </button>
  </div>
{:else}
  <div use:melt={$root} style="display:flex;overflow-x:auto;flex-shrink:0;border-bottom:1px solid {T.bd0};background:{T.bg1};scrollbar-width:none;">
    <div use:melt={$list} style="display:flex;">
      {#each groups as g}
        {@const triggerAttrs = $trigger(g.id)}
        <button
          {...triggerAttrs}
          use:melt={triggerAttrs}
          onclick={() => onSelect(g.id)}
          onpointerdown={() => startPress(g.id)}
          onpointerup={cancelPress}
          onpointercancel={cancelPress}
          onmouseenter={(e: MouseEvent) => { if (g.id !== active)(e.currentTarget as HTMLElement).style.background = T.bg2; }}
          onmouseleave={(e: MouseEvent) => { if (g.id !== active)(e.currentTarget as HTMLElement).style.background = 'transparent'; }}
          style="
            flex-shrink:0;padding:13px 14px;min-height:44px;
            background:transparent;border:none;
            border-bottom:2px solid {g.id === active ? T.cyan : 'transparent'};
            color:{g.id === active ? T.ink0 : T.ink2};
            font:{g.id === active ? '600' : '400'} 13px/1.2 {T.mono};
            cursor:pointer;display:flex;align-items:center;gap:6px;letter-spacing:0.3px;
          "
        >
          <span>{g.name}</span>
          <span style="font:10px/1 {T.mono};color:{g.id === active ? T.cyan : T.ink3};font-variant-numeric:tabular-nums;">{counts[g.id] ?? g.n}</span>
        </button>
      {/each}
    </div>
    <div style="flex:1;min-width:8px;"></div>
    <button
      onclick={() => { editing = true; }}
      style="flex-shrink:0;padding:0 12px;background:transparent;border:none;border-left:1px solid {T.bd0};color:{T.ink2};display:flex;align-items:center;cursor:pointer;"
    >
      <Icon name="edit" size={14} />
    </button>
  </div>
{/if}
