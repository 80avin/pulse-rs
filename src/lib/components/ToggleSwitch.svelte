<script lang="ts">
  import { T } from '$lib/tokens';
  let { on, change, ariaLabel }: { on: boolean; change: () => void; ariaLabel?: string } = $props();
</script>

<button
  onclick={change}
  role="switch"
  aria-checked={on}
  aria-label={ariaLabel ?? (on ? 'on' : 'off')}
  class="w-11 h-6 rounded-xl border-none cursor-pointer relative shrink-0 transition-[background_0.15s]"
  style="background:{on ? T.cyan : T.bg3};"
>
  <span class="absolute top-[3px] w-4.5 h-[18px] rounded-full transition-[left_0.15s]" style="left:{on ? '23px' : '3px'};background:{on ? T.bg0 : T.ink3};"></span>
</button>
