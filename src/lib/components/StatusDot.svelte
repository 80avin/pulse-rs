<script lang="ts">
  import { T } from '$lib/tokens';
  let { status, size = 6, label }: { status: string; size?: number; label?: string } = $props();
  const c = $derived(status === 'ok' ? T.green : status === 'stale' ? T.amber : status === 'error' ? T.red : T.ink3);
  const labelText = $derived(label ?? (status === 'ok' ? 'healthy' : status === 'stale' ? 'stale' : status === 'error' ? 'error' : 'unknown'));
</script>

<span
  role="img"
  aria-label={labelText}
  title={labelText}
  style="
    display:inline-block;
    width:{size}px;height:{size}px;border-radius:{size}px;
    background:{c};box-shadow:0 0 4px {c}66;
    flex-shrink:0;
  "
></span>
