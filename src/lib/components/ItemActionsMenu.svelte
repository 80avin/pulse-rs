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

  const rowCls = 'flex items-center gap-2.5 w-full bg-transparent border-none border-b border-bd-0 cursor-pointer text-left p-[11px_14px] text-[11px] leading-none font-mono';
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
      <div class="fixed inset-0 z-[100] bg-black/40" role="presentation" onclick={closeItemMenu} aria-label="Close actions"></div>
      <div class="fixed inset-x-0 bottom-0 z-[101] bg-bg-1 border-t border-bd-1 rounded-t-xl p-2 pb-[max(8px,env(safe-area-inset-bottom))]">
        <div class="flex justify-center mb-1.5"><span class="inline-block w-9 h-1 rounded-full bg-bd-2"></span></div>
        <div class="flex items-center gap-2.5 px-3 py-2 border-b border-bd-0 mb-1">
          <span class="text-ink-0 flex-1 truncate text-[12px] leading-[1.3] font-mono">{item.title}</span>
        </div>
        {@render tagEditor()}
        {#if item.url && !isHnSelf}
          <button class={rowCls} onclick={() => act(() => openExternal(item.url!))}><Icon name="ext" size={11} color={T.ink2} /><span>Open in browser</span></button>
        {/if}
        {#if item.url}
          <button class={rowCls} onclick={() => act(() => copyUrl(item.url!))}><Icon name="link" size={11} color={T.ink2} /><span>Copy URL</span></button>
        {/if}
        <button class={rowCls} onclick={() => act(() => copyTitle(item.title))}><Icon name="edit" size={11} color={T.ink2} /><span>Copy title</span></button>
        {#if item.title && (item.url || item.externalUrl)}
          <button class={rowCls} onclick={() => act(() => shareItem(item.title, item.url ?? item.externalUrl))}><Icon name="share" size={11} color={T.ink2} /><span>Share</span></button>
        {/if}
        <div class="bg-bd-0 h-px my-1"></div>
        <button class={rowCls} style="color:{item.read ? T.ink1 : T.cyan};" onclick={() => act(() => markRead(item.id, !item.read))}><Icon name="check" size={11} color={item.read ? T.ink2 : T.cyan} /><span>{item.read ? 'Mark as unread' : 'Mark as read'}</span></button>
        <button class={rowCls} style="color:{item.saved ? T.amber : T.ink1};" onclick={() => act(() => toggleSaved(item.id))}><Icon name="bookmark" size={11} color={item.saved ? T.amber : T.ink2} /><span>{item.saved ? 'Unsave' : 'Save'}</span></button>
        <button class={rowCls} style="border-bottom:none;color:{T.red};" onclick={() => act(() => hideItem(item.id))}><Icon name="eye-off" size={11} color={T.red} /><span>Hide</span></button>
      </div>
    {:else}
      <!-- Floating popup (desktop right-click) -->
      <div class="fixed inset-0 z-[100]" onclick={closeItemMenu} aria-hidden="true"></div>
      <div class="fixed z-[101] bg-bg-1 border border-bd-1 rounded overflow-hidden w-64 shadow-[0_8px_32px_rgba(0,0,0,0.6)]" style="left:{Math.min(Math.max(4, x), window.innerWidth - 260)}px;top:{Math.min(Math.max(4, y), window.innerHeight - 360)}px;">
        {@render tagEditor()}
        {#if item.url && !isHnSelf}
          <button class={rowCls} onclick={() => act(() => openExternal(item.url!))}><Icon name="ext" size={11} color={T.ink2} /><span>Open in browser</span></button>
        {/if}
        {#if item.url}
          <button class={rowCls} onclick={() => act(() => copyUrl(item.url!))}><Icon name="link" size={11} color={T.ink2} /><span>Copy URL</span></button>
        {/if}
        <button class={rowCls} onclick={() => act(() => copyTitle(item.title))}><Icon name="edit" size={11} color={T.ink2} /><span>Copy title</span></button>
        {#if item.title && (item.url || item.externalUrl)}
          <button class={rowCls} onclick={() => act(() => shareItem(item.title, item.url ?? item.externalUrl))}><Icon name="share" size={11} color={T.ink2} /><span>Share</span></button>
        {/if}
        <div class="bg-bd-0 h-px my-1"></div>
        <button class={rowCls} style="color:{item.read ? T.ink1 : T.cyan};" onclick={() => act(() => markRead(item.id, !item.read))}><Icon name="check" size={11} color={item.read ? T.ink2 : T.cyan} /><span>{item.read ? 'Mark as unread' : 'Mark as read'}</span></button>
        <button class={rowCls} style="color:{item.saved ? T.amber : T.ink1};" onclick={() => act(() => toggleSaved(item.id))}><Icon name="bookmark" size={11} color={item.saved ? T.amber : T.ink2} /><span>{item.saved ? 'Unsave' : 'Save'}</span></button>
        <button class={rowCls} style="border-bottom:none;color:{T.red};" onclick={() => act(() => hideItem(item.id))}><Icon name="eye-off" size={11} color={T.red} /><span>Hide</span></button>
      </div>
    {/if}
  </Portal>
{/if}
