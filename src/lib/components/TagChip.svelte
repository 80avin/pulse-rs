<script lang="ts">
  import { T, TAG_COLORS } from '$lib/tokens';
  let { tag, dim = false, size = 10, onclick }: {
    tag: string; dim?: boolean; size?: number; onclick?: (e: Event) => void;
  } = $props();
  const c = $derived(TAG_COLORS[tag] ?? TAG_COLORS['low-effort']);
</script>

<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
<span
  role={onclick ? 'button' : undefined}
  tabindex={onclick ? 0 : undefined}
  {onclick}
  onkeydown={onclick ? (e: KeyboardEvent) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); e.stopPropagation(); onclick!(e); } } : undefined}
  class="inline-flex items-center p-[2px_5px] rounded-sm tracking-[0.2px] whitespace-nowrap leading-none font-mono"
  style="font-size:{size}px;color:{dim ? T.ink2 : c.fg};background:{dim ? 'transparent' : c.bg};border:1px solid {dim ? T.bd1 : c.bd};cursor:{onclick ? 'pointer' : 'default'};"
>{tag}</span>
