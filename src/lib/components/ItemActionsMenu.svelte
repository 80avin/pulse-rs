<script lang="ts">
  import { T } from '$lib/tokens';
  import { markRead, toggleSaved, hideItem } from '$lib/stores/data.svelte';
  import { openExternal, shareItem } from '$lib/utils';
  import Icon from './Icon.svelte';
  import type { FeedItem } from '$lib/types';

  let { item, mode, x = 0, y = 0, onClose }: {
    item: FeedItem;
    mode: 'popup' | 'sheet';
    x?: number;
    y?: number;
    onClose: () => void;
  } = $props();

  function copyUrl(u: string) { navigator.clipboard.writeText(u).catch(() => {}); }
  function copyTitle(t: string) { navigator.clipboard.writeText(t).catch(() => {}); }
  const isHnSelf = $derived(item.url?.includes('news.ycombinator.com/item') ?? false);

  function act(fn: () => void) { fn(); onClose(); }

  const rowCls = 'flex items-center gap-2.5 w-full bg-transparent border-none border-b border-bd-0 cursor-pointer text-left p-[11px_14px] text-[11px] leading-none font-mono';
</script>

{#if mode === 'sheet'}
  <!-- Bottom sheet -->
  <div class="fixed inset-0 z-[100] bg-black/40" role="presentation" onclick={onClose} aria-label="Close actions"></div>
  <div class="fixed inset-x-0 bottom-0 z-[101] bg-bg-1 border-t border-bd-1 rounded-t-xl p-2 pb-[max(8px,env(safe-area-inset-bottom))]">
    <div class="flex justify-center mb-1.5"><span class="inline-block w-9 h-1 rounded-full bg-bd-2"></span></div>
    <div class="flex items-center gap-2.5 px-3 py-2 border-b border-bd-0 mb-1">
      <span class="text-ink-0 flex-1 truncate text-[12px] leading-[1.3] font-mono">{item.title}</span>
    </div>
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
  <div class="fixed z-[100] bg-bg-1 border border-bd-1 rounded overflow-hidden w-52 shadow-[0_8px_32px_rgba(0,0,0,0.6)]" style="left:{Math.min(x, window.innerWidth - 220)}px;top:{Math.min(y, window.innerHeight - 320)}px;">
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
