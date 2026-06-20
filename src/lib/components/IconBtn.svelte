<script lang="ts">
  import Icon from './Icon.svelte';

  let {
    name,
    ariaLabel,
    onclick,
    active = false,
    size = 44,
  }: {
    name: string;
    ariaLabel: string;
    onclick?: () => void;
    active?: boolean;
    size?: number;
  } = $props();

  const minSide = Math.max(size, 44);
  const iconSize = Math.round(minSide * 0.5);
</script>

<button
  {onclick}
  aria-label={ariaLabel}
  class="inline-flex items-center justify-center rounded cursor-pointer border-none p-0"
  style="
    width:{minSide}px;height:{minSide}px;
    background:{active ? 'var(--color-bg-3)' : 'transparent'};
  "
  onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.background = 'var(--color-bg-2)'; }}
  onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.background = active ? 'var(--color-bg-3)' : 'transparent'; }}
>
  <Icon {name} size={iconSize} color={active ? 'var(--color-cyan)' : 'var(--color-ink-1)'} />
</button>
