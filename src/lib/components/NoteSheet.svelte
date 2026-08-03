<script lang="ts">
  import { Dialog } from 'bits-ui';
  import Icon from './Icon.svelte';

  let { open, note, onSave, onClose }: {
    open: boolean;
    note: string;
    onSave: (note: string) => void;
    onClose: () => void;
  } = $props();

  // Seed the draft from the incoming note on every open transition; never
  // clobber a draft the user is actively editing if `note` changes mid-sheet.
  let draft = $state('');
  let wasOpen = false;
  $effect(() => {
    if (open && !wasOpen) draft = note;
    wasOpen = open;
  });

  function submit() {
    onSave(draft);
  }
</script>

<Dialog.Root open={open} onOpenChange={(o) => { if (!o) onClose(); }}>
  <Dialog.Portal>
    <Dialog.Overlay class="fixed inset-0 bg-black/55 z-20 anim-sheet-overlay-in" />
    <Dialog.Content preventScroll={false} class="fixed bg-bg-2 text-ink-0 bottom-0 left-0 right-0 w-full p-[14px_14px_24px] z-20 border-t border-t-bd-1 text-[12px] leading-[1.4] font-sans anim-sheet-in">
      <div class="flex items-center justify-between mb-2.5">
        <span class="uppercase tracking-[0.5px] text-[10px] leading-none font-mono text-ink-3">note</span>
        <Dialog.Close class="bg-transparent border-none text-ink-2 cursor-pointer flex"><Icon name="x" size={14} /></Dialog.Close>
      </div>
      <textarea bind:value={draft} placeholder="Add a note about this post…" class="w-full box-border min-h-20 rounded p-2.5 resize-y bg-bg-0 border border-bd-1 text-[12px] leading-normal font-sans text-ink-0"></textarea>
      <div class="flex gap-2 mt-3">
        <Dialog.Close class="flex-1 bg-transparent text-ink-1 border border-bd-2 cursor-pointer py-2.5 rounded tracking-[0.3px] text-[11px] leading-none font-mono">cancel</Dialog.Close>
        <button onclick={submit} class="flex-1 bg-amber text-bg-0 border-none cursor-pointer py-2.5 rounded tracking-[0.3px] text-[11px] leading-none font-mono">save with note</button>
      </div>
    </Dialog.Content>
  </Dialog.Portal>
</Dialog.Root>
