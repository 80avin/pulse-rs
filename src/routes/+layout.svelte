<script lang="ts">
  import '../app.css';
  import { setupTaggingListener } from '$lib/store.svelte';
  import { setupShareListener, shareSheet } from '$lib/share.svelte';
  import ShareSheet from '$lib/components/ShareSheet.svelte';

  let { children } = $props();

  $effect(() => {
    let unlisten: (() => void) | undefined;
    setupTaggingListener().then(u => { unlisten = u; });
    return () => { unlisten?.(); };
  });

  $effect(() => {
    let unlisten: (() => void) | undefined;
    setupShareListener().then(u => { unlisten = u; });
    return () => { unlisten?.(); };
  });
</script>

{@render children()}
{#if shareSheet.candidate !== null}
  <ShareSheet />
{/if}
