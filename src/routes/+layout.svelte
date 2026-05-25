<script lang="ts">
  import { onMount } from 'svelte';
  import '../app.css';
  import { setupTaggingListener, initStore } from '$lib/store.svelte';
  import { logger } from '$lib/logger';
  import { setupShareListener, shareSheet } from '$lib/share.svelte';
  import { initSettings } from '$lib/settings.svelte';
  import ShareSheet from '$lib/components/ShareSheet.svelte';

  let { children } = $props();

  // onMount fires after the WebView has painted, guaranteeing the Tauri IPC
  // bridge is ready — avoids the silent-failure race on Android cold start.
  onMount(() => {
    Promise.all([initStore(), initSettings()]).catch(e => logger.error('app init failed', e));
  });

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
