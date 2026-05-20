<script lang="ts">
  import { T } from '$lib/tokens';
  import Icon from './Icon.svelte';
  let { name, onclick, active = false, size = 26, color }: {
    name: string; onclick?: () => void; active?: boolean; size?: number; color?: string;
  } = $props();
  const iconColor = $derived(color ?? (active ? T.cyan : T.ink1));
</script>

<button
  {onclick}
  style="
    width:{size}px;height:{size}px;
    display:inline-flex;align-items:center;justify-content:center;
    background:{active ? T.bg3 : 'transparent'};
    color:{iconColor};
    border:none;cursor:pointer;padding:0;border-radius:3px;
  "
>
  <Icon {name} size={Math.round(size * 0.55)} color={iconColor} />
</button>
