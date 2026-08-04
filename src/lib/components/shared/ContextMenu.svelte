<script lang="ts">
  import type { Snippet } from 'svelte';
  import { Portal } from 'bits-ui';

  let {
    open = false,
    x = 0,
    y = 0,
    mode,
    onClose,
    class: shellClass = '',
    children,
  }: {
    open?: boolean;
    x?: number;
    y?: number;
    mode: 'popup' | 'sheet';
    onClose: () => void;
    class?: string;
    children: Snippet;
  } = $props();

  let menuEl = $state<HTMLDivElement | null>(null);
  let previouslyFocused: HTMLElement | null = null;

  const pos = $derived.by(() => {
    if (typeof window === 'undefined' || !menuEl) return { x, y };
    const margin = 4;
    const w = menuEl.offsetWidth;
    const h = menuEl.offsetHeight;
    return {
      x: Math.max(margin, Math.min(x, window.innerWidth - w - margin)),
      y: Math.max(margin, Math.min(y, window.innerHeight - h - margin)),
    };
  });

  $effect(() => {
    if (!open || typeof window === 'undefined') return;
    previouslyFocused = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    const raf = requestAnimationFrame(() => menuEl?.focus());
    function onKey(e: KeyboardEvent) {
      if (e.key === 'Escape') {
        e.preventDefault();
        e.stopImmediatePropagation();
        onClose();
      }
    }
    window.addEventListener('keydown', onKey, true);
    return () => {
      cancelAnimationFrame(raf);
      window.removeEventListener('keydown', onKey, true);
      if (previouslyFocused && document.contains(previouslyFocused)) {
        previouslyFocused.focus();
      }
      previouslyFocused = null;
    };
  });
</script>

{#if open}
  <Portal>
    {#if mode === 'popup'}
      <div class="fixed inset-0 z-[100] bg-black/40 anim-sheet-overlay-in" role="presentation" aria-hidden="true" onclick={onClose}></div>
      <div
        bind:this={menuEl}
        tabindex="-1"
        class="fixed z-[101] bg-bg-1 border border-bd-1 rounded overflow-hidden shadow-[0_8px_32px_rgba(0,0,0,0.6)] anim-pop-in outline-none {shellClass}"
        style="left:{pos.x}px;top:{pos.y}px;"
      >
        {@render children()}
      </div>
    {:else}
      <div class="fixed inset-0 z-[100] bg-black/40 anim-sheet-overlay-in" role="presentation" aria-hidden="true" onclick={onClose}></div>
      <div
        bind:this={menuEl}
        tabindex="-1"
        class="fixed inset-x-0 bottom-0 z-[101] bg-bg-1 border-t border-bd-1 rounded-t-xl anim-sheet-in outline-none"
        style="padding:8px;padding-bottom:max(8px,env(safe-area-inset-bottom));"
      >
        {@render children()}
      </div>
    {/if}
  </Portal>
{/if}

<style>
  :global(.menu-row) {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 10px;
    background: transparent;
    border: none;
    border-bottom: 1px solid var(--color-bd-0);
    cursor: pointer;
    text-align: left;
    color: var(--color-ink-0);
    font-family: var(--font-mono);
    padding: 11px 14px;
  }
  :global(.menu-row:hover) {
    background: var(--color-bg-2);
  }
</style>
