<script lang="ts">
  import { T } from '$lib/tokens';
  let { k, dim = false }: { k: string; dim?: boolean } = $props();
</script>

<span
  class="inline-block font-semibold text-[10px] leading-none font-mono border border-bd-2 rounded p-[2px_4px] min-w-[12px] text-center"
  style="color:{dim ? T.ink2 : T.ink0};background:{dim ? 'transparent' : T.bg3};"
>{k}</span>
