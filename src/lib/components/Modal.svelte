<script lang="ts">
  import type { Snippet } from 'svelte';
  import { T } from '$lib/tokens';
  import Icon from './Icon.svelte';
  import { createDialog, melt } from '@melt-ui/svelte';

  let {
    open,
    title,
    onClose,
    width = '600px',
    children,
  }: {
    open: boolean;
    title: string;
    onClose: () => void;
    width?: string;
    children: Snippet;
  } = $props();

  const dialog = createDialog({
    defaultOpen: true,
    preventScroll: true,
    onOpenChange: ({ next }) => {
      if (!next) onClose();
      return next;
    },
  });
  const { overlay, content, title: dialogTitle, close: dialogClose } = dialog.elements;
</script>

{#if open}
  <div use:melt={$overlay} style="
    position:fixed;inset:0;
    background:rgba(0,0,0,0.55);
    z-index:200;display:flex;align-items:center;justify-content:center;
  ">
    <div use:melt={$content} style="
      background:{T.bg1};
      border:1px solid {T.bd1};
      border-radius:6px;
      width:{width};
      max-width:90vw;
      max-height:80vh;
      display:flex;flex-direction:column;
      box-shadow:0 16px 48px rgba(0,0,0,0.7);
      transition:opacity 0.15s ease, transform 0.15s ease;
    ">
      <div style="padding:12px 16px;border-bottom:1px solid {T.bd0};display:flex;align-items:center;justify-content:space-between;flex-shrink:0;">
        <h2 use:melt={$dialogTitle} style="font:600 12px/1 {T.mono};color:{T.ink0};letter-spacing:0.6px;text-transform:uppercase;margin:0;">{title}</h2>
        <button use:melt={$dialogClose} onclick={onClose} style="background:transparent;border:none;cursor:pointer;display:flex;align-items:center;padding:4px;border-radius:3px;" onmouseenter={(e: MouseEvent) => { (e.currentTarget as HTMLElement).style.background = T.bg2; }} onmouseleave={(e: MouseEvent) => { (e.currentTarget as HTMLElement).style.background = 'transparent'; }} aria-label="Close dialog">
          <Icon name="x" size={14} color={T.ink1} />
        </button>
      </div>
      <div style="flex:1;overflow-y:auto;padding:16px;">
        {@render children()}
      </div>
    </div>
  </div>
{/if}
