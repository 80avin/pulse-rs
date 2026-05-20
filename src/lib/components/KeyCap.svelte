<script lang="ts">
  import { T } from '$lib/tokens';
  let { k, dim = false }: { k: string; dim?: boolean } = $props();
</script>

<span style="
  display:inline-block;
  font:600 10px/1 {T.mono};
  color:{dim ? T.ink2 : T.ink0};
  background:{dim ? 'transparent' : T.bg3};
  border:1px solid {T.bd2};
  border-radius:3px;
  padding:2px 4px;
  min-width:12px;
  text-align:center;
">{k}</span>
