<script lang="ts">
  import { T } from '$lib/tokens';
  let { status, size = 6 }: { status: string; size?: number } = $props();
  const c = $derived(status === 'ok' ? T.green : status === 'stale' ? T.amber : status === 'error' ? T.red : T.ink3);
</script>

<span style="
  display:inline-block;
  width:{size}px;height:{size}px;border-radius:{size}px;
  background:{c};box-shadow:0 0 4px {c}66;
  flex-shrink:0;
"></span>
