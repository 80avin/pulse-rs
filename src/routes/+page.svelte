<script lang="ts">
  import { onMount } from 'svelte';
  import DesktopShell from '$lib/screens/DesktopShell.svelte';
  import MobileShell from '$lib/screens/MobileShell.svelte';

  // null until onMount determines the viewport — prevents Desktop from
  // mounting and tearing down immediately on Android.
  let isMobile = $state<boolean | null>(null);

  onMount(() => {
    const check = () => { isMobile = window.innerWidth < 768; };
    check();
    window.addEventListener('resize', check);
    return () => window.removeEventListener('resize', check);
  });
</script>

{#if isMobile === true}
  <MobileShell />
{:else if isMobile === false}
  <DesktopShell />
{/if}
