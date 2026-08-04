<script lang="ts">
  import type { Snippet } from 'svelte';
  import Icon from '$lib/components/Icon.svelte';

  let {
    name,
    size = 13,
    color,
    onclick,
    disabled = false,
    title,
    ariaLabel,
    class: cls = '',
    style,
    children,
  }: {
    name: string;
    size?: number;
    color?: string;
    onclick?: (e: MouseEvent) => void;
    disabled?: boolean;
    title?: string;
    ariaLabel?: string;
    class?: string;
    style?: string;
    children?: Snippet;
  } = $props();
</script>

<button
  type="button"
  {onclick}
  {disabled}
  {title}
  aria-label={ariaLabel}
  {style}
  class="bg-transparent border-none cursor-pointer rounded min-h-11 min-w-11 inline-flex items-center justify-center disabled:cursor-default disabled:opacity-50 {cls}"
>
  {#if children}
    {@render children()}
  {:else}
    <Icon {name} {size} {color} />
  {/if}
</button>
