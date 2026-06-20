<script lang="ts">
  import { T } from '$lib/tokens';
  import { markRead, toggleSaved, hideItem } from '$lib/stores/data.svelte';
  import { openExternal, shareItem } from '$lib/utils';
  import Icon from './Icon.svelte';

  let {
    item,
    onClose,
    variant = 'menu',
    onEditNote,
    menuItemBuilder,
  }: {
    item: { id: string; read: boolean; saved: boolean; url?: string; externalUrl?: string; title: string; note?: string };
    onClose: () => void;
    variant?: 'menu' | 'sheet';
    onEditNote?: () => void;
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    menuItemBuilder?: any;
  } = $props();

  const isHnSelf = $derived(item.url?.includes('news.ycombinator.com/item') ?? false);
  const isMenu = $derived(variant === 'menu');
  const iconSz = $derived(isMenu ? 11 : 13);
  const btnPad = $derived(isMenu ? '9px 12px' : '11px 0');
  const btnFont = $derived(isMenu ? `11px/1 ${T.mono}` : `12px/1 ${T.sans}`);

  function copyUrl(u: string) { navigator.clipboard.writeText(u).catch(() => {}); onClose(); }
  function copyTitle(t: string) { navigator.clipboard.writeText(t).catch(() => {}); onClose(); }
  function hoverOn(e: MouseEvent) { if (isMenu) (e.currentTarget as HTMLElement).style.background = T.bg2; }
  function hoverOff(e: MouseEvent) { if (isMenu) (e.currentTarget as HTMLElement).style.background = 'transparent'; }
</script>

{#snippet btn(args: { label: string; icon: string; color: string; iconColor?: string; handler: () => void; last?: boolean })}
  {#if menuItemBuilder}
    <button
      onclick={args.handler}
      onmouseenter={hoverOn}
      onmouseleave={hoverOff}
      {...$menuItemBuilder} use:menuItemBuilder
      style="display:flex;align-items:center;gap:10px;width:100%;padding:{btnPad};background:transparent;border:none;border-bottom:1px solid {args.last ? T.bd1 : T.bd0};color:{args.color};cursor:pointer;text-align:left;font:{btnFont};"
    >
      <Icon name={args.icon} size={iconSz} color={args.iconColor ?? T.ink2} />
      <span>{args.label}</span>
    </button>
  {:else}
    <button
      onclick={args.handler}
      role={isMenu ? undefined : 'menuitem'}
      onmouseenter={hoverOn}
      onmouseleave={hoverOff}
      style="display:flex;align-items:center;gap:10px;width:100%;padding:{btnPad};background:transparent;border:none;border-bottom:1px solid {args.last ? T.bd1 : T.bd0};color:{args.color};cursor:pointer;text-align:left;font:{btnFont};"
    >
      <Icon name={args.icon} size={iconSz} color={args.iconColor ?? T.ink2} />
      <span>{args.label}</span>
    </button>
  {/if}
{/snippet}

{#if item.url && !isHnSelf}
  {@render btn({ label: 'Open in browser', icon: 'ext', color: T.ink0, handler: () => { openExternal(item.url!); onClose(); } })}
{/if}
{#if item.url}
  {@render btn({ label: 'Copy URL', icon: 'link', color: T.ink0, handler: () => copyUrl(item.url!) })}
{/if}
{@render btn({ label: 'Copy title', icon: 'edit', color: T.ink0, handler: () => copyTitle(item.title) })}
{#if item.title && (item.url || item.externalUrl)}
  {@render btn({ label: 'Share', icon: 'share', color: T.ink0, handler: () => { shareItem(item.title, item.url ?? item.externalUrl); onClose(); }, last: true })}
{/if}

<div style="height:1px;background:{T.bd0};margin:4px 0;"></div>

{@render btn({ label: item.read ? 'Mark as unread' : 'Mark as read', icon: 'check', color: item.read ? T.ink1 : T.cyan, iconColor: item.read ? T.ink2 : T.cyan, handler: () => { markRead(item.id, !item.read); onClose(); } })}
{@render btn({ label: item.saved ? 'Unsave' : 'Save', icon: 'bookmark', color: item.saved ? T.amber : T.ink1, iconColor: item.saved ? T.amber : T.ink2, handler: () => { toggleSaved(item.id); onClose(); } })}

{#if isMenu && onEditNote}
  {#if item.note}
    {@render btn({ label: 'Edit note', icon: 'edit', color: T.ink1, handler: () => { onEditNote(); onClose(); } })}
  {:else}
    {@render btn({ label: 'Save with note…', icon: 'edit', color: T.ink1, handler: () => { onEditNote(); onClose(); } })}
  {/if}
{/if}

{@render btn({ label: 'Hide', icon: 'eye-off', color: T.red, iconColor: T.red, handler: () => { hideItem(item.id); onClose(); }, last: true })}
