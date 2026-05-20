<script lang="ts">
  import { T, TAG_COLORS } from '$lib/tokens';
  let { tag, dim = false, size = 11, onclick }: {
    tag: string; dim?: boolean; size?: number; onclick?: () => void;
  } = $props();
  const c = $derived(TAG_COLORS[tag] ?? TAG_COLORS['low-effort']);
</script>

<span
  role={onclick ? 'button' : undefined}
  tabindex={onclick ? 0 : undefined}
  {onclick}
  onkeydown={onclick ? (e: KeyboardEvent) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); onclick!(); } } : undefined}
  style="
    display:inline-flex;align-items:center;
    font:{size}px/1 {T.mono};
    color:{dim ? T.ink2 : c.fg};
    background:{dim ? 'transparent' : c.bg};
    border:1px solid {dim ? T.bd1 : c.bd};
    padding:2px 5px;border-radius:2px;
    letter-spacing:0.2px;white-space:nowrap;
    cursor:{onclick ? 'pointer' : 'default'};
  "
>{tag}</span>
