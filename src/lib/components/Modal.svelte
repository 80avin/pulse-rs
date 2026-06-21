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
    <Dialog.Overlay class="fixed inset-0 bg-black/55 z-20" />
    <Dialog.Content
      preventScroll={true}
      class="bg-bg-1 border border-bd-1 flex flex-col fixed left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2 rounded-[6px] max-w-[90vw] max-h-[80vh] shadow-[0_16px_48px_rgba(0,0,0,0.7)] z-20"
      style="width:{width};"
    >
      <div class="flex items-center justify-between shrink-0 border-b border-bd-0 px-4 py-3">
        <Dialog.Title class="text-ink-0 uppercase m-0 tracking-[0.6px] font-semibold text-[12px] leading-none font-mono">{title}</Dialog.Title>
        <Dialog.Close class="bg-transparent border-none cursor-pointer flex items-center p-1 rounded" aria-label="Close dialog">
          <Icon name="x" size={14} color={T.ink1} />
        </Dialog.Close>
      </div>
      <div class="flex-1 overflow-y-auto p-4">
        {@render children()}
      </div>
    </Dialog.Content>
  </Dialog.Portal>
</Dialog.Root>
