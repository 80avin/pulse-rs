<script lang="ts">
  import type { Snippet } from 'svelte';

  let {
    onclick,
    disabled = false,
    type = 'button',
    title,
    ariaLabel,
    class: cls = '',
    style,
    ariaExpanded,
    ariaCurrent,
    ontouchstart,
    ontouchend,
    ontouchcancel,
    children,
  }: {
    onclick?: (e: MouseEvent) => void;
    disabled?: boolean;
    type?: 'button' | 'submit' | 'reset';
    title?: string;
    ariaLabel?: string;
    class?: string;
    style?: string;
    ariaExpanded?: boolean;
    ariaCurrent?: 'page' | 'step' | 'location' | 'date' | 'time' | boolean | null;
    ontouchstart?: (e: TouchEvent) => void;
    ontouchend?: (e: TouchEvent) => void;
    ontouchcancel?: (e: TouchEvent) => void;
    children: Snippet;
  } = $props();
</script>

<button
  {type}
  {onclick}
  {disabled}
  {title}
  aria-label={ariaLabel}
  {style}
  aria-expanded={ariaExpanded}
  aria-current={ariaCurrent}
  {ontouchstart}
  {ontouchend}
  {ontouchcancel}
  class="bg-transparent border-none cursor-pointer font-mono disabled:cursor-default disabled:opacity-50 {cls}"
>
  {@render children()}
</button>
