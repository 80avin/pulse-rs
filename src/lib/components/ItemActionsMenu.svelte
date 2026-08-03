<script lang="ts">
  import { T } from '$lib/tokens';
  import { Portal } from 'bits-ui';
  import { markRead, toggleSaved, hideItem, setItemTags, normalizeTag } from '$lib/stores/data.svelte';
  import { openExternal, shareItem } from '$lib/utils';
  import { itemMenu, closeItemMenu } from '$lib/stores/item-menu.svelte';
  import Icon from './Icon.svelte';

  // Single shared menu instance, portal-rendered into <body> so it escapes any
  // transformed/overflow ancestor (a virtualized row would otherwise anchor the
  // fixed sheet to itself and scroll with the list).
  const item = $derived(itemMenu.current?.item);
  const mode = $derived(itemMenu.current?.mode ?? 'sheet');
  const x = $derived(itemMenu.current?.x ?? 0);
  const y = $derived(itemMenu.current?.y ?? 0);

  function copyUrl(u: string) { navigator.clipboard.writeText(u).catch(() => {}); }
  function copyTitle(t: string) { navigator.clipboard.writeText(t).catch(() => {}); }
  const isHnSelf = $derived(item?.url?.includes('news.ycombinator.com/item') ?? false);

  function act(fn: () => void) { fn(); closeItemMenu(); }

  let tagDraft = $state('');
  function addTag() {
    if (!item) return;
    const t = normalizeTag(tagDraft);
    if (t) setItemTags(item.id, [...item.userTags, t]);
    tagDraft = '';
  }
  function removeTag(tag: string) { if (item) setItemTags(item.id, item.userTags.filter(t => t !== tag)); }

  type MenuAction =
    | { kind: 'action'; icon: string; label: string; style: string; iconColor: string; action: () => void }
    | { kind: 'divider' };

  const actions = $derived<MenuAction[]>(
    item
      ? [
          ...(item.url && !isHnSelf
            ? [{ kind: 'action' as const, icon: 'ext', label: 'Open in browser', style: `color:${T.ink2}`, iconColor: T.ink2, action: () => openExternal(item.url!) }]
            : []),
          ...(item.url
            ? [{ kind: 'action' as const, icon: 'link', label: 'Copy URL', style: `color:${T.ink2}`, iconColor: T.ink2, action: () => copyUrl(item.url!) }]
            : []),
          { kind: 'action' as const, icon: 'edit', label: 'Copy title', style: `color:${T.ink2}`, iconColor: T.ink2, action: () => copyTitle(item.title) },
          ...(item.title && (item.url || item.externalUrl)
            ? [{ kind: 'action' as const, icon: 'share', label: 'Share', style: `color:${T.ink2}`, iconColor: T.ink2, action: () => shareItem(item.title, item.url ?? item.externalUrl) }]
            : []),
          { kind: 'divider' as const },
          { kind: 'action' as const, icon: 'check', label: item.read ? 'Mark as unread' : 'Mark as read', style: `color:${item.read ? T.ink1 : T.cyan}`, iconColor: item.read ? T.ink2 : T.cyan, action: () => markRead(item.id, !item.read) },
          { kind: 'action' as const, icon: 'bookmark', label: item.saved ? 'Unsave' : 'Save', style: `color:${item.saved ? T.amber : T.ink1}`, iconColor: item.saved ? T.amber : T.ink2, action: () => toggleSaved(item.id) },
          { kind: 'action' as const, icon: 'eye-off', label: 'Hide', style: `border-bottom:none;color:${T.red}`, iconColor: T.red, action: () => hideItem(item.id) },
        ]
      : []
  );

  const pad = { sheet: 'p-[11px_14px]', popup: 'p-[11px_14px]' } as const;
  const rowCls = $derived(`flex items-center gap-2.5 w-full bg-transparent border-none border-b border-bd-0 cursor-pointer text-left ${pad[mode]} text-[11px] leading-none font-mono`);
</script>

{#snippet tagEditor()}
  <div class="px-3 py-2 border-b border-bd-0">
    <div class="flex items-center gap-1 mb-1.5">
      <span class="text-ink-3 uppercase tracking-[0.5px] text-[9px] leading-none font-mono">your tags</span>
      <span class="text-ink-4 text-[9px] leading-none font-mono">(click to remove)</span>
    </div>
    {#if item && item.userTags.length > 0}
      <div class="flex flex-wrap gap-1 mb-1.5">
        {#each item.userTags as tag}
          <button class="inline-flex items-center gap-1 bg-transparent cursor-pointer px-1.5 py-0.5 rounded-sm text-[9px] leading-none font-mono" style="color:{T.amber};border:1px dashed {T.amber}55;" onclick={() => removeTag(tag)}>{tag} ×</button>
        {/each}
      </div>
    {/if}
    <form onsubmit={(e) => { e.preventDefault(); addTag(); }}>
      <input bind:value={tagDraft} placeholder="add tag…" aria-label="Add a tag" class="w-full bg-bg-0 border border-bd-1 rounded px-2 py-1 text-[11px] leading-none font-mono text-ink-0" />
    </form>
  </div>
{/snippet}

{#if item}
  <Portal>
    {#if mode === 'sheet'}
      <!-- Bottom sheet -->
      <div class="fixed inset-0 z-[100] bg-black/40 anim-sheet-overlay-in" role="presentation" onclick={closeItemMenu} aria-label="Close actions"></div>
      <div class="fixed inset-x-0 bottom-0 z-[101] bg-bg-1 border-t border-bd-1 rounded-t-xl p-2 pb-[max(8px,env(safe-area-inset-bottom))] anim-sheet-in">
        <div class="flex justify-center mb-1.5"><span class="inline-block w-9 h-1 rounded-full bg-bd-2"></span></div>
        <div class="flex items-center gap-2.5 px-3 py-2 border-b border-bd-0 mb-1">
          <span class="text-ink-0 flex-1 truncate text-[12px] leading-[1.3] font-mono">{item.title}</span>
        </div>
        {@render tagEditor()}
        {#each actions as a}
          {#if a.kind === 'divider'}
            <div class="bg-bd-0 h-px my-1"></div>
          {:else}
            <button class={rowCls} style={a.style} onclick={() => act(a.action)}><Icon name={a.icon} size={11} color={a.iconColor} /><span>{a.label}</span></button>
          {/if}
        {/each}
      </div>
    {:else}
      <!-- Floating popup (desktop right-click) -->
      <div class="fixed inset-0 z-[100] anim-sheet-overlay-in" onclick={closeItemMenu} aria-hidden="true"></div>
      <div class="fixed z-[101] bg-bg-1 border border-bd-1 rounded overflow-hidden w-64 shadow-[0_8px_32px_rgba(0,0,0,0.6)] anim-pop-in" style="left:{Math.max(4, Math.min(x, window.innerWidth - 260))}px;top:{Math.max(4, Math.min(y, window.innerHeight - 360))}px;">
          {@render tagEditor()}
          {#each actions as a}
            {#if a.kind === 'divider'}
              <div class="bg-bd-0 h-px my-1"></div>
            {:else}
              <button class={rowCls} style={a.style} onclick={() => act(a.action)}><Icon name={a.icon} size={11} color={a.iconColor} /><span>{a.label}</span></button>
            {/if}
          {/each}
        </div>
    {/if}
  </Portal>
{/if}
