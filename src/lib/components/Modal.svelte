<script lang="ts">
  import type { Snippet } from 'svelte';
  import { T } from '$lib/tokens';
  import Icon from './Icon.svelte';
  import { Dialog } from 'bits-ui';

  let {
    open = $bindable(false),
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
</script>

<Dialog.Root bind:open onOpenChange={(isOpen) => { if (!isOpen) onClose(); }}>
  <Dialog.Portal>
    <Dialog.Overlay style="
      position:fixed;inset:0;
      background:rgba(0,0,0,0.55);
      z-index:200;
    " />
    <Dialog.Content
      preventScroll={true}
      style="
        position:fixed;left:50%;top:50%;transform:translate(-50%,-50%);
        background:{T.bg1};
        border:1px solid {T.bd1};
        border-radius:6px;
        width:{width};
        max-width:90vw;
        max-height:80vh;
        display:flex;flex-direction:column;
        box-shadow:0 16px 48px rgba(0,0,0,0.7);
        z-index:200;
      "
    >
      <div style="padding:12px 16px;border-bottom:1px solid {T.bd0};display:flex;align-items:center;justify-content:space-between;flex-shrink:0;">
        <Dialog.Title style="font:600 12px/1 {T.mono};color:{T.ink0};letter-spacing:0.6px;text-transform:uppercase;margin:0;">{title}</Dialog.Title>
        <Dialog.Close style="background:transparent;border:none;cursor:pointer;display:flex;align-items:center;padding:4px;border-radius:3px;" aria-label="Close dialog">
          <Icon name="x" size={14} color={T.ink1} />
        </Dialog.Close>
      </div>
      <div style="flex:1;overflow-y:auto;padding:16px;">
        {@render children()}
      </div>
    </Dialog.Content>
  </Dialog.Portal>
</Dialog.Root>
